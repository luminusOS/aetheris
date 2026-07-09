use std::collections::BTreeMap;

use anyhow::{Context as AnyhowContext, Result};
use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{DynamicObject, ListParams};
use kube::runtime::watcher::{Event as WatcherEvent, watcher};
use kube::{Api, ResourceExt};
use serde_json::Value;

use crate::status::{age_label, status_label};
use crate::{
    KubeSession, ObjectCondition, ObjectDetail, ObjectSummary, ObjectWatchEvent, PodSummary,
    ResourceKind, ResourceRatio, ResourceScope, ResourceUsage, api_resource, namespace_scope,
    resource_scope,
};

impl KubeSession {
    pub async fn list_pods(&self, namespace: Option<&str>) -> Result<Vec<PodSummary>> {
        let pods: Api<Pod> = match namespace {
            Some(namespace) if !namespace.is_empty() && namespace != "all" => {
                Api::namespaced(self.client.clone(), namespace)
            }
            _ => Api::all(self.client.clone()),
        };

        let mut summaries = pods
            .list(&ListParams::default())
            .await
            .with_context(|| {
                format!(
                    "Could not list Pods {} using context {}.",
                    namespace_scope(namespace),
                    self.context
                )
            })?
            .items
            .into_iter()
            .map(pod_summary)
            .collect::<Vec<_>>();

        summaries.sort_by(|left, right| {
            left.namespace
                .cmp(&right.namespace)
                .then_with(|| left.name.cmp(&right.name))
        });

        Ok(summaries)
    }

    pub async fn list_objects(
        &self,
        resource: &ResourceKind,
        namespace: Option<&str>,
    ) -> Result<Vec<ObjectSummary>> {
        let api_resource = api_resource(resource);
        let objects: Api<DynamicObject> = match (resource.is_namespaced(), namespace) {
            (true, Some(namespace)) if !namespace.is_empty() && namespace != "all" => {
                Api::namespaced_with(self.client.clone(), namespace, &api_resource)
            }
            _ => Api::all_with(self.client.clone(), &api_resource),
        };

        let metrics = self
            .resource_metrics(resource, namespace)
            .await
            .unwrap_or_default();

        let mut summaries = objects
            .list(&ListParams::default())
            .await
            .with_context(|| {
                format!(
                    "Could not list {} {} using context {}.",
                    resource.kind,
                    resource_scope(resource, namespace),
                    self.context
                )
            })?
            .items
            .into_iter()
            .map(|object| object_summary(object, resource, &metrics))
            .collect::<Vec<_>>();

        sort_object_summaries(&mut summaries);

        Ok(summaries)
    }

    pub async fn watch_objects<F>(
        &self,
        resource: ResourceKind,
        namespace: Option<String>,
        mut on_event: F,
    ) -> Result<()>
    where
        F: FnMut(ObjectWatchEvent) + Send + 'static,
    {
        let namespace = namespace.filter(|namespace| !namespace.is_empty() && namespace != "all");
        let api_resource = api_resource(&resource);
        let objects: Api<DynamicObject> = match (resource.is_namespaced(), namespace.as_deref()) {
            (true, Some(namespace)) => {
                Api::namespaced_with(self.client.clone(), namespace, &api_resource)
            }
            _ => Api::all_with(self.client.clone(), &api_resource),
        };
        let mut metrics = self
            .resource_metrics(&resource, namespace.as_deref())
            .await
            .unwrap_or_default();
        let mut stream = Box::pin(watcher(objects, kube::runtime::watcher::Config::default()));
        let mut init_buffer = Vec::new();
        let mut known_objects: BTreeMap<(String, String), DynamicObject> = BTreeMap::new();
        let mut metrics_refresh = tokio::time::interval(std::time::Duration::from_secs(15));
        let refresh_metrics = supports_metrics(&resource);

        loop {
            tokio::select! {
                event = stream.next() => {
                    let Some(event) = event else {
                        break;
                    };
                    match event {
                        Ok(WatcherEvent::Init) => {
                            init_buffer.clear();
                        }
                        Ok(WatcherEvent::InitApply(object)) => {
                            init_buffer.push(object);
                        }
                        Ok(WatcherEvent::InitDone) => {
                            metrics = self
                                .resource_metrics(&resource, namespace.as_deref())
                                .await
                                .unwrap_or_default();
                            known_objects = init_buffer
                                .drain(..)
                                .map(|object| (object_key(&object), object))
                                .collect();
                            on_event(ObjectWatchEvent::Restarted(object_summaries(
                                known_objects.values().cloned(),
                                &resource,
                                &metrics,
                            )));
                        }
                        Ok(WatcherEvent::Apply(object)) => {
                            let summary = object_summary(object.clone(), &resource, &metrics);
                            known_objects.insert(object_key(&object), object);
                            on_event(ObjectWatchEvent::Applied(summary));
                        }
                        Ok(WatcherEvent::Delete(object)) => {
                            let summary = object_summary(object.clone(), &resource, &metrics);
                            known_objects.remove(&object_key(&object));
                            on_event(ObjectWatchEvent::Deleted(summary));
                        }
                        Err(error) => {
                            on_event(ObjectWatchEvent::Error(error.to_string()));
                        }
                    }
                }
                _ = metrics_refresh.tick(), if refresh_metrics => {
                    if known_objects.is_empty() {
                        continue;
                    }
                    metrics = self
                        .resource_metrics(&resource, namespace.as_deref())
                        .await
                        .unwrap_or_default();
                    on_event(ObjectWatchEvent::Restarted(object_summaries(
                        known_objects.values().cloned(),
                        &resource,
                        &metrics,
                    )));
                }
            }
        }

        Ok(())
    }

