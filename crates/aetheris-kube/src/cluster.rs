use anyhow::{Context as AnyhowContext, Result};

use crate::{ClusterSummary, KubeSession};

impl KubeSession {
    /// A best-effort snapshot for the Clusters list page. Only the initial
    /// connectivity check (the version endpoint) is fatal; node listing,
    /// pod counting and metrics-server queries each degrade to `None`
    /// independently so a cluster without metrics-server (or with a slow
    /// node list) still reports as reachable.
    /// Only the version endpoint is queried: it's a discovery call granted
    /// to `system:authenticated` in every stock RBAC setup, unlike listing
    /// Nodes or cluster-wide Pods, which regularly aren't (the same reason
    /// listing all Namespaces can fail for a scoped kubeconfig context even
    /// though Rancher's own UI can see them).
    pub async fn cluster_summary(&self) -> Result<ClusterSummary> {
        let version_info = self
            .client
            .apiserver_version()
            .await
            .with_context(|| format!("Could not reach context {}.", self.context))?;
        let provider = detect_provider(&version_info.git_version);

        Ok(ClusterSummary {
            version: Some(version_info.git_version),
            provider,
        })
    }
}

/// Best-effort distribution guess from the server version string alone
/// (e.g. `-eks-`, `+k3s`). Not authoritative — Kubernetes has no generic
/// "who built this cluster" API — but unlike node-label based detection,
/// this needs no extra RBAC permissions beyond the version endpoint.
fn detect_provider(version: &str) -> Option<String> {
    let lower = version.to_ascii_lowercase();
    if lower.contains("-eks-") {
        return Some(String::from("EKS"));
    }
    if lower.contains("-gke.") || lower.contains("-gke-") {
        return Some(String::from("GKE"));
    }
    if lower.contains("+k3s") {
        return Some(String::from("k3s"));
    }
    if lower.contains("+rke2") {
        return Some(String::from("RKE2"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::detect_provider;

    #[test]
    fn detect_provider_reads_eks_version_suffix() {
        assert_eq!(
            detect_provider("v1.34.8-eks-0247562"),
            Some(String::from("EKS"))
        );
    }

    #[test]
    fn detect_provider_reads_k3s_version_suffix() {
        assert_eq!(detect_provider("v1.27.3+k3s1"), Some(String::from("k3s")));
    }

    #[test]
    fn detect_provider_reports_none_when_unrecognized() {
        assert_eq!(detect_provider("v1.28.0"), None);
    }
}
