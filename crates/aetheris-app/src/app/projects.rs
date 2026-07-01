use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStore {
    pub(super) projects: Vec<Project>,
    pub(super) selected_project: Option<String>,
    #[serde(default = "default_object_columns")]
    pub(super) visible_object_columns: Vec<ObjectColumn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Project {
    pub(super) name: String,
    pub(super) contexts: Vec<String>,
    #[serde(default)]
    pub(super) custom_namespaces: Vec<String>,
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
pub(super) enum ResourceSection {
    Workloads,
    Network,
    Storage,
    Configuration,
    Access,
    Cluster,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StatusFilter {
    All,
    Ready,
    Unavailable,
    Running,
    Pending,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ObjectColumn {
    Namespace,
    Status,
    Cpu,
    Memory,
    Api,
    Age,
}

impl ObjectColumn {
    pub(super) const ALL: [Self; 6] = [
        Self::Namespace,
        Self::Status,
        Self::Cpu,
        Self::Memory,
        Self::Api,
        Self::Age,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Namespace => "Namespace",
            Self::Status => "Status",
            Self::Cpu => "CPU",
            Self::Memory => "Memory",
            Self::Api => "API",
            Self::Age => "Age",
        }
    }

    pub(super) fn width(self) -> i32 {
        match self {
            Self::Namespace => OBJECT_NAMESPACE_WIDTH,
            Self::Status => OBJECT_STATUS_WIDTH,
            Self::Cpu | Self::Memory => OBJECT_METRIC_WIDTH,
            Self::Api => OBJECT_API_WIDTH,
            Self::Age => OBJECT_AGE_WIDTH,
        }
    }
}

pub(super) fn default_object_columns() -> Vec<ObjectColumn> {
    ObjectColumn::ALL.to_vec()
}

impl StatusFilter {
    pub(super) const ALL: [Self; 6] = [
        Self::All,
        Self::Ready,
        Self::Unavailable,
        Self::Running,
        Self::Pending,
        Self::Failed,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "Any status",
            Self::Ready => "Ready",
            Self::Unavailable => "Unavailable",
            Self::Running => "Running",
            Self::Pending => "Pending",
            Self::Failed => "Failed",
        }
    }

    pub(super) fn matches(self, status: &str) -> bool {
        match self {
            Self::All => true,
            filter => status
                .split_whitespace()
                .next()
                .is_some_and(|part| part.eq_ignore_ascii_case(filter.keyword())),
        }
    }

    pub(super) fn keyword(self) -> &'static str {
        match self {
            Self::All => "",
            Self::Ready => "Ready",
            Self::Unavailable => "Unavailable",
            Self::Running => "Running",
            Self::Pending => "Pending",
            Self::Failed => "Failed",
        }
    }
}

impl ResourceSection {
    pub(super) const ALL: [Self; 7] = [
        Self::Workloads,
        Self::Network,
        Self::Storage,
        Self::Configuration,
        Self::Access,
        Self::Cluster,
        Self::Custom,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Workloads => "Workloads",
            Self::Network => "Network",
            Self::Storage => "Storage",
            Self::Configuration => "Configuration",
            Self::Access => "Access",
            Self::Cluster => "Cluster",
            Self::Custom => "Custom",
        }
    }

    pub(super) fn icon_name(self) -> &'static str {
        match self {
            Self::Workloads => "grid-large-symbolic",
            Self::Network => "network-transmit-receive-symbolic",
            Self::Storage => "harddisk-symbolic",
            Self::Configuration => "rich-text-symbolic",
            Self::Access => "key-symbolic",
            Self::Cluster => "network-server-symbolic",
            Self::Custom => "puzzle-piece-symbolic",
        }
    }

    pub(super) fn fallback_icon_name(self) -> &'static str {
        match self {
            Self::Workloads => "applications-system-symbolic",
            Self::Network => "network-workgroup-symbolic",
            Self::Storage => "drive-harddisk-symbolic",
            Self::Configuration => "preferences-system-symbolic",
            Self::Access => "changes-prevent-symbolic",
            Self::Cluster => "network-server-symbolic",
            Self::Custom => "application-x-addon-symbolic",
        }
    }

    pub(super) fn for_resource(resource: &ResourceKind) -> Self {
        Self::ALL
            .iter()
            .copied()
            .find(|section| *section != Self::Custom && section.matches(resource))
            .unwrap_or(Self::Custom)
    }

    pub(super) fn matches(self, resource: &ResourceKind) -> bool {
        match self {
            Self::Workloads => is_workload_resource(resource),
            Self::Network => is_network_resource(resource),
            Self::Storage => is_storage_resource(resource),
            Self::Configuration => is_configuration_resource(resource),
            Self::Access => is_access_resource(resource),
            Self::Cluster => is_cluster_resource(resource),
            Self::Custom => !Self::ALL
                .iter()
                .copied()
                .filter(|section| *section != Self::Custom)
                .any(|section| section.matches(resource)),
        }
    }
}

impl Default for ProjectStore {
    fn default() -> Self {
        Self {
            projects: vec![Project {
                name: String::from(DEFAULT_PROJECT_NAME),
                contexts: Vec::new(),
                custom_namespaces: Vec::new(),
            }],
            selected_project: Some(String::from(DEFAULT_PROJECT_NAME)),
            visible_object_columns: default_object_columns(),
        }
    }
}