    pub async fn object_detail(
        &self,
        resource: &ResourceKind,
        namespace: Option<&str>,
        name: &str,
    ) -> Result<ObjectDetail> {
        let api_resource = api_resource(resource);
        let objects: Api<DynamicObject> = match (resource.is_namespaced(), namespace) {
            (true, Some(namespace)) if !namespace.is_empty() && namespace != "-" => {
                Api::namespaced_with(self.client.clone(), namespace, &api_resource)
            }
            _ => Api::all_with(self.client.clone(), &api_resource),
        };
        let object = objects.get(name).await.with_context(|| {
            format!(
                "Could not load {} {name} {} using context {}.",
                resource.kind,
                resource_scope(resource, namespace),
                self.context
            )
        })?;
        let containers = object_containers(&object, resource);
        let replicas = object_replicas(&object, resource);
        let node_unschedulable = object_node_unschedulable(&object, resource);
        let conditions = object_conditions(&object);
        let metrics = self
            .resource_metrics(resource, namespace)
            .await
            .unwrap_or_default();
        let summary = object_summary(object.clone(), resource, &metrics);
        let container_metrics = if resource.kind == "Pod" && resource.group.is_empty() {
            self.pod_container_metrics(&summary.namespace, &summary.name)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let yaml = serde_yaml::to_string(&object).context("failed to serialize object YAML")?;
        let related_pods = self
            .deployment_pods(resource, &summary.namespace, &object)
            .await
            .unwrap_or_default();
        let events = self
            .object_events(resource, &summary.namespace, name)
            .await
            .map_err(|error| error.to_string());
        let (events, events_error) = match events {
            Ok(events) => (events, None),
            Err(error) => (Vec::new(), Some(error)),
        };

        Ok(ObjectDetail {
            name: summary.name,
            namespace: summary.namespace,
            status: summary.status,
            api_version: summary.api_version,
            kind: resource.kind.clone(),
            age: summary.age,
            metrics: summary.metrics,
            container_metrics,
            yaml,
            containers,
            related_pods,
            replicas,
            node_unschedulable,
            conditions,
            events,
            events_error,
        })
    }

    async fn deployment_pods(
        &self,
        resource: &ResourceKind,
        namespace: &str,
        object: &DynamicObject,
    ) -> Result<Vec<ObjectSummary>> {
        if resource.kind != "Deployment" || resource.group != "apps" || namespace == "-" {
            return Ok(Vec::new());
        }
        let Some(selector) = deployment_label_selector(object) else {
            return Ok(Vec::new());
        };

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        let pod_resource = ResourceKind {
            group: String::new(),
            version: String::from("v1"),
            api_version: String::from("v1"),
            kind: String::from("Pod"),
            plural: String::from("pods"),
            scope: ResourceScope::Namespaced,
        };
        let metrics = self
            .resource_metrics(&pod_resource, Some(namespace))
            .await
            .unwrap_or_default();
        let mut summaries = pods
            .list(&ListParams::default().labels(&selector))
            .await
            .with_context(|| {
                format!(
                    "Could not list Pods owned by Deployment {} in namespace {namespace}.",
                    object.name_any()
                )
            })?
            .items
            .into_iter()
            .map(|pod| {
                let object = serde_json::to_value(&pod)
                    .ok()
                    .and_then(|value| serde_json::from_value::<DynamicObject>(value).ok());
                object
                    .map(|object| object_summary(object, &pod_resource, &metrics))
                    .unwrap_or_else(|| ObjectSummary {
                        name: pod.name_any(),
                        namespace: namespace.to_owned(),
                        status: String::from("-"),
                        status_ratio: None,
                        api_version: String::from("v1"),
                        age: String::from("-"),
                        metrics: None,
                    })
            })
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(summaries)
    }
}

fn sort_object_summaries(objects: &mut [ObjectSummary]) {
    objects.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.name.cmp(&right.name))
    });
}

