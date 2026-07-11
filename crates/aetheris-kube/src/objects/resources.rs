use kube::api::DynamicObject;
use serde_json::Value;

use crate::{ResourceKind, ResourceRatio, ResourceUsage};

pub(super) fn attach_resource_ratios(
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

pub(super) fn resource_ratio(used: &str, base: Option<f64>) -> Option<ResourceRatio> {
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

pub(super) fn object_images(object: &DynamicObject, resource: &ResourceKind) -> Vec<String> {
    if resource.kind != "Pod" || !resource.group.is_empty() {
        return Vec::new();
    }

    object
        .data
        .get("spec")
        .and_then(|spec| spec.get("containers"))
        .and_then(Value::as_array)
        .map(|containers| {
            containers
                .iter()
                .filter_map(|container| {
                    container
                        .get("image")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}
