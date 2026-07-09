use super::*;

pub(super) fn text_buffer_text(buffer: &impl IsA<gtk::TextBuffer>) -> String {
    let buffer = buffer.as_ref();
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

// Mirrors where `kube`'s `Kubeconfig::read` looks: every path in
// $KUBECONFIG if set, otherwise ~/.kube/config. Used to tell "no kubeconfig
// yet" (normal first run) apart from "kubeconfig exists but won't load".
pub(super) fn kubeconfig_present() -> bool {
    if let Some(paths) = std::env::var_os("KUBECONFIG") {
        return std::env::split_paths(&paths).any(|path| path.exists());
    }
    dirs::home_dir().is_some_and(|home| home.join(".kube").join("config").exists())
}

pub(super) fn format_error(error: anyhow::Error) -> String {
    let mut chain = error.chain();
    let headline = chain
        .next()
        .map(ToString::to_string)
        .unwrap_or_else(|| tr("Unknown error"));

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

pub(super) fn terminal_error_message(error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if lower.contains("forbidden")
        || lower.contains("pods/exec")
        || (lower.contains("cannot create") && lower.contains("exec"))
    {
        return tr(
            "You do not have permission to open a terminal for this Pod. Your Kubernetes user needs create access to pods/exec in this namespace.",
        );
    }

    tr_format(
        "Terminal failed to start: {error}",
        &[("{error}", error.to_string())],
    )
}

/// Kubernetes RBAC denials spell out the exact user, resource, API group,
/// and scope, e.g. `namespaces is forbidden: User "..." cannot list
/// resource "namespaces" in API group "" at the cluster scope` — all noise
/// for a toast. Keep just "<resource> is forbidden".
fn forbidden_summary(message: &str) -> Option<String> {
    let words: Vec<&str> = message.split_whitespace().collect();
    let position = words
        .iter()
        .position(|word| word.starts_with("forbidden"))?;
    if position < 2 || words[position - 1] != "is" {
        return None;
    }
    Some(tr_format(
        "{resource} is forbidden",
        &[("{resource}", words[position - 2].to_string())],
    ))
}

pub(super) fn object_matches(object: &ObjectSummary, query: &str) -> bool {
    object.name.to_ascii_lowercase().contains(query)
        || object.namespace.to_ascii_lowercase().contains(query)
        || object.status.to_ascii_lowercase().contains(query)
        || object.api_version.to_ascii_lowercase().contains(query)
        || object
            .images
            .iter()
            .any(|image| image.to_ascii_lowercase().contains(query))
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
            ObjectColumn::Image => {
                resource.is_some_and(|resource| resource.group.is_empty() && resource.kind == "Pod")
            }
            ObjectColumn::Namespace => resource.map(ResourceKind::is_namespaced).unwrap_or(true),
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

pub(super) fn default_terminal_container(target: &PodLogTarget) -> Option<String> {
    target
        .containers
        .get(default_log_container_index(&target.pod, &target.containers))
        .cloned()
}

pub(super) fn default_log_container_index(pod: &str, containers: &[String]) -> usize {
    containers
        .iter()
        .position(|container| container == pod || pod.starts_with(&format!("{container}-")))
        .unwrap_or(0)
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

/// Parses a Kubernetes quantity string ("125m", "512Mi", "2") into a plain
/// number for sorting. Decimal (n/u/m/k/M/G/T/P/E) and binary (Ki..Ei)
/// suffixes are supported; `None` for blanks and anything unparsable, so
/// objects without a metrics sample sort together at one end.
pub(super) fn parse_quantity(raw: &str) -> Option<f64> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "-" {
        return None;
    }
    let split = raw
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(raw.len());
    let (number, suffix) = raw.split_at(split);
    let factor = match suffix {
        "" => 1.0,
        "n" => 1e-9,
        "u" => 1e-6,
        "m" => 1e-3,
        "k" | "K" => 1e3,
        "M" => 1e6,
        "G" => 1e9,
        "T" => 1e12,
        "P" => 1e15,
        "E" => 1e18,
        "Ki" => 1024f64,
        "Mi" => 1024f64.powi(2),
        "Gi" => 1024f64.powi(3),
        "Ti" => 1024f64.powi(4),
        "Pi" => 1024f64.powi(5),
        "Ei" => 1024f64.powi(6),
        _ => return None,
    };
    number.parse::<f64>().ok().map(|value| value * factor)
}

pub(super) fn shortened_image(image: &str) -> String {
    let mut image = image;
    for prefix in [
        "index.docker.io/library/",
        "index.docker.io/",
        "docker.io/library/",
        "docker.io/",
    ] {
        if let Some(stripped) = image.strip_prefix(prefix) {
            image = stripped;
            break;
        }
    }

    let mut shortened = image.strip_suffix(":latest").unwrap_or(image).to_owned();
    if let Some((prefix, digest)) = shortened.split_once("@sha256:")
        && digest.len() > 8
        && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        shortened = format!("{prefix}@sha256:{}…", &digest[..8]);
    }
    shortened
}

pub(super) fn pod_main_image(images: &[String]) -> Option<String> {
    const IGNORE_IMAGES: [&str; 3] = [
        "istio/proxy",
        "gcr.io/istio-release/proxy",
        "mirrored-istio-proxy",
    ];

    let shortened = images
        .iter()
        .map(|image| shortened_image(image))
        .collect::<Vec<_>>();
    shortened
        .iter()
        .find(|image| !IGNORE_IMAGES.iter().any(|ignore| image.contains(ignore)))
        .or_else(|| shortened.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::{forbidden_summary, offerable_columns_for, pod_main_image, shortened_image};
    use crate::app::projects::ObjectColumn;
    use aetheris_kube::{ResourceKind, ResourceScope};

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

    #[test]
    fn parse_quantity_handles_kubernetes_suffixes() {
        use super::parse_quantity;

        assert_eq!(parse_quantity("2"), Some(2.0));
        assert_eq!(parse_quantity("125m"), Some(0.125));
        assert_eq!(parse_quantity("1500n"), Some(1.5e-6));
        assert_eq!(parse_quantity("512Mi"), Some(512.0 * 1024.0 * 1024.0));
        assert_eq!(parse_quantity("1Gi"), Some(1024f64.powi(3)));
        assert_eq!(parse_quantity("2k"), Some(2000.0));
        assert_eq!(parse_quantity(""), None);
        assert_eq!(parse_quantity("-"), None);
        assert_eq!(parse_quantity("weird"), None);
    }

    #[test]
    fn shortened_image_matches_rancher_display_rules() {
        assert_eq!(shortened_image("docker.io/library/nginx:latest"), "nginx");
        assert_eq!(
            shortened_image("index.docker.io/library/ubuntu:latest"),
            "ubuntu"
        );
        assert_eq!(
            shortened_image("example.com/my/app:v1.2.3"),
            "example.com/my/app:v1.2.3"
        );
        assert_eq!(
            shortened_image("myrepo/myimage@sha256:abcdef1234567890abcd"),
            "myrepo/myimage@sha256:abcdef12…"
        );
    }

    #[test]
    fn pod_main_image_skips_istio_sidecar_images() {
        let images = vec![
            String::from("docker.io/istio/proxyv2:latest"),
            String::from("docker.io/library/nginx:latest"),
        ];

        assert_eq!(pod_main_image(&images).as_deref(), Some("nginx"));
    }

    #[test]
    fn offerable_columns_hide_namespace_for_cluster_scoped_resources() {
        let node = ResourceKind {
            group: String::new(),
            version: String::from("v1"),
            api_version: String::from("v1"),
            kind: String::from("Node"),
            plural: String::from("nodes"),
            scope: ResourceScope::Cluster,
        };

        let columns = offerable_columns_for(Some(&node));

        assert!(!columns.contains(&ObjectColumn::Namespace));
        assert!(columns.contains(&ObjectColumn::Cpu));
        assert!(columns.contains(&ObjectColumn::Memory));
    }

    #[test]
    fn offerable_columns_show_image_only_for_pods() {
        let pod = ResourceKind {
            group: String::new(),
            version: String::from("v1"),
            api_version: String::from("v1"),
            kind: String::from("Pod"),
            plural: String::from("pods"),
            scope: ResourceScope::Namespaced,
        };
        let deployment = ResourceKind {
            group: String::from("apps"),
            version: String::from("v1"),
            api_version: String::from("apps/v1"),
            kind: String::from("Deployment"),
            plural: String::from("deployments"),
            scope: ResourceScope::Namespaced,
        };

        assert!(offerable_columns_for(Some(&pod)).contains(&ObjectColumn::Image));
        assert!(!offerable_columns_for(Some(&deployment)).contains(&ObjectColumn::Image));
    }
}
