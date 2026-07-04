use std::collections::BTreeMap;

use anyhow::Result;
use kube::api::{ApiResource, DynamicObject, ListParams};
use kube::{Api, ResourceExt};
use serde_json::Value;

use crate::{ContainerUsage, KubeSession, ResourceKind, ResourceUsage};

impl KubeSession {
    pub(crate) async fn resource_metrics(
        &self,
        resource: &ResourceKind,
        namespace: Option<&str>,
    ) -> Result<BTreeMap<(String, String), ResourceUsage>> {
        let namespace = namespace.filter(|namespace| !namespace.is_empty() && *namespace != "all");
        let metrics_resource = match (resource.group.as_str(), resource.kind.as_str()) {
            ("", "Pod") => metrics_api_resource("PodMetrics", "pods"),
            ("", "Node") => metrics_api_resource("NodeMetrics", "nodes"),
            _ => return Ok(BTreeMap::new()),
        };
        let metrics: Api<DynamicObject> = match (resource.kind.as_str(), namespace) {
            ("Pod", Some(namespace)) if !namespace.is_empty() && namespace != "all" => {
                Api::namespaced_with(self.client.clone(), namespace, &metrics_resource)
            }
            _ => Api::all_with(self.client.clone(), &metrics_resource),
        };

        let items = metrics.list(&ListParams::default()).await?.items;
        Ok(items
            .into_iter()
            .filter_map(|object| {
                let name = object.metadata.name.clone()?;
                let namespace = object.namespace().unwrap_or_else(|| String::from("-"));
                let usage = object_usage(&object)?;
                Some(((namespace, name), usage))
            })
            .collect())
    }

    pub(crate) async fn pod_container_metrics(
        &self,
        namespace: &str,
        pod: &str,
    ) -> Result<Vec<ContainerUsage>> {
        if namespace == "-" {
            return Ok(Vec::new());
        }
        let metrics_resource = metrics_api_resource("PodMetrics", "pods");
        let metrics: Api<DynamicObject> =
            Api::namespaced_with(self.client.clone(), namespace, &metrics_resource);
        let object = metrics.get(pod).await?;
        let mut containers = object
            .data
            .get("containers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|container| {
                let name = container.get("name")?.as_str()?.to_owned();
                let usage = usage_from_value(container.get("usage")?)?;
                Some(ContainerUsage {
                    name,
                    cpu: usage.cpu,
                    memory: usage.memory,
                })
            })
            .collect::<Vec<_>>();
        containers.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(containers)
    }
}

/// NodeMetrics carries `usage` at the top level; PodMetrics only carries
/// per-container usage inside `containers[]`, so a Pod's total has to be
/// summed from them (this is also what `kubectl top pods` does).
fn object_usage(object: &DynamicObject) -> Option<ResourceUsage> {
    if let Some(usage) = object.data.get("usage") {
        return usage_from_value(usage);
    }
    pod_usage_from_containers(object.data.get("containers")?)
}

fn pod_usage_from_containers(containers: &Value) -> Option<ResourceUsage> {
    let mut cpu_cores = 0.0_f64;
    let mut memory_bytes = 0.0_f64;
    let mut found = false;
    for container in containers.as_array()?.iter() {
        let Some(usage) = container.get("usage") else {
            continue;
        };
        if let Some(value) = usage
            .get("cpu")
            .and_then(Value::as_str)
            .and_then(crate::objects::quantity_as_f64)
        {
            cpu_cores += value;
            found = true;
        }
        if let Some(value) = usage
            .get("memory")
            .and_then(Value::as_str)
            .and_then(crate::objects::quantity_as_f64)
        {
            memory_bytes += value;
            found = true;
        }
    }

    found.then(|| ResourceUsage {
        cpu: format_cpu_quantity(cpu_cores),
        memory: format_memory_quantity(memory_bytes),
        cpu_ratio: None,
        memory_ratio: None,
    })
}

/// Millicores, like `kubectl top`: "4m". Rounds up so tiny-but-nonzero
/// usage never shows as "0m".
fn format_cpu_quantity(cores: f64) -> String {
    format!("{}m", (cores * 1000.0).ceil().max(0.0) as u64)
}

/// Mebibytes, like `kubectl top`: "83Mi".
fn format_memory_quantity(bytes: f64) -> String {
    format!("{}Mi", (bytes / (1024.0 * 1024.0)).ceil().max(0.0) as u64)
}

fn metrics_api_resource(kind: &str, plural: &str) -> ApiResource {
    ApiResource {
        group: String::from("metrics.k8s.io"),
        version: String::from("v1beta1"),
        api_version: String::from("metrics.k8s.io/v1beta1"),
        kind: kind.to_owned(),
        plural: plural.to_owned(),
    }
}

fn usage_from_value(value: &Value) -> Option<ResourceUsage> {
    Some(ResourceUsage {
        cpu: value
            .get("cpu")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or("-")
            .to_owned(),
        memory: value
            .get("memory")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or("-")
            .to_owned(),
        cpu_ratio: None,
        memory_ratio: None,
    })
}

#[cfg(test)]
mod tests {
    use super::pod_usage_from_containers;
    use serde_json::json;

    #[test]
    fn pod_usage_sums_container_usage() {
        let containers = json!([
            {"name": "app", "usage": {"cpu": "250m", "memory": "512Mi"}},
            {"name": "sidecar", "usage": {"cpu": "750u", "memory": "512Ki"}},
        ]);

        let usage = pod_usage_from_containers(&containers).unwrap();

        assert_eq!(usage.cpu, "251m");
        assert_eq!(usage.memory, "513Mi");
    }

    #[test]
    fn pod_usage_is_none_without_container_samples() {
        assert!(pod_usage_from_containers(&json!([])).is_none());
        assert!(pod_usage_from_containers(&json!([{"name": "app"}])).is_none());
    }
}
