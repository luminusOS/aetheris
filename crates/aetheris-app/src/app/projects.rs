use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStore {
    pub(super) projects: Vec<Project>,
    pub(super) selected_project: Option<String>,
    #[serde(default)]
    pub(super) last_namespaces_by_context: Vec<ContextNamespaceSelection>,
    #[serde(default = "default_object_columns")]
    pub(super) visible_object_columns: Vec<ObjectColumn>,
    #[serde(default)]
    pub(super) object_name_width: Option<i32>,
    #[serde(default)]
    pub(super) object_column_widths: Vec<ObjectColumnWidth>,
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
pub(super) enum ResourceSection {
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
    Unavailable,
    Running,
    Pending,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObjectColumn {
    Namespace,
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

impl ObjectColumn {
    pub(super) const ALL: [Self; 6] = [
        Self::Namespace,
        Self::Status,
        Self::Cpu,
        Self::Memory,
        Self::Api,
        Self::Age,
    ];

    pub(super) fn label(self) -> String {
        match self {
            Self::Namespace => tr("Namespace"),
            Self::Status => tr("Status"),
            Self::Cpu => tr("CPU"),
            Self::Memory => tr("Memory"),
            Self::Api => tr("API"),
            Self::Age => tr("Age"),
        }
    }

    pub(super) fn default_width(self) -> i32 {
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
    pub(super) const ALL: [Self; 5] = [
        Self::Ready,
        Self::Unavailable,
        Self::Running,
        Self::Pending,
        Self::Failed,
    ];

    pub(super) fn label(self) -> String {
        match self {
            Self::Ready => tr("Ready"),
            Self::Unavailable => tr("Unavailable"),
            Self::Running => tr("Running"),
            Self::Pending => tr("Pending"),
            Self::Failed => tr("Failed"),
        }
    }

    pub(super) fn matches(self, status: &str) -> bool {
        status
            .split_whitespace()
            .next()
            .is_some_and(|part| part.eq_ignore_ascii_case(self.keyword()))
    }

    pub(super) fn matches_any(status: &str, filters: &BTreeSet<Self>) -> bool {
        if filters.len() == Self::ALL.len() {
            return true;
        }
        filters.iter().any(|filter| filter.matches(status))
    }

    pub(super) fn default_filters() -> BTreeSet<Self> {
        Self::ALL.into_iter().collect()
    }

    pub(super) fn keyword(self) -> &'static str {
        match self {
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

    pub(super) fn label(self) -> String {
        match self {
            Self::Workloads => tr("Workloads"),
            Self::Network => tr("Network"),
            Self::Storage => tr("Storage"),
            Self::Configuration => tr("Configuration"),
            Self::Access => tr("Access"),
            Self::Cluster => tr("Cluster"),
            Self::Custom => tr("Custom"),
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
                custom_namespaces_by_context: Vec::new(),
            }],
            selected_project: Some(String::from(DEFAULT_PROJECT_NAME)),
            last_namespaces_by_context: Vec::new(),
            visible_object_columns: default_object_columns(),
            object_name_width: None,
            object_column_widths: Vec::new(),
        }
    }
}

impl ProjectStore {
    pub(super) fn load(contexts: &[ContextInfo]) -> Self {
        let mut store = Self::read_from_disk().unwrap_or_default();
        store.normalize_object_columns();
        store.normalize_object_column_widths();
        store.normalize_object_name_width();
        store.normalize_contexts(contexts);
        store.normalize_last_namespaces(contexts);
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

    pub(super) fn object_column_width(&self, column: ObjectColumn) -> i32 {
        self.object_column_widths
            .iter()
            .find(|entry| entry.column == column)
            .map(|entry| entry.width)
            .unwrap_or_else(|| column.default_width())
            .clamp(OBJECT_COLUMN_MIN_WIDTH, OBJECT_COLUMN_MAX_WIDTH)
    }

    pub(super) fn object_name_width(&self) -> i32 {
        self.object_name_width
            .unwrap_or(OBJECT_NAME_WIDTH)
            .clamp(OBJECT_NAME_MIN_WIDTH, OBJECT_NAME_MAX_WIDTH)
    }

    pub(super) fn set_object_table_column_width(
        &mut self,
        column: ObjectTableColumn,
        width: i32,
    ) -> bool {
        match column {
            ObjectTableColumn::Name => self.set_object_name_width(width),
            ObjectTableColumn::Data(column) => self.set_object_column_width(column, width),
        }
    }

    pub(super) fn last_namespace_for_context(&self, context: Option<&str>) -> Option<&str> {
        let context = context?;
        self.last_namespaces_by_context
            .iter()
            .find(|entry| entry.context == context)
            .map(|entry| entry.namespace.as_str())
    }

    pub(super) fn set_last_namespace_for_context(
        &mut self,
        context: &str,
        namespace: &str,
    ) -> bool {
        let context = context.trim();
        let namespace = namespace.trim();
        if context.is_empty() || namespace.is_empty() {
            return false;
        }

        if let Some(entry) = self
            .last_namespaces_by_context
            .iter_mut()
            .find(|entry| entry.context == context)
        {
            if entry.namespace == namespace {
                return false;
            }
            entry.namespace = namespace.to_owned();
            return true;
        }

        self.last_namespaces_by_context
            .push(ContextNamespaceSelection {
                context: context.to_owned(),
                namespace: namespace.to_owned(),
            });
        self.normalize_last_namespaces(&[]);
        true
    }

    fn set_object_name_width(&mut self, width: i32) -> bool {
        let width = width.clamp(OBJECT_NAME_MIN_WIDTH, OBJECT_NAME_MAX_WIDTH);
        let next = (width != OBJECT_NAME_WIDTH).then_some(width);
        if self.object_name_width == next {
            return false;
        }
        self.object_name_width = next;
        true
    }

    pub(super) fn set_object_column_width(&mut self, column: ObjectColumn, width: i32) -> bool {
        let width = width.clamp(OBJECT_COLUMN_MIN_WIDTH, OBJECT_COLUMN_MAX_WIDTH);
        if width == column.default_width() {
            let previous_len = self.object_column_widths.len();
            self.object_column_widths
                .retain(|entry| entry.column != column);
            return previous_len != self.object_column_widths.len();
        }

        if let Some(entry) = self
            .object_column_widths
            .iter_mut()
            .find(|entry| entry.column == column)
        {
            if entry.width == width {
                return false;
            }
            entry.width = width;
            return true;
        }

        self.object_column_widths
            .push(ObjectColumnWidth { column, width });
        self.normalize_object_column_widths();
        true
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

    fn normalize_object_column_widths(&mut self) {
        self.object_column_widths
            .retain(|entry| ObjectColumn::ALL.contains(&entry.column));
        for entry in &mut self.object_column_widths {
            entry.width = entry
                .width
                .clamp(OBJECT_COLUMN_MIN_WIDTH, OBJECT_COLUMN_MAX_WIDTH);
        }
        self.object_column_widths.sort_by_key(|entry| {
            ObjectColumn::ALL
                .iter()
                .position(|candidate| *candidate == entry.column)
                .unwrap_or(usize::MAX)
        });
        self.object_column_widths.dedup_by_key(|entry| entry.column);
        self.object_column_widths
            .retain(|entry| entry.width != entry.column.default_width());
    }

    fn normalize_object_name_width(&mut self) {
        self.object_name_width = self
            .object_name_width
            .map(|width| width.clamp(OBJECT_NAME_MIN_WIDTH, OBJECT_NAME_MAX_WIDTH))
            .filter(|width| *width != OBJECT_NAME_WIDTH);
    }

    pub(super) fn save(&self) -> Result<(), String> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                tr_format(
                    "Unable to create project config directory: {error}",
                    &[("{error}", error.to_string())],
                )
            })?;
        }
        let data = serde_json::to_string_pretty(self).map_err(|error| {
            tr_format(
                "Unable to encode projects: {error}",
                &[("{error}", error.to_string())],
            )
        })?;
        fs::write(&path, data).map_err(|error| {
            tr_format(
                "Unable to write project config {path}: {error}",
                &[
                    ("{path}", path.display().to_string()),
                    ("{error}", error.to_string()),
                ],
            )
        })
    }

    pub(super) fn read_from_disk() -> Option<Self> {
        let path = Self::path()?;
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub(super) fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|path| path.join("aetheris").join("projects.json"))
    }

    pub(super) fn normalize_contexts(&mut self, contexts: &[ContextInfo]) {
        if self.projects.is_empty() {
            self.projects.push(Project {
                name: String::from(DEFAULT_PROJECT_NAME),
                contexts: Vec::new(),
                custom_namespaces_by_context: Vec::new(),
            });
            self.selected_project = Some(String::from(DEFAULT_PROJECT_NAME));
        }

        // The kubeconfig can be changed by kubectl/oc outside Aetheris. Keep
        // only clusters explicitly saved in projects.json; use the live
        // kubeconfig here only to prune deleted/renamed entries. An empty
        // live list means the kubeconfig is missing or unreadable, not that
        // every cluster was deleted — pruning against it (and persisting the
        // result below) would wipe the user's saved clusters on a single bad
        // startup, so leave the store alone until contexts load again.
        if !contexts.is_empty() {
            let live_names: BTreeSet<&str> = contexts
                .iter()
                .map(|context| context.name.as_str())
                .collect();
            for project in &mut self.projects {
                project
                    .contexts
                    .retain(|name| live_names.contains(name.as_str()));
                let project_contexts = project
                    .contexts
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                project
                    .custom_namespaces_by_context
                    .retain(|entry| project_contexts.contains(entry.context.as_str()));
            }
        }

        let selected_exists = self
            .selected_project
            .as_ref()
            .is_some_and(|selected| self.has_project(selected));
        if !selected_exists {
            self.selected_project = self.projects.first().map(|project| project.name.clone());
        }

        for project in &mut self.projects {
            project.contexts.sort();
            project.contexts.dedup();
            project.normalize_custom_namespaces();
        }
    }

    fn normalize_last_namespaces(&mut self, contexts: &[ContextInfo]) {
        if !contexts.is_empty() {
            let live_names: BTreeSet<&str> = contexts
                .iter()
                .map(|context| context.name.as_str())
                .collect();
            self.last_namespaces_by_context
                .retain(|entry| live_names.contains(entry.context.as_str()));
        }

        for entry in &mut self.last_namespaces_by_context {
            entry.context = entry.context.trim().to_owned();
            entry.namespace = entry.namespace.trim().to_owned();
        }
        self.last_namespaces_by_context
            .retain(|entry| !entry.context.is_empty() && !entry.namespace.is_empty());
        self.last_namespaces_by_context
            .sort_by(|left, right| left.context.cmp(&right.context));
        self.last_namespaces_by_context
            .dedup_by(|left, right| left.context == right.context);
    }

    pub(super) fn add_contexts_to_selected_project<I>(&mut self, contexts: I)
    where
        I: IntoIterator<Item = String>,
    {
        let Some(project) = self.selected_project_mut() else {
            return;
        };
        project.contexts.extend(
            contexts
                .into_iter()
                .map(|context| context.trim().to_owned())
                .filter(|context| !context.is_empty()),
        );
        project.contexts.sort();
        project.contexts.dedup();
    }

    pub(super) fn remove_context_from_selected_project(&mut self, context: &str) {
        let Some(project) = self.selected_project_mut() else {
            return;
        };
        project.contexts.retain(|candidate| candidate != context);
        project
            .custom_namespaces_by_context
            .retain(|entry| entry.context != context);
    }

    pub(super) fn selected_project_mut(&mut self) -> Option<&mut Project> {
        let selected = self.selected_project.clone()?;
        self.projects
            .iter_mut()
            .find(|project| project.name == selected)
    }
}

