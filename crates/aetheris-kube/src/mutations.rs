use anyhow::{bail, Context as AnyhowContext, Result};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{Node, Pod};
use kube::api::{DeleteParams, DynamicObject, ListParams, Patch, PatchParams};
use kube::{Api, ResourceExt};
use serde_json::Value;

use crate::{api_resource, resource_scope, KubeSession, ObjectDetail, ResourceKind};

impl KubeSession {
    pub async fn apply_object_yaml(
        &self,
        resource: &ResourceKind,
        namespace: Option<&str>,
        name: &str,
        yaml: &str,
    ) -> Result<ObjectDetail> {
        let api_resource = api_resource(resource);
        let objects: Api<DynamicObject> = match (resource.is_namespaced(), namespace) {
            (true, Some(namespace)) if !namespace.is_empty() && namespace != "-" => {
                Api::namespaced_with(self.client.clone(), namespace, &api_resource)
            }
            _ => Api::all_with(self.client.clone(), &api_resource),
        };
        let mut value: Value = serde_yaml::from_str(yaml).context("failed to parse YAML")?;
        sanitize_apply_value(&mut value);
        let params = PatchParams::apply("aetheris").force();
        objects
            .patch(name, &params, &Patch::Apply(&value))
            .await
            .with_context(|| {
                format!(
                    "Could not apply YAML to {} {name} {} using context {}.",
                    resource.kind,
                    resource_scope(resource, namespace),
                    self.context
                )
            })?;

        self.object_detail(resource, namespace, name).await
    }

