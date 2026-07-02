use anyhow::{Context as AnyhowContext, Result};
use k8s_openapi::api::core::v1::Namespace;
use kube::api::{ApiResource, ListParams};
use kube::discovery::{verbs, Discovery, Scope};
use kube::{Api, ResourceExt};

use crate::{KubeSession, ResourceKind, ResourceScope};

impl KubeSession {
    pub async fn discover_resources(&self) -> Result<Vec<ResourceKind>> {
        let discovery = Discovery::new(self.client.clone())
            .run()
            .await
            .with_context(|| {
                format!(
                    "Could not discover Kubernetes resource types using context {}.",
                    self.context
                )
            })?;
        let mut resources = Vec::new();

        for group in discovery.groups_alphabetical() {
            for (resource, capabilities) in group.recommended_resources() {
                if !capabilities.supports_operation(verbs::LIST)
                    || resource.plural.contains('/')
                    || resource.kind.ends_with("List")
                {
                    continue;
                }

                resources.push(ResourceKind {
                    group: resource.group,
                    version: resource.version,
                    api_version: resource.api_version,
                    kind: resource.kind,
                    plural: resource.plural,
                    scope: match capabilities.scope {
                        Scope::Cluster => ResourceScope::Cluster,
                        Scope::Namespaced => ResourceScope::Namespaced,
                    },
                });
            }
        }

        resources.sort_by(|left, right| {
            resource_group_order(left)
                .cmp(&resource_group_order(right))
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.api_version.cmp(&right.api_version))
        });

        Ok(resources)
    }

    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        let mut names = namespaces
            .list(&ListParams::default())
            .await
            .with_context(|| {
                format!(
                    "Could not list Namespaces at cluster scope using context {}.",
                    self.context
                )
            })?
            .items
            .into_iter()
            .map(|namespace| namespace.name_any())
            .collect::<Vec<_>>();

        names.sort();
        names.dedup();

        if names.is_empty() {
            names.push(String::from("default"));
        }

        Ok(names)
    }
}

pub(crate) fn api_resource(resource: &ResourceKind) -> ApiResource {
    ApiResource {
        group: resource.group.clone(),
        version: resource.version.clone(),
        api_version: resource.api_version.clone(),
        kind: resource.kind.clone(),
        plural: resource.plural.clone(),
    }
}

pub(crate) fn namespace_scope(namespace: Option<&str>) -> String {
    match namespace {
        Some(namespace) if !namespace.is_empty() && namespace != "all" && namespace != "-" => {
            format!("in namespace {namespace}")
        }
        _ => String::from("across all namespaces"),
    }
}

pub(crate) fn resource_scope(resource: &ResourceKind, namespace: Option<&str>) -> String {
    if resource.is_namespaced() {
        namespace_scope(namespace)
    } else {
        String::from("at cluster scope")
    }
}

fn resource_group_order(resource: &ResourceKind) -> (u8, &str) {
    let rank = match resource.group.as_str() {
        "" | "apps" | "batch" => 0,
        "networking.k8s.io" | "discovery.k8s.io" => 1,
        "storage.k8s.io" => 2,
        "rbac.authorization.k8s.io" => 3,
        "apiextensions.k8s.io" => 4,
        _ => 5,
    };

    (rank, resource.group.as_str())
}
