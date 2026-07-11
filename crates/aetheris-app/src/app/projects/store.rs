use super::*;

use super::column::default_object_columns;

const OBJECT_COLUMN_SCHEMA_VERSION: u32 = 3;

impl Default for ProjectStore {
    fn default() -> Self {
        Self::with_default_project()
    }
}

impl ProjectStore {
    fn empty() -> Self {
        Self {
            projects: Vec::new(),
            selected_project: None,
            last_namespaces_by_context: Vec::new(),
            object_column_schema_version: OBJECT_COLUMN_SCHEMA_VERSION,
            visible_object_columns: default_object_columns(),
            object_name_width: None,
            object_column_widths: Vec::new(),
            favorite_objects: Vec::new(),
        }
    }

    fn with_default_project() -> Self {
        Self {
            projects: vec![Project {
                name: String::from(DEFAULT_PROJECT_NAME),
                contexts: Vec::new(),
                custom_namespaces_by_context: Vec::new(),
            }],
            selected_project: Some(String::from(DEFAULT_PROJECT_NAME)),
            last_namespaces_by_context: Vec::new(),
            object_column_schema_version: OBJECT_COLUMN_SCHEMA_VERSION,
            visible_object_columns: default_object_columns(),
            object_name_width: None,
            object_column_widths: Vec::new(),
            favorite_objects: Vec::new(),
        }
    }

    pub(crate) fn load(contexts: &[ContextInfo]) -> Self {
        let Some(mut store) = Self::read_from_disk() else {
            return Self::empty();
        };
        store.migrate_object_columns();
        store.normalize_object_columns();
        store.normalize_object_column_widths();
        store.normalize_object_name_width();
        store.normalize_favorite_objects();
        store.normalize_contexts(contexts);
        store.normalize_last_namespaces(contexts);
        if let Err(error) = store.save() {
            tracing::warn!("Unable to persist projects: {error}");
        }
        store
    }

    pub(crate) fn has_project(&self, name: &str) -> bool {
        self.projects.iter().any(|project| project.name == name)
    }

    pub(crate) fn selected_project(&self) -> Option<&Project> {
        let selected = self.selected_project.as_deref()?;
        self.projects
            .iter()
            .find(|project| project.name == selected)
    }

    pub(crate) fn selected_project_name(&self) -> &str {
        self.selected_project
            .as_deref()
            .unwrap_or(DEFAULT_PROJECT_NAME)
    }