fn pod_summary(pod: Pod) -> PodSummary {
    let namespace = pod.namespace().unwrap_or_else(|| String::from("<cluster>"));
    let status = pod.status.as_ref();
    let phase = status
        .and_then(|status| status.phase.clone())
        .unwrap_or_else(|| String::from("Unknown"));
    let node = pod
        .spec
        .as_ref()
        .and_then(|spec| spec.node_name.clone())
        .unwrap_or_else(|| String::from("-"));
    let age = pod
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|timestamp| age_label(timestamp.0))
        .unwrap_or_else(|| String::from("-"));

    PodSummary {
        name: pod.name_any(),
        namespace,
        phase,
        node,
        age,
    }
}

fn supports_metrics(resource: &ResourceKind) -> bool {
    resource.group.is_empty() && matches!(resource.kind.as_str(), "Pod" | "Node")
}

fn object_key(object: &DynamicObject) -> (String, String) {
    (
        object.namespace().unwrap_or_else(|| String::from("-")),
        object.name_any(),
    )
}

fn object_summaries<I>(
    objects: I,
    resource: &ResourceKind,
    metrics: &BTreeMap<(String, String), ResourceUsage>,
) -> Vec<ObjectSummary>
where
    I: IntoIterator<Item = DynamicObject>,
{
    let mut summaries = objects
        .into_iter()
        .map(|object| object_summary(object, resource, metrics))
        .collect::<Vec<_>>();
    sort_object_summaries(&mut summaries);
    summaries
}

fn object_summary(
    object: DynamicObject,
    resource: &ResourceKind,
    metrics: &BTreeMap<(String, String), ResourceUsage>,
) -> ObjectSummary {
    let namespace = object.namespace().unwrap_or_else(|| String::from("-"));
    let name = object.name_any();
    let (status, status_ratio) = status_label(&object, resource);
    let age = object
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|timestamp| age_label(timestamp.0))
        .unwrap_or_else(|| String::from("-"));
    let mut usage = metrics.get(&(namespace.clone(), name.clone())).cloned();
    if let Some(usage) = usage.as_mut() {
        attach_resource_ratios(usage, &object, resource);
    }

    ObjectSummary {
        name,
        metrics: usage,
        namespace,
        status,
        status_ratio,
        api_version: resource.api_version.clone(),
        age,
    }
}

fn attach_resource_ratios(
    usage: &mut ResourceUsage,
    object: &DynamicObject,
    resource: &ResourceKind,
) {
    match (resource.group.as_str(), resource.kind.as_str()) {
        ("", "Pod") => {
            usage.cpu_ratio =
                resource_ratio(&usage.cpu, pod_container_requests_total(object, "cpu"));
            usage.memory_ratio = resource_ratio(
                &usage.memory,
                pod_container_requests_total(object, "memory"),
            );
        }
        ("", "Node") => {
            usage.cpu_ratio = resource_ratio(&usage.cpu, node_allocatable(object, "cpu"));
            usage.memory_ratio = resource_ratio(&usage.memory, node_allocatable(object, "memory"));
        }
        _ => {}
    }
}

fn pod_container_requests_total(object: &DynamicObject, resource_name: &str) -> Option<f64> {
    object
        .data
        .get("spec")?
        .get("containers")?
        .as_array()?
        .iter()
        .filter_map(|container| {
            container
                .get("resources")?
                .get("requests")?
                .get(resource_name)?
                .as_str()
                .and_then(quantity_as_f64)
        })
        .reduce(|total, quantity| total + quantity)
}

fn node_allocatable(object: &DynamicObject, resource_name: &str) -> Option<f64> {
    object
        .data
        .get("status")?
        .get("allocatable")?
        .get(resource_name)?
        .as_str()
        .and_then(quantity_as_f64)
}

