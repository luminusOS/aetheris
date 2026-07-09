use k8s_openapi::jiff::Timestamp;
use kube::api::DynamicObject;
use serde_json::Value;

use crate::ResourceKind;

pub(crate) fn status_label(
    object: &DynamicObject,
    resource: &ResourceKind,
) -> (String, Option<(i64, i64)>) {
    match (resource.group.as_str(), resource.kind.as_str()) {
        ("apps", "Deployment") => return deployment_status_label(object),
        ("apps", "StatefulSet") => return ready_replicas_status_label(object),
        ("apps", "ReplicaSet") => return ready_replicas_status_label(object),
        ("apps", "DaemonSet") => return daemonset_status_label(object),
        ("batch", "Job") => return job_status_label(object),
        ("batch", "CronJob") => return (cronjob_status_label(object), None),
        ("", "Pod") => return (pod_status_label(object), None),
        ("", "Node") => return (node_status_label(object), None),
        ("", "Service") => {
            return (
                spec_string(object, "type").unwrap_or_else(|| String::from("ClusterIP")),
                None,
            );
        }
        ("networking.k8s.io", "Ingress") => return (ingress_status_label(object), None),
        ("", "ConfigMap") => return (data_entries_status_label(object), None),
        ("", "Secret") => return (data_entries_status_label(object), None),
        _ => {}
    }

    let Some(status) = object.data.get("status") else {
        return (String::from("-"), None);
    };

    if let Some(phase) = status.get("phase").and_then(|value| value.as_str()) {
        return (phase.to_owned(), None);
    }

    if let Some(conditions) = status.get("conditions").and_then(|value| value.as_array())
        && let Some(ready) = conditions.iter().find(|condition| {
            condition.get("type").and_then(|value| value.as_str()) == Some("Ready")
        })
    {
        let label = ready
            .get("status")
            .and_then(|value| value.as_str())
            .map(|status| format!("Ready={status}"))
            .unwrap_or_else(|| String::from("Ready"));
        return (label, None);
    }

    let ready_replicas = status.get("readyReplicas").and_then(|value| value.as_i64());
    let replicas = status.get("replicas").and_then(|value| value.as_i64());
    if ready_replicas.is_some() || replicas.is_some() {
        let ready = ready_replicas.unwrap_or(0);
        let total = replicas.unwrap_or(0);
        return (format!("{ready}/{total}"), Some((ready, total)));
    }

    (String::from("-"), None)
}

fn deployment_status_label(object: &DynamicObject) -> (String, Option<(i64, i64)>) {
    let desired = spec_i64(object, "replicas").unwrap_or(1);
    let ready = status_i64(object, "readyReplicas").unwrap_or(0);
    let updated = status_i64(object, "updatedReplicas").unwrap_or(0);
    let available = status_i64(object, "availableReplicas").unwrap_or(0);

    if desired == 0 {
        return (String::from("Scaled 0"), Some((0, 0)));
    }
    let ratio = Some((ready, desired));
    if ready >= desired && available >= desired {
        return (format!("Ready {ready}/{desired}"), ratio);
    }
    if updated < desired {
        return (format!("Updating {ready}/{desired}"), ratio);
    }
    (format!("Unavailable {ready}/{desired}"), ratio)
}

fn ready_replicas_status_label(object: &DynamicObject) -> (String, Option<(i64, i64)>) {
    let desired = status_i64(object, "replicas")
        .or_else(|| spec_i64(object, "replicas"))
        .unwrap_or(0);
    let ready = status_i64(object, "readyReplicas").unwrap_or(0);

    if desired == 0 {
        (String::from("Scaled 0"), Some((0, 0)))
    } else if ready >= desired {
        (format!("Ready {ready}/{desired}"), Some((ready, desired)))
    } else {
        (
            format!("Progressing {ready}/{desired}"),
            Some((ready, desired)),
        )
    }
}

fn daemonset_status_label(object: &DynamicObject) -> (String, Option<(i64, i64)>) {
    let desired = status_i64(object, "desiredNumberScheduled").unwrap_or(0);
    let ready = status_i64(object, "numberReady").unwrap_or(0);
    let unavailable = status_i64(object, "numberUnavailable").unwrap_or(0);

    if desired == 0 {
        (String::from("No nodes"), Some((0, 0)))
    } else if ready >= desired && unavailable == 0 {
        (format!("Ready {ready}/{desired}"), Some((ready, desired)))
    } else {
        (
            format!("Unavailable {ready}/{desired}"),
            Some((ready, desired)),
        )
    }
}

fn job_status_label(object: &DynamicObject) -> (String, Option<(i64, i64)>) {
    let completions = spec_i64(object, "completions").unwrap_or(1);
    let succeeded = status_i64(object, "succeeded").unwrap_or(0);
    let failed = status_i64(object, "failed").unwrap_or(0);
    let active = status_i64(object, "active").unwrap_or(0);
    let ratio = Some((succeeded, completions));

    if succeeded >= completions {
        (format!("Complete {succeeded}/{completions}"), ratio)
    } else if failed > 0 {
        (format!("Failed {failed}"), ratio)
    } else if active > 0 {
        (format!("Running {succeeded}/{completions}"), ratio)
    } else {
        (format!("Pending {succeeded}/{completions}"), ratio)
    }
}