    pub(crate) fn set_object_column_visible(&mut self, column: ObjectColumn, visible: bool) {
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

    pub(crate) fn favorite_objects_for_context(&self, context: &str) -> Vec<ObjectFavorite> {
        self.favorite_objects
            .iter()
            .filter(|favorite| favorite.context == context)
            .cloned()
            .collect()
    }

    pub(crate) fn is_object_favorite(&self, target: &DetailTarget) -> bool {
        self.favorite_objects
            .iter()
            .any(|favorite| favorite.matches_target(target))
    }

    pub(crate) fn toggle_object_favorite(&mut self, target: &DetailTarget) -> bool {
        let favorite = ObjectFavorite::from_target(target);
        let previous_len = self.favorite_objects.len();
        self.favorite_objects
            .retain(|existing| existing != &favorite);

        if previous_len == self.favorite_objects.len() {
            self.favorite_objects.push(favorite);
        }

        self.normalize_favorite_objects();
        true
    }

    pub(crate) fn object_column_width(&self, column: ObjectColumn) -> i32 {
        self.object_column_widths
            .iter()
            .find(|entry| entry.column == column)
            .map(|entry| entry.width)
            .unwrap_or_else(|| column.default_width())
            .max(OBJECT_COLUMN_MIN_WIDTH)
    }

    pub(crate) fn object_name_width(&self) -> i32 {
        self.object_name_width
            .unwrap_or(OBJECT_NAME_WIDTH)
            .max(OBJECT_NAME_MIN_WIDTH)
    }

    pub(crate) fn set_object_table_column_width(
        &mut self,
        column: ObjectTableColumn,
        width: i32,
    ) -> bool {
        match column {
            ObjectTableColumn::Name => self.set_object_name_width(width),
            ObjectTableColumn::Data(column) => self.set_object_column_width(column, width),
        }
    }

    pub(crate) fn last_namespace_for_context(&self, context: Option<&str>) -> Option<&str> {
        let context = context?;
        self.last_namespaces_by_context
            .iter()
            .find(|entry| entry.context == context)
            .map(|entry| entry.namespace.as_str())
    }

    pub(crate) fn set_last_namespace_for_context(
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
        let width = width.max(OBJECT_NAME_MIN_WIDTH);
        let next = (width != OBJECT_NAME_WIDTH).then_some(width);
        if self.object_name_width == next {
            return false;
        }
        self.object_name_width = next;
        true
    }

    pub(crate) fn set_object_column_width(&mut self, column: ObjectColumn, width: i32) -> bool {
        let width = width.max(OBJECT_COLUMN_MIN_WIDTH);
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

    fn migrate_object_columns(&mut self) {
        if self.object_column_schema_version < 1
            && !self.visible_object_columns.contains(&ObjectColumn::Image)
        {
            self.visible_object_columns.push(ObjectColumn::Image);
        }
        if self.object_column_schema_version < 2 {
            self.visible_object_columns
                .extend([ObjectColumn::Target, ObjectColumn::Selector]);
        }
        if self.object_column_schema_version < 3 {
            self.visible_object_columns.push(ObjectColumn::IngressClass);
        }
        self.object_column_schema_version = OBJECT_COLUMN_SCHEMA_VERSION;
    }

    fn normalize_object_column_widths(&mut self) {
        self.object_column_widths
            .retain(|entry| ObjectColumn::ALL.contains(&entry.column));
        for entry in &mut self.object_column_widths {
            entry.width = entry.width.max(OBJECT_COLUMN_MIN_WIDTH);
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
            .map(|width| width.max(OBJECT_NAME_MIN_WIDTH))
            .filter(|width| *width != OBJECT_NAME_WIDTH);
    }

    fn normalize_favorite_objects(&mut self) {
        for favorite in &mut self.favorite_objects {
            favorite.context = favorite.context.trim().to_owned();
            favorite.group = favorite.group.trim().to_owned();
            favorite.version = favorite.version.trim().to_owned();
            favorite.api_version = favorite.api_version.trim().to_owned();
            favorite.kind = favorite.kind.trim().to_owned();
            favorite.plural = favorite.plural.trim().to_owned();
            favorite.namespace = favorite
                .namespace
                .as_deref()
                .map(str::trim)
                .filter(|namespace| !namespace.is_empty())
                .map(str::to_owned);
            favorite.name = favorite.name.trim().to_owned();
        }
        self.favorite_objects.retain(|favorite| {
            !favorite.context.is_empty() && !favorite.kind.is_empty() && !favorite.name.is_empty()
        });
        self.favorite_objects.sort_by(|left, right| {
            left.context
                .cmp(&right.context)
                .then(left.group.cmp(&right.group))
                .then(left.kind.cmp(&right.kind))
                .then(left.namespace.cmp(&right.namespace))
                .then(left.name.cmp(&right.name))
        });
        self.favorite_objects.dedup();
    }

    pub(crate) fn save(&self) -> Result<(), String> {
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

    pub(crate) fn read_from_disk() -> Option<Self> {
        let path = Self::path()?;
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub(crate) fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|path| path.join("aetheris").join("projects.json"))
    }

    pub(crate) fn normalize_contexts(&mut self, contexts: &[ContextInfo]) {
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

    pub(crate) fn add_contexts_to_selected_project<I>(&mut self, contexts: I)
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

    pub(crate) fn remove_context_from_selected_project(&mut self, context: &str) {
        let Some(project) = self.selected_project_mut() else {
            return;
        };
        project.contexts.retain(|candidate| candidate != context);
        project
            .custom_namespaces_by_context
            .retain(|entry| entry.context != context);
    }

    pub(crate) fn selected_project_mut(&mut self) -> Option<&mut Project> {
        let selected = self.selected_project.clone()?;
        self.projects
            .iter_mut()
            .find(|project| project.name == selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aetheris_kube::ResourceScope;

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

    fn resource(group: &str, kind: &str) -> ResourceKind {
        ResourceKind {
            group: group.to_owned(),
            version: String::from("v1"),
            api_version: if group.is_empty() {
                String::from("v1")
            } else {
                format!("{group}/v1")
            },
            kind: kind.to_owned(),
            plural: format!("{}s", kind.to_ascii_lowercase()),
            scope: ResourceScope::Namespaced,
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
            object_column_schema_version: OBJECT_COLUMN_SCHEMA_VERSION,
            visible_object_columns: default_object_columns(),
            object_name_width: None,
            object_column_widths: Vec::new(),
            favorite_objects: Vec::new(),
        };

        store.normalize_contexts(&[context("local"), context("external")]);

        let project = store.selected_project().unwrap();
        assert_eq!(project.contexts, vec![String::from("local")]);
    }

    #[test]
    fn normalize_contexts_keeps_empty_store_empty() {
        let mut store = ProjectStore::empty();

        store.normalize_contexts(&[context("prod")]);

        assert!(store.projects.is_empty());
        assert_eq!(store.selected_project, None);
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
            object_column_schema_version: OBJECT_COLUMN_SCHEMA_VERSION,
            visible_object_columns: default_object_columns(),
            object_name_width: None,
            object_column_widths: Vec::new(),
            favorite_objects: Vec::new(),
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
    fn object_table_columns_have_no_maximum_width() {
        let mut store = ProjectStore::default();
        let wide = 10_000;

        assert!(store.set_object_column_width(ObjectColumn::Image, wide));
        assert_eq!(store.object_column_width(ObjectColumn::Image), wide);

        assert!(store.set_object_column_width(ObjectColumn::Namespace, wide));
        assert_eq!(store.object_column_width(ObjectColumn::Namespace), wide);

        assert!(store.set_object_table_column_width(ObjectTableColumn::Name, wide));
        assert_eq!(store.object_name_width(), wide);
    }

    #[test]
    fn object_table_columns_keep_minimum_width() {
        let mut store = ProjectStore::default();

        assert!(store.set_object_column_width(ObjectColumn::Image, 0));
        assert_eq!(
            store.object_column_width(ObjectColumn::Image),
            OBJECT_COLUMN_MIN_WIDTH
        );

        assert!(store.set_object_table_column_width(ObjectTableColumn::Name, 0));
        assert_eq!(store.object_name_width(), OBJECT_NAME_MIN_WIDTH);
    }

    #[test]
    fn toggle_object_favorite_tracks_object_by_context_resource_namespace_and_name() {
        let mut store = ProjectStore::default();
        let target = DetailTarget {
            context: String::from("prod"),
            resource: resource("apps", "Deployment"),
            namespace: Some(String::from("my-namespace")),
            name: String::from("sample-deploy"),
        };

        assert!(!store.is_object_favorite(&target));
        assert!(store.toggle_object_favorite(&target));
        assert!(store.is_object_favorite(&target));
        assert_eq!(
            store.favorite_objects_for_context("prod")[0].kind(),
            "Deployment"
        );

        assert!(store.toggle_object_favorite(&target));
        assert!(!store.is_object_favorite(&target));
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
            object_column_schema_version: OBJECT_COLUMN_SCHEMA_VERSION,
            visible_object_columns: default_object_columns(),
            object_name_width: None,
            object_column_widths: Vec::new(),
            favorite_objects: Vec::new(),
        };

        store.normalize_last_namespaces(&[context("prod")]);

        assert_eq!(
            store.last_namespace_for_context(Some("prod")),
            Some("team-a")
        );
        assert_eq!(store.last_namespace_for_context(Some("external")), None);
    }
}
