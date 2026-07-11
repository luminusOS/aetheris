use super::*;

mod column;
mod favorite;
mod project;
mod resource_section;
mod status_filter;
mod store;

use column::default_object_columns;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStore {
    pub(super) projects: Vec<Project>,
    pub(super) selected_project: Option<String>,
    #[serde(default)]
    pub(super) last_namespaces_by_context: Vec<ContextNamespaceSelection>,
    #[serde(default)]
    pub(super) object_column_schema_version: u32,
    #[serde(default = "default_object_columns")]
    pub(super) visible_object_columns: Vec<ObjectColumn>,
    #[serde(default)]
    pub(super) object_name_width: Option<i32>,
    #[serde(default)]
    pub(super) object_column_widths: Vec<ObjectColumnWidth>,
    #[serde(default)]
    pub(super) favorite_objects: Vec<ObjectFavorite>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Project {
    pub(super) name: String,
    pub(super) contexts: Vec<String>,
    #[serde(default)]
    pub(super) custom_namespaces_by_context: Vec<ContextNamespaces>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ContextNamespaces {
    pub(super) context: String,
    pub(super) namespaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ContextNamespaceSelection {
    pub(super) context: String,
    pub(super) namespace: String,
}

#[derive(Debug, Clone)]
pub(super) struct DetailTarget {
    pub(super) context: String,
    pub(super) resource: ResourceKind,
    pub(super) namespace: Option<String>,
    pub(super) name: String,
}

#[derive(Debug, Clone)]
pub(super) struct PodLogTarget {
    pub(super) context: String,
    pub(super) namespace: String,
    pub(super) pod: String,
    pub(super) containers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceSection {
    Workloads,
    Network,
    Storage,
    Configuration,
    Access,
    Cluster,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum StatusFilter {
    Ready,
    Available,
    Unavailable,
    Running,
    Pending,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObjectColumn {
    Image,
    Namespace,
    Target,
    Selector,
    IngressClass,
    Status,
    Cpu,
    Memory,
    Api,
    Age,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectTableColumn {
    Name,
    Data(ObjectColumn),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ObjectColumnWidth {
    column: ObjectColumn,
    width: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ObjectFavorite {
    pub(super) context: String,
    group: String,
    version: String,
    api_version: String,
    kind: String,
    plural: String,
    namespace: Option<String>,
    pub(super) name: String,
}
