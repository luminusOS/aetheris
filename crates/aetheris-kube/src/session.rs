use kube::Client;

#[derive(Clone)]
pub struct KubeSession {
    pub(crate) context: String,
    pub(crate) client: Client,
    /// The API server URL this session points at, kept to recognize
    /// provider-specific proxies (e.g. Rancher's `…/k8s/clusters/<id>`).
    pub(crate) server: String,
}

impl KubeSession {
    pub fn context(&self) -> &str {
        &self.context
    }
}
