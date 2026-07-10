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
    ContainerResources, IngressRule, KubeSession, ObjectCondition, ObjectDetail, ObjectSummary,
    ObjectWatchEvent, PodStateCount, PodSummary, ResourceKind, ResourceRatio, ResourceScope,
    ResourceUsage, ServicePort, ServiceSelector, api_resource, namespace_scope, resource_scope,
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
        let container_resources = object_container_resources(&object, resource);
        let containers = container_resources
            .iter()
            .map(|container| container.name.clone())
            .collect();
        let replicas = object_replicas(&object, resource);
        let node_unschedulable = object_node_unschedulable(&object, resource);
        let conditions = object_conditions(&object);
        let object_namespace = object.namespace().unwrap_or_else(|| String::from("-"));
        let object_name = object.name_any();
        let is_pod = resource.kind == "Pod" && resource.group.is_empty();
        let (metrics, container_metrics) = if is_pod {
            let (metrics, container_metrics) = tokio::join!(
                self.resource_metrics(resource, namespace),
                self.pod_container_metrics(&object_namespace, &object_name),
            );
            (
                metrics.unwrap_or_default(),
                container_metrics.unwrap_or_default(),
            )
        } else {
            (
                self.resource_metrics(resource, namespace)
                    .await
                    .unwrap_or_default(),
                Vec::new(),
            )
        };
        let summary = object_summary(object.clone(), resource, &metrics);
        let yaml = serde_yaml::to_string(&object).context("failed to serialize object YAML")?;
        let deployment_pods = self
            .deployment_pods(resource, &summary.namespace, &object)
            .await
            .unwrap_or_default();
        let (service_ports, service_selectors) = service_details(&object, resource);
        let ingress_rules = ingress_rules(&object, resource);
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
            container_resources,
            yaml,
            containers,
            related_pods: deployment_pods.summaries,
            related_pod_states: deployment_pods.states,
            service_ports,
            service_selectors,
            ingress_rules,
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
    ) -> Result<DeploymentPods> {
        if resource.kind != "Deployment" || resource.group != "apps" || namespace == "-" {
            return Ok(DeploymentPods::default());
        }
        let Some(selector) = deployment_label_selector(object) else {
            return Ok(DeploymentPods::default());
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
        let pods = pods
            .list(&ListParams::default().labels(&selector))
            .await
            .with_context(|| {
                format!(
                    "Could not list Pods owned by Deployment {} in namespace {namespace}.",
                    object.name_any()
                )
            })?
            .items;
        let states = pod_state_counts(&pods);
        let mut summaries = pods
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
                        images: Vec::new(),
                        service_target: String::new(),
                        service_selector: String::new(),
                        ingress_target: String::new(),
                        ingress_class: String::new(),
                        metrics: None,
                    })
            })
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(DeploymentPods { summaries, states })
    }
}

#[derive(Default)]
struct DeploymentPods {
    summaries: Vec<ObjectSummary>,
    states: Vec<PodStateCount>,
}

fn pod_state_counts(pods: &[Pod]) -> Vec<PodStateCount> {
    const ORDER: [&str; 5] = ["Running", "Pending", "Succeeded", "Failed", "Unknown"];

    let mut counts = BTreeMap::<String, u32>::new();
    for pod in pods {
        let state = pod
            .status
            .as_ref()
            .and_then(|status| status.phase.as_deref())
            .unwrap_or("Unknown");
        *counts.entry(state.to_owned()).or_default() += 1;
    }

    let mut states = ORDER
        .into_iter()
        .filter_map(|state| {
            counts.remove(state).map(|count| PodStateCount {
                state: state.to_owned(),
                count,
            })
        })
        .collect::<Vec<_>>();
    states.extend(
        counts
            .into_iter()
            .map(|(state, count)| PodStateCount { state, count }),
    );
    states
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
        images: object_images(&object, resource),
        service_target: service_target(&object, resource),
        service_selector: service_selector(&object, resource),
        ingress_target: ingress_target(&object, resource),
        ingress_class: ingress_class(&object, resource),
    }
}

fn is_ingress(resource: &ResourceKind) -> bool {
    resource.group == "networking.k8s.io" && resource.kind == "Ingress"
}

