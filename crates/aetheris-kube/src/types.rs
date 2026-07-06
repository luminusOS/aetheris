use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextInfo {
    pub name: String,
    pub cluster: String,
    pub server: String,
    pub host: String,
    pub user: String,
    pub is_current: bool,
    #[serde(default)]
    pub insecure_skip_tls_verify: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodSummary {
    pub name: String,
    pub namespace: String,
    pub phase: String,
    pub node: String,
    pub age: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceScope {
    Cluster,
    Namespaced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceKind {
    pub group: String,
    pub version: String,
    pub api_version: String,
    pub kind: String,
    pub plural: String,
    pub scope: ResourceScope,
}

impl ResourceKind {
    pub fn label(&self) -> String {
        if self.group.is_empty() {
            self.kind.clone()
        } else {
            format!("{} ({})", self.kind, self.group)
        }
    }

    pub fn is_namespaced(&self) -> bool {
        self.scope == ResourceScope::Namespaced
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSummary {
    pub name: String,
    pub namespace: String,
    pub status: String,
    /// The `(ready, desired)` counts backing `status`, when the resource
    /// kind exposes one (Deployments, StatefulSets, Jobs, ...).
    pub status_ratio: Option<(i64, i64)>,
    pub api_version: String,
    pub age: String,
    pub metrics: Option<ResourceUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectDetail {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub api_version: String,
    pub kind: String,
    pub age: String,
    pub metrics: Option<ResourceUsage>,
    pub container_metrics: Vec<ContainerUsage>,
    pub yaml: String,
    pub containers: Vec<String>,
    pub related_pods: Vec<ObjectSummary>,
    pub replicas: Option<i32>,
    pub node_unschedulable: Option<bool>,
    pub conditions: Vec<ObjectCondition>,
    pub events: Vec<ObjectEvent>,
    pub events_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectCondition {
    pub type_: String,
    pub status: String,
    pub reason: String,
    pub message: String,
    pub last_transition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectEvent {
    pub type_: String,
    pub reason: String,
    pub message: String,
    pub count: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu: String,
    pub memory: String,
    pub cpu_ratio: Option<ResourceRatio>,
    pub memory_ratio: Option<ResourceRatio>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRatio {
    /// Ratio in basis points: 10000 means 100%.
    pub basis_points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerUsage {
    pub name: String,
    pub cpu: String,
    pub memory: String,
}

/// Version/provider data shown on the Clusters list page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub version: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddClusterRequest {
    pub context_name: String,
    pub server: String,
    pub bearer_token: String,
    pub certificate_authority_data: Option<String>,
    pub insecure_skip_tls_verify: bool,
    /// The context's name before this edit, if this request renames an
    /// existing cluster. `None` when adding a brand-new cluster.
    pub original_context_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodLogRequest {
    pub namespace: String,
    pub pod: String,
    pub container: Option<String>,
    pub follow: bool,
    pub timestamps: bool,
    pub tail_lines: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodPortForwardRequest {
    pub namespace: String,
    pub pod: String,
    pub local_port: u16,
    pub remote_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodExecRequest {
    pub namespace: String,
    pub pod: String,
    pub container: Option<String>,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodExecEvent {
    Stdout(String),
    Stderr(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectWatchEvent {
    Restarted(Vec<ObjectSummary>),
    Applied(ObjectSummary),
    Deleted(ObjectSummary),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodPortForwardEvent {
    Ready { local_port: u16 },
    ConnectionOpened,
    ConnectionClosed,
}