impl Project {
    pub(super) fn custom_namespaces_for_context(&self, context: Option<&str>) -> Vec<String> {
        let Some(context) = context else {
            return Vec::new();
        };
        self.custom_namespaces_by_context
            .iter()
            .find(|entry| entry.context == context)
            .map(|entry| entry.namespaces.clone())
            .unwrap_or_default()
    }

    pub(super) fn has_custom_namespace(&self, context: Option<&str>, namespace: &str) -> bool {
        self.custom_namespaces_for_context(context)
            .iter()
            .any(|known| known == namespace)
    }

    pub(super) fn add_custom_namespace(&mut self, context: &str, namespace: &str) -> bool {
        if context.is_empty() || namespace.is_empty() {
            return false;
        }

        if let Some(entry) = self
            .custom_namespaces_by_context
            .iter_mut()
            .find(|entry| entry.context == context)
        {
            if entry.namespaces.iter().any(|known| known == namespace) {
                return false;
            }
            entry.namespaces.push(namespace.to_owned());
            entry.namespaces.sort();
            entry.namespaces.dedup();
            return true;
        }

        self.custom_namespaces_by_context.push(ContextNamespaces {
            context: context.to_owned(),
            namespaces: vec![namespace.to_owned()],
        });
        self.normalize_custom_namespaces();
        true
    }

