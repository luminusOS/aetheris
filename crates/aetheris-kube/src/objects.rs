use std::collections::BTreeMap;

use anyhow::{Context as AnyhowContext, Result};
use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{DynamicObject, ListParams};
use kube::runtime::watcher::{watcher, Event as WatcherEvent};
use kube::{Api, ResourceExt};
use serde_json::Value;

use crate::status::{age_label, status_label};
use crate::{
    api_resource, namespace_scope, resource_scope, KubeSession, ObjectCondition, ObjectDetail,
    ObjectSummary, ObjectWatchEvent, PodSummary, ResourceKind, ResourceScope, ResourceUsage,
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
        let metrics = self
            .resource_metrics(&resource, namespace.as_deref())
            .await
            .unwrap_or_default();
        let mut stream = Box::pin(watcher(objects, kube::runtime::watcher::Config::default()));
        let mut init_buffer = Vec::new();

        while let Some(event) = stream.next().await {
            match event {
                Ok(WatcherEvent::Init) => {
                    init_buffer.clear();
                }
                Ok(WatcherEvent::InitApply(object)) => {
                    init_buffer.push(object_summary(object, &resource, &metrics));
                }
                Ok(WatcherEvent::InitDone) => {
                    sort_object_summaries(&mut init_buffer);
                    on_event(ObjectWatchEvent::Restarted(std::mem::take(
                        &mut init_buffer,
                    )));
                }
                Ok(WatcherEvent::Apply(object)) => {
                    on_event(ObjectWatchEvent::Applied(object_summary(
                        object, &resource, &metrics,
                    )));
                }
                Ok(WatcherEvent::Delete(object)) => {
                    on_event(ObjectWatchEvent::Deleted(object_summary(
                        object, &resource, &metrics,
                    )));
                }
                Err(error) => {
                    on_event(ObjectWatchEvent::Error(error.to_string()));
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

fn object_summary(
    object: DynamicObject,
    resource: &ResourceKind,
    metrics: &BTreeMap<(String, String), ResourceUsage>,
) -> ObjectSummary {
    let namespace = object.namespace().unwrap_or_else(|| String::from("-"));
    let (status, status_ratio) = status_label(&object, resource);
    let age = object
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|timestamp| age_label(timestamp.0))
        .unwrap_or_else(|| String::from("-"));

    ObjectSummary {
        name: object.name_any(),
        metrics: metrics
            .get(&(namespace.clone(), object.name_any()))
            .cloned(),
        namespace,
        status,
        status_ratio,
        api_version: resource.api_version.clone(),
        age,
    }
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