fn ingress_target(object: &DynamicObject, resource: &ResourceKind) -> String {
    ingress_rules(object, resource)
        .into_iter()
        .map(|rule| {
            format!(
                "{}{} → {}:{}",
                rule.host, rule.path, rule.service, rule.port
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn ingress_class(object: &DynamicObject, resource: &ResourceKind) -> String {
    if !is_ingress(resource) {
        return String::new();
    }

    object
        .data
        .get("spec")
        .and_then(|spec| spec.get("ingressClassName"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn ingress_rules(object: &DynamicObject, resource: &ResourceKind) -> Vec<IngressRule> {
    if !is_ingress(resource) {
        return Vec::new();
    }

    let Some(spec) = object.data.get("spec") else {
        return Vec::new();
    };
    let mut rules = Vec::new();
    if let Some(backend) = spec.get("defaultBackend")
        && let Some(rule) = ingress_rule_from_backend(backend, "*", "/", "Default")
    {
        rules.push(rule);
    }
    if let Some(ingress_rules) = spec.get("rules").and_then(Value::as_array) {
        for rule in ingress_rules {
            let host = rule.get("host").and_then(Value::as_str).unwrap_or("*");
            let Some(paths) = rule
                .get("http")
                .and_then(|http| http.get("paths"))
                .and_then(Value::as_array)
            else {
                continue;
            };
            for path in paths {
                let path_value = path.get("path").and_then(Value::as_str).unwrap_or("/");
                let path_type = path
                    .get("pathType")
                    .and_then(Value::as_str)
                    .unwrap_or("ImplementationSpecific");
                if let Some(rule) = path.get("backend").and_then(|backend| {
                    ingress_rule_from_backend(backend, host, path_value, path_type)
                }) {
                    rules.push(rule);
                }
            }
        }
    }
    rules
}

fn ingress_rule_from_backend(
    backend: &Value,
    host: &str,
    path: &str,
    path_type: &str,
) -> Option<IngressRule> {
    let service = backend.get("service")?;
    let service_name = service.get("name").and_then(Value::as_str)?;
    let port = service.get("port")?;
    let port = value_string(port.get("name")).or_else(|| value_string(port.get("number")))?;
    Some(IngressRule {
        host: host.to_owned(),
        path: path.to_owned(),
        path_type: path_type.to_owned(),
        service: service_name.to_owned(),
        port,
    })
}

fn service_target(object: &DynamicObject, resource: &ResourceKind) -> String {
    service_ports(object, resource)
        .into_iter()
        .map(|port| format!("{}:{}/{}", port.port, port.target_port, port.protocol))
        .collect::<Vec<_>>()
        .join(", ")
}

fn service_selector(object: &DynamicObject, resource: &ResourceKind) -> String {
    service_selectors(object, resource)
        .into_iter()
        .map(|selector| format!("{}={}", selector.key, selector.value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn service_details(
    object: &DynamicObject,
    resource: &ResourceKind,
) -> (Vec<ServicePort>, Vec<ServiceSelector>) {
    (
        service_ports(object, resource),
        service_selectors(object, resource),
    )
}

fn service_ports(object: &DynamicObject, resource: &ResourceKind) -> Vec<ServicePort> {
    if resource.kind != "Service" || !resource.group.is_empty() {
        return Vec::new();
    }

    object
        .data
        .get("spec")
        .and_then(|spec| spec.get("ports"))
        .and_then(Value::as_array)
        .map(|ports| {
            ports
                .iter()
                .filter_map(|port| {
                    let port_number = value_string(port.get("port"))?;
                    let target_port =
                        value_string(port.get("targetPort")).unwrap_or_else(|| port_number.clone());
                    Some(ServicePort {
                        name: value_string(port.get("name")).unwrap_or_default(),
                        protocol: value_string(port.get("protocol"))
                            .unwrap_or_else(|| String::from("TCP")),
                        port: port_number,
                        target_port,
                        node_port: value_string(port.get("nodePort")),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn service_selectors(object: &DynamicObject, resource: &ResourceKind) -> Vec<ServiceSelector> {
    if resource.kind != "Service" || !resource.group.is_empty() {
        return Vec::new();
    }

    let mut selectors = object
        .data
        .get("spec")
        .and_then(|spec| spec.get("selector"))
        .and_then(Value::as_object)
        .map(|selectors| {
            selectors
                .iter()
                .filter_map(|(key, value)| {
                    value_string(Some(value)).map(|value| ServiceSelector {
                        key: key.clone(),
                        value,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    selectors.sort_by(|left, right| left.key.cmp(&right.key));
    selectors
}

fn value_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|value| match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    })
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

fn object_container_resources(
    object: &DynamicObject,
    resource: &ResourceKind,
) -> Vec<ContainerResources> {
    if resource.kind != "Pod" || !resource.group.is_empty() {
        return Vec::new();
    }

    let mut containers = Vec::new();
    let Some(spec) = object.data.get("spec") else {
        return containers;
    };

    for field in ["containers", "initContainers", "ephemeralContainers"] {
        if let Some(items) = spec.get(field).and_then(serde_json::Value::as_array) {
            containers.extend(items.iter().filter_map(container_resources));
        }
    }

    containers
}

fn container_resources(container: &Value) -> Option<ContainerResources> {
    let name = container.get("name")?.as_str()?.to_owned();
    let resources = container.get("resources");

    Some(ContainerResources {
        name,
        cpu_request: resource_quantity(resources, "requests", "cpu"),
        cpu_limit: resource_quantity(resources, "limits", "cpu"),
        memory_request: resource_quantity(resources, "requests", "memory"),
        memory_limit: resource_quantity(resources, "limits", "memory"),
    })
}

fn resource_quantity(resources: Option<&Value>, section: &str, name: &str) -> String {
    resources
        .and_then(|resources| resources.get(section))
        .and_then(|section| section.get(name))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("-")
        .to_owned()
}

fn object_images(object: &DynamicObject, resource: &ResourceKind) -> Vec<String> {
    if resource.kind != "Pod" || !resource.group.is_empty() {
        return Vec::new();
    }

    object
        .data
        .get("spec")
        .and_then(|spec| spec.get("containers"))
        .and_then(serde_json::Value::as_array)
        .map(|containers| {
            containers
                .iter()
                .filter_map(|container| {
                    container
                        .get("image")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
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
    use super::{
        ingress_class, ingress_rules, ingress_target, object_container_resources, object_images,
        pod_state_counts, quantity_as_f64, resource_ratio, service_details, service_selector,
        service_target,
    };
    use crate::{
        ContainerResources, IngressRule, ResourceKind, ResourceScope, ServicePort, ServiceSelector,
    };
    use k8s_openapi::api::core::v1::Pod;
    use kube::api::DynamicObject;
    use serde_json::json;

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

    #[test]
    fn object_images_uses_pod_spec_containers_only() {
        let pod: DynamicObject = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": "sample",
                "namespace": "my-namespace"
            },
            "spec": {
                "containers": [
                    {"name": "app", "image": "docker.io/library/nginx:latest"},
                    {"name": "sidecar", "image": "example.com/sidecar:v1"}
                ],
                "initContainers": [
                    {"name": "init", "image": "example.com/init:v1"}
                ]
            }
        }))
        .expect("pod should deserialize");
        let resource = ResourceKind {
            group: String::new(),
            version: String::from("v1"),
            api_version: String::from("v1"),
            kind: String::from("Pod"),
            plural: String::from("pods"),
            scope: ResourceScope::Namespaced,
        };

        assert_eq!(
            object_images(&pod, &resource),
            vec![
                String::from("docker.io/library/nginx:latest"),
                String::from("example.com/sidecar:v1")
            ]
        );
    }

    #[test]
    fn object_container_resources_reads_pod_container_requests_and_limits() {
        let pod: DynamicObject = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": "sample",
                "namespace": "my-namespace"
            },
            "spec": {
                "containers": [
                    {
                        "name": "app",
                        "resources": {
                            "requests": {"cpu": "250m", "memory": "128Mi"},
                            "limits": {"cpu": "500m", "memory": "256Mi"}
                        }
                    },
                    {
                        "name": "sidecar",
                        "resources": {
                            "requests": {"cpu": "100m"}
                        }
                    }
                ],
                "initContainers": [
                    {
                        "name": "init",
                        "resources": {
                            "limits": {"memory": "64Mi"}
                        }
                    }
                ]
            }
        }))
        .expect("pod should deserialize");
        let resource = ResourceKind {
            group: String::new(),
            version: String::from("v1"),
            api_version: String::from("v1"),
            kind: String::from("Pod"),
            plural: String::from("pods"),
            scope: ResourceScope::Namespaced,
        };

        assert_eq!(
            object_container_resources(&pod, &resource),
            vec![
                ContainerResources {
                    name: String::from("app"),
                    cpu_request: String::from("250m"),
                    cpu_limit: String::from("500m"),
                    memory_request: String::from("128Mi"),
                    memory_limit: String::from("256Mi"),
                },
                ContainerResources {
                    name: String::from("sidecar"),
                    cpu_request: String::from("100m"),
                    cpu_limit: String::from("-"),
                    memory_request: String::from("-"),
                    memory_limit: String::from("-"),
                },
                ContainerResources {
                    name: String::from("init"),
                    cpu_request: String::from("-"),
                    cpu_limit: String::from("-"),
                    memory_request: String::from("-"),
                    memory_limit: String::from("64Mi"),
                },
            ]
        );
    }

    #[test]
    fn pod_state_counts_groups_phases_in_dashboard_order() {
        let pods: Vec<Pod> = serde_json::from_value(json!([
            {"apiVersion": "v1", "kind": "Pod", "status": {"phase": "Pending"}},
            {"apiVersion": "v1", "kind": "Pod", "status": {"phase": "Running"}},
            {"apiVersion": "v1", "kind": "Pod", "status": {"phase": "Running"}},
            {"apiVersion": "v1", "kind": "Pod", "status": {"phase": "Failed"}},
            {"apiVersion": "v1", "kind": "Pod", "status": {}}
        ]))
        .expect("pods should deserialize");

        assert_eq!(
            pod_state_counts(&pods),
            vec![
                crate::PodStateCount {
                    state: String::from("Running"),
                    count: 2
                },
                crate::PodStateCount {
                    state: String::from("Pending"),
                    count: 1
                },
                crate::PodStateCount {
                    state: String::from("Failed"),
                    count: 1
                },
                crate::PodStateCount {
                    state: String::from("Unknown"),
                    count: 1
                },
            ]
        );
    }

    #[test]
    fn service_details_extract_ports_targets_and_sorted_selectors() {
        let service: DynamicObject = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Service",
            "spec": {
                "selector": {"tier": "frontend", "app": "web"},
                "ports": [
                    {"name": "http", "port": 80, "targetPort": 8080, "protocol": "TCP"},
                    {"name": "metrics", "port": 9090, "targetPort": "metrics", "protocol": "TCP", "nodePort": 32090}
                ]
            }
        }))
        .expect("service should deserialize");
        let resource = ResourceKind {
            group: String::new(),
            version: String::from("v1"),
            api_version: String::from("v1"),
            kind: String::from("Service"),
            plural: String::from("services"),
            scope: ResourceScope::Namespaced,
        };

        let (ports, selectors) = service_details(&service, &resource);
        assert_eq!(
            ports,
            vec![
                ServicePort {
                    name: String::from("http"),
                    protocol: String::from("TCP"),
                    port: String::from("80"),
                    target_port: String::from("8080"),
                    node_port: None,
                },
                ServicePort {
                    name: String::from("metrics"),
                    protocol: String::from("TCP"),
                    port: String::from("9090"),
                    target_port: String::from("metrics"),
                    node_port: Some(String::from("32090")),
                },
            ]
        );
        assert_eq!(
            selectors,
            vec![
                ServiceSelector {
                    key: String::from("app"),
                    value: String::from("web")
                },
                ServiceSelector {
                    key: String::from("tier"),
                    value: String::from("frontend")
                },
            ]
        );
        assert_eq!(
            service_target(&service, &resource),
            "80:8080/TCP, 9090:metrics/TCP"
        );
        assert_eq!(
            service_selector(&service, &resource),
            "app=web, tier=frontend"
        );
    }

    #[test]
    fn ingress_rules_extract_targets_and_ingress_class() {
        let ingress: DynamicObject = serde_json::from_value(json!({
            "apiVersion": "networking.k8s.io/v1",
            "kind": "Ingress",
            "spec": {
                "ingressClassName": "nginx",
                "defaultBackend": {
                    "service": {"name": "fallback", "port": {"number": 80}}
                },
                "rules": [{
                    "host": "app.example.com",
                    "http": {"paths": [{
                        "path": "/api",
                        "pathType": "Prefix",
                        "backend": {"service": {"name": "api", "port": {"number": 8080}}}
                    }, {
                        "path": "/metrics",
                        "pathType": "Exact",
                        "backend": {"service": {"name": "metrics", "port": {"name": "http"}}}
                    }]}
                }]
            }
        }))
        .expect("ingress should deserialize");
        let resource = ResourceKind {
            group: String::from("networking.k8s.io"),
            version: String::from("v1"),
            api_version: String::from("networking.k8s.io/v1"),
            kind: String::from("Ingress"),
            plural: String::from("ingresses"),
            scope: ResourceScope::Namespaced,
        };

        assert_eq!(ingress_class(&ingress, &resource), "nginx");
        assert_eq!(
            ingress_rules(&ingress, &resource),
            vec![
                IngressRule {
                    host: String::from("*"),
                    path: String::from("/"),
                    path_type: String::from("Default"),
                    service: String::from("fallback"),
                    port: String::from("80"),
                },
                IngressRule {
                    host: String::from("app.example.com"),
                    path: String::from("/api"),
                    path_type: String::from("Prefix"),
                    service: String::from("api"),
                    port: String::from("8080"),
                },
                IngressRule {
                    host: String::from("app.example.com"),
                    path: String::from("/metrics"),
                    path_type: String::from("Exact"),
                    service: String::from("metrics"),
                    port: String::from("http"),
                },
            ]
        );
        assert_eq!(
            ingress_target(&ingress, &resource),
            "*/ → fallback:80, app.example.com/api → api:8080, app.example.com/metrics → metrics:http"
        );
    }
}
