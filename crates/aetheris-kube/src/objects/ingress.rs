use kube::api::DynamicObject;
use serde_json::Value;

use crate::{IngressRule, ResourceKind};

use super::services::value_string;

fn is_ingress(resource: &ResourceKind) -> bool {
    resource.group == "networking.k8s.io" && resource.kind == "Ingress"
}

pub(super) fn ingress_target(object: &DynamicObject, resource: &ResourceKind) -> String {
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

pub(super) fn ingress_class(object: &DynamicObject, resource: &ResourceKind) -> String {
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

pub(super) fn ingress_rules(object: &DynamicObject, resource: &ResourceKind) -> Vec<IngressRule> {
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