    pub async fn create_object_yaml(
        &self,
        resource: &ResourceKind,
        default_namespace: Option<&str>,
        yaml: &str,
    ) -> Result<ObjectDetail> {
        let mut value: Value = serde_yaml::from_str(yaml).context("failed to parse YAML")?;
        let name = value
            .get("metadata")
            .and_then(|metadata| metadata.get("name"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .context("YAML metadata.name is required")?;
        let namespace = value
            .get("metadata")
            .and_then(|metadata| metadata.get("namespace"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| default_namespace.map(ToOwned::to_owned))
            .filter(|namespace| !namespace.is_empty() && namespace != "all" && namespace != "-");

        if resource.is_namespaced() {
            let namespace = namespace
                .clone()
                .context("namespace is required for this resource")?;
            ensure_yaml_namespace(&mut value, &namespace);
        }

        let yaml = serde_yaml::to_string(&value).context("failed to serialize YAML")?;
        self.apply_object_yaml(resource, namespace.as_deref(), &name, &yaml)
            .await
    }

    pub async fn delete_object(
        &self,
        resource: &ResourceKind,
        namespace: Option<&str>,
        name: &str,
    ) -> Result<()> {
        let api_resource = api_resource(resource);
        let objects: Api<DynamicObject> = match (resource.is_namespaced(), namespace) {
            (true, Some(namespace)) if !namespace.is_empty() && namespace != "-" => {
                Api::namespaced_with(self.client.clone(), namespace, &api_resource)
            }
            _ => Api::all_with(self.client.clone(), &api_resource),
        };
        objects
            .delete(name, &DeleteParams::background())
            .await
            .with_context(|| {
                format!(
                    "Could not delete {} {name} {} using context {}.",
                    resource.kind,
                    resource_scope(resource, namespace),
                    self.context
                )
            })?;

        Ok(())
    }

    pub async fn scale_deployment(&self, namespace: &str, name: &str, replicas: i32) -> Result<()> {
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        let params = PatchParams::apply("aetheris").force();
        let patch = serde_json::json!({
            "apiVersion": "autoscaling/v1",
            "kind": "Scale",
            "metadata": { "name": name, "namespace": namespace },
            "spec": { "replicas": replicas }
        });
        deployments
            .patch_scale(name, &params, &Patch::Apply(&patch))
            .await
            .with_context(|| {
                format!(
                    "Could not scale Deployment {name} in namespace {namespace} using context {}.",
                    self.context
                )
            })?;

        Ok(())
    }

    pub async fn set_node_unschedulable(&self, name: &str, unschedulable: bool) -> Result<()> {
        let nodes: Api<Node> = Api::all(self.client.clone());
        let params = PatchParams::default();
        let patch = serde_json::json!({
            "spec": { "unschedulable": unschedulable }
        });
        nodes
            .patch(name, &params, &Patch::Merge(&patch))
            .await
            .with_context(|| {
                format!(
                    "Could not update scheduling state for Node {name} using context {}.",
                    self.context
                )
            })?;

        Ok(())
    }

    pub async fn drain_node(&self, name: &str) -> Result<usize> {
        let pods: Api<Pod> = Api::all(self.client.clone());
        let params = ListParams::default().fields(&format!("spec.nodeName={name}"));
        let items = pods
            .list(&params)
            .await
            .with_context(|| {
                format!(
                    "Could not list Pods scheduled on Node {name} using context {}.",
                    self.context
                )
            })?
            .items;
        let unmanaged = items
            .iter()
            .filter(|pod| {
                !is_terminal_pod(pod)
                    && !is_daemonset_pod(pod)
                    && !is_mirror_pod(pod)
                    && is_unmanaged_pod(pod)
            })
            .map(|pod| pod.name_any())
            .collect::<Vec<_>>();
        if !unmanaged.is_empty() {
            bail!(
                "drain blocked because these Pods do not have a controller: {}",
                unmanaged.join(", ")
            );
        }

        let mut deleted = 0;
        for pod in items {
            if is_terminal_pod(&pod) || is_daemonset_pod(&pod) || is_mirror_pod(&pod) {
                continue;
            }
            let Some(namespace) = pod.namespace() else {
                continue;
            };
            let namespaced_pods: Api<Pod> = Api::namespaced(self.client.clone(), &namespace);
            namespaced_pods
                .delete(&pod.name_any(), &DeleteParams::default())
                .await
                .with_context(|| {
                    format!(
                        "Could not evict/delete Pod {} in namespace {namespace} while draining Node {name}.",
                        pod.name_any()
                    )
                })?;
            deleted += 1;
        }

        Ok(deleted)
    }
}

fn ensure_yaml_namespace(value: &mut Value, namespace: &str) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    let metadata = object
        .entry("metadata")
        .or_insert_with(|| Value::Object(Default::default()));
    if let Some(metadata) = metadata.as_object_mut() {
        metadata.insert(
            String::from("namespace"),
            Value::String(namespace.to_owned()),
        );
    }
}

fn sanitize_apply_value(value: &mut Value) {
    if let Some(object) = value.as_object_mut() {
        object.remove("status");
        if let Some(metadata) = object.get_mut("metadata").and_then(Value::as_object_mut) {
            for field in [
                "creationTimestamp",
                "deletionGracePeriodSeconds",
                "deletionTimestamp",
                "generation",
                "managedFields",
                "resourceVersion",
                "selfLink",
                "uid",
            ] {
                metadata.remove(field);
            }
        }
    }
}

fn is_terminal_pod(pod: &Pod) -> bool {
    pod.status
        .as_ref()
        .and_then(|status| status.phase.as_deref())
        .is_some_and(|phase| matches!(phase, "Succeeded" | "Failed"))
}

fn is_daemonset_pod(pod: &Pod) -> bool {
    pod.metadata
        .owner_references
        .as_deref()
        .unwrap_or_default()
        .iter()
        .any(|owner| owner.kind == "DaemonSet")
}

fn is_mirror_pod(pod: &Pod) -> bool {
    pod.metadata
        .annotations
        .as_ref()
        .is_some_and(|annotations| annotations.contains_key("kubernetes.io/config.mirror"))
}

fn is_unmanaged_pod(pod: &Pod) -> bool {
    pod.metadata
        .owner_references
        .as_deref()
        .unwrap_or_default()
        .is_empty()
}