fn resource_ratio(used: &str, base: Option<f64>) -> Option<ResourceRatio> {
    let used = quantity_as_f64(used)?;
    let base = base?;
    if !used.is_finite() || !base.is_finite() || base <= 0.0 {
        return None;
    }

    Some(ResourceRatio {
        basis_points: ((used / base) * 10_000.0)
            .round()
            .clamp(0.0, u32::MAX as f64) as u32,
    })
}

pub(crate) fn quantity_as_f64(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.is_empty() || value == "-" {
        return None;
    }

    for (suffix, multiplier) in [
        ("Ki", 1024.0),
        ("Mi", 1024.0_f64.powi(2)),
        ("Gi", 1024.0_f64.powi(3)),
        ("Ti", 1024.0_f64.powi(4)),
        ("Pi", 1024.0_f64.powi(5)),
        ("Ei", 1024.0_f64.powi(6)),
        ("n", 0.000_000_001),
        ("u", 0.000_001),
        ("m", 0.001),
        ("k", 1_000.0),
        ("K", 1_000.0),
        ("M", 1_000_000.0),
        ("G", 1_000_000_000.0),
        ("T", 1_000_000_000_000.0),
        ("P", 1_000_000_000_000_000.0),
        ("E", 1_000_000_000_000_000_000.0),
    ] {
        if let Some(number) = value.strip_suffix(suffix) {
            return number.parse::<f64>().ok().map(|number| number * multiplier);
        }
    }

    value.parse::<f64>().ok()
}

fn object_containers(object: &DynamicObject, resource: &ResourceKind) -> Vec<String> {
    if resource.kind != "Pod" || !resource.group.is_empty() {
        return Vec::new();
    }

    let mut containers = Vec::new();
    let Some(spec) = object.data.get("spec") else {
        return containers;
    };

    for field in ["containers", "initContainers", "ephemeralContainers"] {
        if let Some(items) = spec.get(field).and_then(serde_json::Value::as_array) {
            containers.extend(items.iter().filter_map(|container| {
                container
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
            }));
        }
    }

    containers
}

fn object_replicas(object: &DynamicObject, resource: &ResourceKind) -> Option<i32> {
    (resource.kind == "Deployment" && resource.group == "apps")
        .then(|| {
            object
                .data
                .get("spec")
                .and_then(|spec| spec.get("replicas"))
                .and_then(Value::as_i64)
                .and_then(|replicas| i32::try_from(replicas).ok())
        })
        .flatten()
}

fn object_node_unschedulable(object: &DynamicObject, resource: &ResourceKind) -> Option<bool> {
    (resource.kind == "Node" && resource.group.is_empty()).then(|| {
        object
            .data
            .get("spec")
            .and_then(|spec| spec.get("unschedulable"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn object_conditions(object: &DynamicObject) -> Vec<ObjectCondition> {
    object
        .data
        .get("status")
        .and_then(|status| status.get("conditions"))
        .and_then(Value::as_array)
        .map(|conditions| {
            conditions
                .iter()
                .map(|condition| ObjectCondition {
                    type_: condition_string(condition, "type"),
                    status: condition_string(condition, "status"),
                    reason: condition_string(condition, "reason"),
                    message: condition_string(condition, "message"),
                    last_transition: condition_string(condition, "lastTransitionTime"),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn condition_string(condition: &Value, field: &str) -> String {
    condition
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_owned()
}

fn deployment_label_selector(object: &DynamicObject) -> Option<String> {
    let labels = object
        .data
        .get("spec")?
        .get("selector")?
        .get("matchLabels")?
        .as_object()?;
    let mut parts = labels
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|value| format!("{key}={value}")))
        .collect::<Vec<_>>();
    parts.sort();
    (!parts.is_empty()).then(|| parts.join(","))
}

#[cfg(test)]
mod tests {
    use super::{quantity_as_f64, resource_ratio};

    #[test]
    fn quantity_as_f64_handles_kubernetes_cpu_and_memory_suffixes() {
        assert_eq!(quantity_as_f64("250m"), Some(0.25));
        assert_eq!(quantity_as_f64("500000000n"), Some(0.5));
        assert_eq!(quantity_as_f64("1"), Some(1.0));
        assert_eq!(quantity_as_f64("1Ki"), Some(1024.0));
        assert_eq!(quantity_as_f64("2Mi"), Some(2.0 * 1024.0 * 1024.0));
    }

    #[test]
    fn resource_ratio_uses_basis_points() {
        let ratio = resource_ratio("250m", Some(1.0)).expect("ratio should parse");

        assert_eq!(ratio.basis_points, 2500);
    }
}