impl ProjectStore {
    pub(super) fn load(contexts: &[ContextInfo]) -> Self {
        let mut store = Self::read_from_disk().unwrap_or_default();
        store.normalize_object_columns();
        store.ensure_contexts(contexts);
        if let Err(error) = store.save() {
            tracing::warn!("Unable to persist projects: {error}");
        }
        store
    }

    pub(super) fn has_project(&self, name: &str) -> bool {
        self.projects.iter().any(|project| project.name == name)
    }

    pub(super) fn selected_project(&self) -> Option<&Project> {
        let selected = self.selected_project.as_deref()?;
        self.projects
            .iter()
            .find(|project| project.name == selected)
    }

    pub(super) fn selected_project_name(&self) -> &str {
        self.selected_project
            .as_deref()
            .unwrap_or(DEFAULT_PROJECT_NAME)
    }

    pub(super) fn set_object_column_visible(&mut self, column: ObjectColumn, visible: bool) {
        if visible {
            if !self.visible_object_columns.contains(&column) {
                self.visible_object_columns.push(column);
            }
        } else {
            self.visible_object_columns
                .retain(|visible_column| *visible_column != column);
        }
        self.normalize_object_columns();
    }

    fn normalize_object_columns(&mut self) {
        self.visible_object_columns
            .retain(|column| ObjectColumn::ALL.contains(column));
        self.visible_object_columns.sort_by_key(|column| {
            ObjectColumn::ALL
                .iter()
                .position(|candidate| candidate == column)
                .unwrap_or(usize::MAX)
        });
        self.visible_object_columns.dedup();
    }

    pub(super) fn save(&self) -> Result<(), String> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Unable to create project config directory: {error}"))?;
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(|error| format!("Unable to encode projects: {error}"))?;
        fs::write(&path, data)
            .map_err(|error| format!("Unable to write project config {}: {error}", path.display()))
    }

    pub(super) fn read_from_disk() -> Option<Self> {
        let path = Self::path()?;
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub(super) fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|path| path.join("aetheris").join("projects.json"))
    }

    pub(super) fn ensure_contexts(&mut self, contexts: &[ContextInfo]) {
        if self.projects.is_empty() {
            self.projects.push(Project {
                name: String::from(DEFAULT_PROJECT_NAME),
                contexts: contexts
                    .iter()
                    .map(|context| context.name.clone())
                    .collect(),
                custom_namespaces: Vec::new(),
            });
            self.selected_project = Some(String::from(DEFAULT_PROJECT_NAME));
            return;
        }

        // Contexts that were renamed or deleted no longer appear in the
        // live kubeconfig; drop them so a rename doesn't leave the old name
        // behind as a phantom cluster in whichever project held it.
        let live_names: BTreeSet<&str> = contexts.iter().map(|context| context.name.as_str()).collect();
        for project in &mut self.projects {
            project
                .contexts
                .retain(|name| live_names.contains(name.as_str()));
        }

        let selected_exists = self
            .selected_project
            .as_ref()
            .is_some_and(|selected| self.has_project(selected));
        if !selected_exists {
            self.selected_project = self.projects.first().map(|project| project.name.clone());
        }

        let assigned = self
            .projects
            .iter()
            .flat_map(|project| project.contexts.iter().cloned())
            .collect::<BTreeSet<_>>();
        let unassigned = contexts
            .iter()
            .filter(|context| !assigned.contains(&context.name))
            .map(|context| context.name.clone())
            .collect::<Vec<_>>();
        let selected = self.selected_project_name().to_owned();
        if let Some(project) = self
            .projects
            .iter_mut()
            .find(|project| project.name == selected)
        {
            project.contexts.extend(unassigned);
        }

        for project in &mut self.projects {
            project.contexts.sort();
            project.contexts.dedup();
            project.custom_namespaces.sort();
            project.custom_namespaces.dedup();
        }
    }

    pub(super) fn selected_project_mut(&mut self) -> Option<&mut Project> {
        let selected = self.selected_project.clone()?;
        self.projects
            .iter_mut()
            .find(|project| project.name == selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(name: &str) -> ContextInfo {
        ContextInfo {
            name: name.to_owned(),
            cluster: name.to_owned(),
            server: String::from("https://example.com:6443"),
            host: String::from("example.com"),
            user: format!("{name}-user"),
            is_current: false,
        }
    }

    #[test]
    fn ensure_contexts_drops_renamed_cluster_instead_of_duplicating_it() {
        let mut store = ProjectStore {
            projects: vec![Project {
                name: String::from("Work"),
                contexts: vec![String::from("old-name")],
                custom_namespaces: Vec::new(),
            }],
            selected_project: Some(String::from("Work")),
            visible_object_columns: default_object_columns(),
        };

        // The cluster behind "old-name" was renamed to "new-name"; the
        // kubeconfig now only reports the new name.
        store.ensure_contexts(&[context("new-name")]);

        let project = store.selected_project().unwrap();
        assert_eq!(project.contexts, vec![String::from("new-name")]);
    }
}
