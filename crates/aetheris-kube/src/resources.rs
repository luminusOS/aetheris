use anyhow::{Context as AnyhowContext, Result};
use k8s_openapi::api::core::v1::Namespace;
use kube::api::{ApiResource, DynamicObject, ListParams};
use kube::discovery::{Discovery, Scope, verbs};
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
        let native_error = match self.list_native_namespaces().await {
            Ok(names) => return Ok(normalized_namespace_names(names)),
            Err(error) => error,
        };

        // Scoped tokens on OpenShift and Rancher regularly lack cluster-wide
        // `list namespaces`, but both platforms expose an endpoint that
        // returns only what the caller can access, filtered server-side.
        if let Ok(names) = self.list_openshift_projects().await {
            return Ok(normalized_namespace_names(names));
        }
        if self.server.contains("/k8s/clusters/")
            && let Ok(names) = self.list_rancher_namespaces().await
        {
            return Ok(normalized_namespace_names(names));
        }

        Err(native_error)
    }

    async fn list_native_namespaces(&self) -> Result<Vec<String>> {
        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        let names = namespaces
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
            .collect();
        Ok(names)
    }

    /// OpenShift's Project API returns only the projects (namespaces) the
    /// current user can see — the filtering happens server-side, so no
    /// cluster-wide RBAC is needed. Same call the web console and
    /// `oc get projects` make. Plain 404s on non-OpenShift clusters.
    async fn list_openshift_projects(&self) -> Result<Vec<String>> {
        let resource = ApiResource {
            group: String::from("project.openshift.io"),
            version: String::from("v1"),
            api_version: String::from("project.openshift.io/v1"),
            kind: String::from("Project"),
            plural: String::from("projects"),
        };
        let projects: Api<DynamicObject> = Api::all_with(self.client.clone(), &resource);
        let names = projects
            .list(&ListParams::default())
            .await?
            .items
            .into_iter()
            .map(|project| project.name_any())
            .collect();
        Ok(names)
    }

    /// Rancher's Steve API (the one its own dashboard uses) also filters
    /// namespaces by the caller's project memberships. It is served next to
    /// the Kubernetes proxy this session already points at
    /// (`…/k8s/clusters/<id>/v1/namespaces`) and accepts the same bearer
    /// token, so the request goes through the existing client.
    async fn list_rancher_namespaces(&self) -> Result<Vec<String>> {
        let request = http::Request::get("/v1/namespaces").body(Vec::new())?;
        let response: serde_json::Value = self.client.request(request).await?;
        let names = steve_namespace_names(&response);
        if names.is_empty() {
            anyhow::bail!("Steve namespace listing returned no entries");
        }
        Ok(names)
    }
}

fn steve_namespace_names(response: &serde_json::Value) -> Vec<String> {
    let Some(items) = response.get("data").and_then(|data| data.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            item.get("id")
                .or_else(|| item.pointer("/metadata/name"))
                .and_then(|name| name.as_str())
        })
        .map(str::to_owned)
        .collect()
}

fn normalized_namespace_names(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names.dedup();
    if names.is_empty() {
        names.push(String::from("default"));
    }
    names
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

#[cfg(test)]
mod tests {
    use super::{normalized_namespace_names, steve_namespace_names};

    #[test]
    fn steve_namespace_names_reads_collection_ids() {
        let response = serde_json::json!({
            "type": "collection",
            "data": [
                { "id": "cattle-system", "metadata": { "name": "cattle-system" } },
                { "id": "rnds-dev" },
                { "metadata": { "name": "no-id-namespace" } },
            ],
        });

        assert_eq!(
            steve_namespace_names(&response),
            vec![
                String::from("cattle-system"),
                String::from("rnds-dev"),
                String::from("no-id-namespace"),
            ]
        );
    }

    #[test]
    fn steve_namespace_names_handles_non_collection_payloads() {
        assert!(steve_namespace_names(&serde_json::json!({"type": "error"})).is_empty());
        assert!(steve_namespace_names(&serde_json::json!(null)).is_empty());
    }

    #[test]
    fn normalized_namespace_names_sorts_dedups_and_defaults() {
        assert_eq!(
            normalized_namespace_names(vec![
                String::from("b"),
                String::from("a"),
                String::from("b"),
            ]),
            vec![String::from("a"), String::from("b")]
        );
        assert_eq!(
            normalized_namespace_names(Vec::new()),
            vec![String::from("default")]
        );
    }
}
