use std::collections::BTreeMap;

use k8s_openapi::api::core::v1::Pod;
use kube::ResourceExt;
use kube::api::DynamicObject;

use crate::status::{age_label, status_label};
use crate::{ObjectSummary, PodStateCount, PodSummary, ResourceKind, ResourceUsage};

use super::ingress::{ingress_class, ingress_target};
use super::resources::{attach_resource_ratios, object_images};
use super::services::{service_selector, service_target};

pub(super) fn pod_state_counts(pods: &[Pod]) -> Vec<PodStateCount> {
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

pub(super) fn sort_object_summaries(objects: &mut [ObjectSummary]) {
    objects.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.name.cmp(&right.name))
    });
}

pub(super) fn pod_summary(pod: Pod) -> PodSummary {
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

pub(super) fn supports_metrics(resource: &ResourceKind) -> bool {
    resource.group.is_empty() && matches!(resource.kind.as_str(), "Pod" | "Node")
}

pub(super) fn object_key(object: &DynamicObject) -> (String, String) {
    (
        object.namespace().unwrap_or_else(|| String::from("-")),
        object.name_any(),
    )
}

pub(super) fn object_summaries<I>(
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

pub(super) fn object_summary(
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