    pub(super) fn remove_custom_namespace(&mut self, context: &str, namespace: &str) -> bool {
        let Some(entry) = self
            .custom_namespaces_by_context
            .iter_mut()
            .find(|entry| entry.context == context)
        else {
            return false;
        };

        let before = entry.namespaces.len();
        entry.namespaces.retain(|known| known != namespace);
        let removed = entry.namespaces.len() != before;

        self.custom_namespaces_by_context
            .retain(|entry| !entry.namespaces.is_empty());

        removed
    }

    pub(super) fn rename_custom_namespace(&mut self, context: &str, old: &str, new: &str) -> bool {
        let new = new.trim();
        if new.is_empty() || new == old {
            return false;
        }

        let Some(entry) = self
            .custom_namespaces_by_context
            .iter_mut()
            .find(|entry| entry.context == context)
        else {
            return false;
        };

        if entry.namespaces.iter().any(|known| known == new) {
            return false;
        }

        let Some(slot) = entry.namespaces.iter_mut().find(|known| *known == old) else {
            return false;
        };
        *slot = new.to_owned();
        entry.namespaces.sort();
        entry.namespaces.dedup();

        true
    }

    fn normalize_custom_namespaces(&mut self) {
        for entry in &mut self.custom_namespaces_by_context {
            entry.context = entry.context.trim().to_owned();
            entry.namespaces.retain(|namespace| {
                let namespace = namespace.trim();
                !namespace.is_empty() && namespace != "all"
            });
            for namespace in &mut entry.namespaces {
                *namespace = namespace.trim().to_owned();
            }
            entry.namespaces.sort();
            entry.namespaces.dedup();
        }
        self.custom_namespaces_by_context
            .retain(|entry| !entry.context.is_empty() && !entry.namespaces.is_empty());
        self.custom_namespaces_by_context
            .sort_by(|left, right| left.context.cmp(&right.context));
        self.custom_namespaces_by_context.dedup_by(|left, right| {
            if left.context == right.context {
                left.namespaces.extend(right.namespaces.clone());
                left.namespaces.sort();
                left.namespaces.dedup();
                true
            } else {
                false
            }
        });
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
            insecure_skip_tls_verify: false,
        }
    }

    #[test]
    fn normalize_contexts_does_not_import_external_contexts() {
        let mut store = ProjectStore {
            projects: vec![Project {
                name: String::from("Work"),
                contexts: vec![String::from("local")],
                custom_namespaces_by_context: Vec::new(),
            }],
            selected_project: Some(String::from("Work")),
            last_namespaces_by_context: Vec::new(),
            visible_object_columns: default_object_columns(),
            object_name_width: None,
            object_column_widths: Vec::new(),
        };

        store.normalize_contexts(&[context("local"), context("external")]);

        let project = store.selected_project().unwrap();
        assert_eq!(project.contexts, vec![String::from("local")]);
    }

    #[test]
    fn normalize_contexts_keeps_saved_contexts_when_live_list_is_empty() {
        let mut store = ProjectStore {
            projects: vec![Project {
                name: String::from("Work"),
                contexts: vec![String::from("prod"), String::from("stage")],
                custom_namespaces_by_context: vec![ContextNamespaces {
                    context: String::from("prod"),
                    namespaces: vec![String::from("billing")],
                }],
            }],
            selected_project: Some(String::from("Work")),
            last_namespaces_by_context: vec![ContextNamespaceSelection {
                context: String::from("prod"),
                namespace: String::from("team-a"),
            }],
            visible_object_columns: default_object_columns(),
            object_name_width: None,
            object_column_widths: Vec::new(),
        };

        store.normalize_contexts(&[]);

        let project = store.selected_project().unwrap();
        assert_eq!(
            project.contexts,
            vec![String::from("prod"), String::from("stage")]
        );
        assert_eq!(
            project.custom_namespaces_for_context(Some("prod")),
            vec![String::from("billing")]
        );
        assert_eq!(
            store.last_namespace_for_context(Some("prod")),
            Some("team-a")
        );
    }

    #[test]
    fn last_namespace_is_stored_by_context() {
        let mut store = ProjectStore::default();

        assert!(store.set_last_namespace_for_context("prod", "team-a"));
        assert_eq!(
            store.last_namespace_for_context(Some("prod")),
            Some("team-a")
        );
        assert!(!store.set_last_namespace_for_context("prod", "team-a"));
        assert!(store.set_last_namespace_for_context("prod", "team-b"));
        assert_eq!(
            store.last_namespace_for_context(Some("prod")),
            Some("team-b")
        );
        assert_eq!(store.last_namespaces_by_context.len(), 1);
    }

    #[test]
    fn normalize_contexts_prunes_last_namespaces_for_deleted_contexts() {
        let mut store = ProjectStore {
            projects: vec![Project {
                name: String::from("Work"),
                contexts: vec![String::from("prod")],
                custom_namespaces_by_context: Vec::new(),
            }],
            selected_project: Some(String::from("Work")),
            last_namespaces_by_context: vec![
                ContextNamespaceSelection {
                    context: String::from("prod"),
                    namespace: String::from("team-a"),
                },
                ContextNamespaceSelection {
                    context: String::from("external"),
                    namespace: String::from("team-b"),
                },
            ],
            visible_object_columns: default_object_columns(),
            object_name_width: None,
            object_column_widths: Vec::new(),
        };

        store.normalize_last_namespaces(&[context("prod")]);

        assert_eq!(
            store.last_namespace_for_context(Some("prod")),
            Some("team-a")
        );
        assert_eq!(store.last_namespace_for_context(Some("external")), None);
    }

    #[test]
    fn custom_namespaces_are_scoped_by_context() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod"), String::from("stage")],
            custom_namespaces_by_context: Vec::new(),
        };

        assert!(project.add_custom_namespace("prod", "billing"));

        assert_eq!(
            project.custom_namespaces_for_context(Some("prod")),
            vec![String::from("billing")]
        );
        assert!(project
            .custom_namespaces_for_context(Some("stage"))
            .is_empty());
        assert!(!project.has_custom_namespace(Some("stage"), "billing"));
    }

    #[test]
    fn remove_custom_namespace_drops_empty_context_entry() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod")],
            custom_namespaces_by_context: Vec::new(),
        };
        project.add_custom_namespace("prod", "billing");

        assert!(project.remove_custom_namespace("prod", "billing"));
        assert!(project
            .custom_namespaces_for_context(Some("prod"))
            .is_empty());
        assert!(project.custom_namespaces_by_context.is_empty());
    }

    #[test]
    fn remove_custom_namespace_returns_false_when_not_found() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod")],
            custom_namespaces_by_context: Vec::new(),
        };
        project.add_custom_namespace("prod", "billing");

        assert!(!project.remove_custom_namespace("prod", "not-there"));
        assert!(!project.remove_custom_namespace("other-context", "billing"));
        assert_eq!(
            project.custom_namespaces_for_context(Some("prod")),
            vec![String::from("billing")]
        );
    }

    #[test]
    fn rename_custom_namespace_replaces_entry_in_place() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod")],
            custom_namespaces_by_context: Vec::new(),
        };
        project.add_custom_namespace("prod", "billing");

        assert!(project.rename_custom_namespace("prod", "billing", "payments"));
        assert_eq!(
            project.custom_namespaces_for_context(Some("prod")),
            vec![String::from("payments")]
        );
        assert!(!project.has_custom_namespace(Some("prod"), "billing"));
    }

    #[test]
    fn rename_custom_namespace_no_ops_when_target_name_already_exists() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod")],
            custom_namespaces_by_context: Vec::new(),
        };
        project.add_custom_namespace("prod", "billing");
        project.add_custom_namespace("prod", "payments");

        assert!(!project.rename_custom_namespace("prod", "billing", "payments"));
        assert_eq!(
            project.custom_namespaces_for_context(Some("prod")),
            vec![String::from("billing"), String::from("payments")]
        );
    }

    #[test]
    fn rename_custom_namespace_returns_false_when_source_missing() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod")],
            custom_namespaces_by_context: Vec::new(),
        };

        assert!(!project.rename_custom_namespace("prod", "billing", "payments"));
    }
}
