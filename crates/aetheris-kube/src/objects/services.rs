use kube::api::DynamicObject;
use serde_json::Value;

use crate::{ResourceKind, ServicePort, ServiceSelector};

pub(super) fn service_target(object: &DynamicObject, resource: &ResourceKind) -> String {
    service_ports(object, resource)
        .into_iter()
        .map(|port| format!("{}:{}/{}", port.port, port.target_port, port.protocol))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn service_selector(object: &DynamicObject, resource: &ResourceKind) -> String {
    service_selectors(object, resource)
        .into_iter()
        .map(|selector| format!("{}={}", selector.key, selector.value))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn service_details(
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

pub(super) fn value_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|value| match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    })
}