fn cronjob_status_label(object: &DynamicObject) -> String {
    if spec_bool(object, "suspend").unwrap_or(false) {
        return String::from("Suspended");
    }
    let active = object
        .data
        .get("status")
        .and_then(|status| status.get("active"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if active > 0 {
        format!("Active {active}")
    } else {
        String::from("Scheduled")
    }
}

fn pod_status_label(object: &DynamicObject) -> String {
    let phase = object
        .data
        .get("status")
        .and_then(|status| status.get("phase"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown");
    if phase == "Running" {
        if condition_status(object, "Ready").as_deref() == Some("True") {
            String::from("Ready")
        } else {
            String::from("Running")
        }
    } else {
        phase.to_owned()
    }
}

fn node_status_label(object: &DynamicObject) -> String {
    let ready = match condition_status(object, "Ready").as_deref() {
        Some("True") => "Ready",
        Some("False") => "NotReady",
        _ => "Unknown",
    };
    if spec_bool(object, "unschedulable").unwrap_or(false) {
        format!("{ready} SchedulingDisabled")
    } else {
        ready.to_owned()
    }
}

fn ingress_status_label(object: &DynamicObject) -> String {
    let ingress_count = object
        .data
        .get("status")
        .and_then(|status| status.get("loadBalancer"))
        .and_then(|load_balancer| load_balancer.get("ingress"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if ingress_count > 0 {
        format!("Ready {ingress_count}")
    } else {
        String::from("Pending")
    }
}

fn data_entries_status_label(object: &DynamicObject) -> String {
    let entries = object
        .data
        .get("data")
        .and_then(Value::as_object)
        .map(serde_json::Map::len)
        .unwrap_or(0);
    format!("{entries} keys")
}

fn condition_status(object: &DynamicObject, condition_type: &str) -> Option<String> {
    object
        .data
        .get("status")
        .and_then(|status| status.get("conditions"))
        .and_then(Value::as_array)
        .and_then(|conditions| {
            conditions.iter().find(|condition| {
                condition.get("type").and_then(Value::as_str) == Some(condition_type)
            })
        })
        .and_then(|condition| condition.get("status"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn status_i64(object: &DynamicObject, key: &str) -> Option<i64> {
    object
        .data
        .get("status")
        .and_then(|status| status.get(key))
        .and_then(Value::as_i64)
}

fn spec_i64(object: &DynamicObject, key: &str) -> Option<i64> {
    object
        .data
        .get("spec")
        .and_then(|spec| spec.get(key))
        .and_then(Value::as_i64)
}

fn spec_bool(object: &DynamicObject, key: &str) -> Option<bool> {
    object
        .data
        .get("spec")
        .and_then(|spec| spec.get(key))
        .and_then(Value::as_bool)
}

fn spec_string(object: &DynamicObject, key: &str) -> Option<String> {
    object
        .data
        .get("spec")
        .and_then(|spec| spec.get(key))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

pub(crate) fn age_label(created_at: Timestamp) -> String {
    let age = Timestamp::now().duration_since(created_at);
    let seconds = age.as_secs().max(0);

    if seconds >= 86_400 {
        format!("{}d", seconds / 86_400)
    } else if seconds >= 3_600 {
        format!("{}h", seconds / 3_600)
    } else if seconds >= 60 {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

#[cfg(test)]
mod tests {
    use k8s_openapi::jiff::{SignedDuration, Timestamp};
    use kube::api::DynamicObject;

    use super::{age_label, deployment_status_label, job_status_label};

    #[test]
    fn age_label_uses_largest_relevant_unit() {
        let now = Timestamp::now();

        assert!(age_label(now - SignedDuration::from_hours(48)).ends_with('d'));
        assert!(age_label(now - SignedDuration::from_hours(3)).ends_with('h'));
        assert!(age_label(now - SignedDuration::from_mins(12)).ends_with('m'));
    }

    #[test]
    fn deployment_status_label_reports_ready_ratio() {
        let object: DynamicObject = serde_json::from_value(serde_json::json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": { "name": "web" },
            "spec": { "replicas": 20 },
            "status": {
                "readyReplicas": 20,
                "updatedReplicas": 20,
                "availableReplicas": 20
            }
        }))
        .unwrap();

        let (label, ratio) = deployment_status_label(&object);

        assert_eq!(label, "Ready 20/20");
        assert_eq!(ratio, Some((20, 20)));
    }

    #[test]
    fn deployment_status_label_reports_ratio_while_updating() {
        let object: DynamicObject = serde_json::from_value(serde_json::json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": { "name": "web" },
            "spec": { "replicas": 10 },
            "status": {
                "readyReplicas": 3,
                "updatedReplicas": 3,
                "availableReplicas": 3
            }
        }))
        .unwrap();

        let (label, ratio) = deployment_status_label(&object);

        assert_eq!(label, "Updating 3/10");
        assert_eq!(ratio, Some((3, 10)));
    }

    #[test]
    fn job_status_label_reports_completions_ratio() {
        let object: DynamicObject = serde_json::from_value(serde_json::json!({
            "apiVersion": "batch/v1",
            "kind": "Job",
            "metadata": { "name": "migrate" },
            "spec": { "completions": 3 },
            "status": { "succeeded": 1, "active": 1 }
        }))
        .unwrap();

        let (label, ratio) = job_status_label(&object);

        assert_eq!(label, "Running 1/3");
        assert_eq!(ratio, Some((1, 3)));
    }
}
