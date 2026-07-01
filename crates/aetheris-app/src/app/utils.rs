use super::*;

pub(super) fn text_buffer_text(buffer: &impl IsA<gtk::TextBuffer>) -> String {
    let buffer = buffer.as_ref();
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

pub(super) fn format_error(error: anyhow::Error) -> String {
    let mut chain = error.chain();
    let headline = chain
        .next()
        .map(ToString::to_string)
        .unwrap_or_else(|| String::from("Unknown error"));

    // Only the root cause, not every link: middle-of-the-chain wrapping
    // context tends to just repeat the headline in other words.
    let Some(cause) = chain.last().map(ToString::to_string) else {
        return headline;
    };

    let cause = forbidden_summary(&cause).unwrap_or(cause);
    // adw::Toast renders on a single line; embedded newlines show up as a
    // literal "↵" glyph instead of wrapping, so keep this on one line.
    format!("{headline} ({cause})")
}

/// Kubernetes RBAC denials spell out the exact user, resource, API group,
/// and scope, e.g. `namespaces is forbidden: User "..." cannot list
/// resource "namespaces" in API group "" at the cluster scope` — all noise
/// for a toast. Keep just "<resource> is forbidden".
fn forbidden_summary(message: &str) -> Option<String> {
    let words: Vec<&str> = message.split_whitespace().collect();
    let position = words.iter().position(|word| word.starts_with("forbidden"))?;
    if position < 2 || words[position - 1] != "is" {
        return None;
    }
    Some(format!("{} is forbidden", words[position - 2]))
}

pub(super) fn object_matches(object: &ObjectSummary, query: &str) -> bool {
    object.name.to_ascii_lowercase().contains(query)
        || object.namespace.to_ascii_lowercase().contains(query)
        || object.status.to_ascii_lowercase().contains(query)
        || object.api_version.to_ascii_lowercase().contains(query)
        || object.metrics.as_ref().is_some_and(|usage| {
            usage.cpu.to_ascii_lowercase().contains(query)
                || usage.memory.to_ascii_lowercase().contains(query)
        })
}

pub(super) fn pod_resource_kind() -> ResourceKind {
    ResourceKind {
        group: String::new(),
        version: String::from("v1"),
        api_version: String::from("v1"),
        kind: String::from("Pod"),
        plural: String::from("pods"),
        scope: aetheris_kube::ResourceScope::Namespaced,
    }
}

pub(super) fn is_deployment_resource(resource: &ResourceKind) -> bool {
    resource.kind == "Deployment" && resource.group == "apps"
}

pub(super) fn is_node_resource(resource: &ResourceKind) -> bool {
    resource.kind == "Node" && resource.group.is_empty()
}

/// Whether this resource kind ever exposes a ready/desired status ratio
/// (mirrors the kinds `aetheris_kube::status_label` computes one for).
pub(super) fn supports_status_ratio(resource: &ResourceKind) -> bool {
    matches!(
        (resource.group.as_str(), resource.kind.as_str()),
        ("apps", "Deployment")
            | ("apps", "StatefulSet")
            | ("apps", "ReplicaSet")
            | ("apps", "DaemonSet")
            | ("batch", "Job")
    )
}

/// Whether this resource kind is ever queried against metrics.k8s.io
/// (mirrors `KubeManager::resource_metrics`, which only looks up Pod and
/// Node metrics — everything else always comes back empty).
pub(super) fn supports_metrics(resource: &ResourceKind) -> bool {
    resource.group.is_empty() && matches!(resource.kind.as_str(), "Pod" | "Node")
}

/// Columns that make sense to render for `resource` (e.g. "Status" only for
/// kinds with a ready/desired ratio; "CPU"/"Memory" only for Pods and
/// Nodes). Shared by the main object list and the related-Pods table in the
/// Deployment detail page, which always shows Pods regardless of which
/// resource kind is selected in the main list.
pub(super) fn offerable_columns_for(resource: Option<&ResourceKind>) -> Vec<ObjectColumn> {
    let has_status_ratio = resource.is_some_and(supports_status_ratio);
    let has_metrics = resource.is_some_and(supports_metrics);
    ObjectColumn::ALL
        .into_iter()
        .filter(|column| match column {
            ObjectColumn::Status => has_status_ratio,
            ObjectColumn::Cpu | ObjectColumn::Memory => has_metrics,
            _ => true,
        })
        .collect()
}

pub(super) fn pod_log_target(
    context: String,
    resource: &ResourceKind,
    namespace: Option<String>,
    pod: String,
) -> Option<PodLogTarget> {
    if resource.kind != "Pod" || !resource.group.is_empty() {
        return None;
    }

    Some(PodLogTarget {
        context,
        namespace: namespace.filter(|namespace| !namespace.is_empty() && namespace != "-")?,
        pod,
        containers: Vec::new(),
    })
}

pub(super) fn selected_log_container(
    dropdown: &gtk::DropDown,
    target: &PodLogTarget,
) -> Option<String> {
    target.containers.get(dropdown.selected() as usize).cloned()
}

pub(super) fn default_log_container_index(pod: &str, containers: &[String]) -> usize {
    containers
        .iter()
        .position(|container| container == pod)
        .unwrap_or(0)
}

pub(super) fn custom_namespace_initial_text(selected_namespace: &str) -> &str {
    if selected_namespace == "all" {
        ""
    } else {
        selected_namespace
    }
}

pub(super) fn with_all_namespace(mut namespaces: Vec<String>) -> Vec<String> {
    namespaces.retain(|namespace| namespace != "all");
    namespaces.sort();
    namespaces.dedup();
    namespaces.insert(0, String::from("all"));
    namespaces
}

pub(super) fn select_default_resource(resources: &[ResourceKind]) -> Option<usize> {
    resources
        .iter()
        .position(|resource| resource.kind == "Pod" && resource.group.is_empty())
        .or_else(|| resources.first().map(|_| 0))
}

#[cfg(test)]
mod tests {
    use super::forbidden_summary;

    #[test]
    fn forbidden_summary_extracts_just_the_resource() {
        let message = "ApiError: namespaces is forbidden: User \"system:serviceaccount:ns:sa\" \
            cannot list resource \"namespaces\" in API group \"\" at the cluster scope";

        assert_eq!(
            forbidden_summary(message).as_deref(),
            Some("namespaces is forbidden")
        );
    }

    #[test]
    fn forbidden_summary_ignores_unrelated_messages() {
        assert_eq!(forbidden_summary("connection refused"), None);
    }
}
