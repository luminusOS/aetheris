use kube::Client;

#[derive(Clone)]
pub struct KubeSession {
    pub(crate) context: String,
    pub(crate) client: Client,
}

impl KubeSession {
    pub fn context(&self) -> &str {
        &self.context
    }
}
