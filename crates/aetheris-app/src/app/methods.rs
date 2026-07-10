use super::commands::*;
use super::object_detail::*;
use super::utils::*;
use super::widgets::*;
use super::*;

impl App {
    pub(super) fn show_projects(&self) {
        self.root_stack.set_visible_child_name("projects");
    }

    pub(super) fn show_browser(&self) {
        self.root_stack.set_visible_child_name("browser");
    }

    pub(super) fn enter_clusters_page(&mut self, sender: ComponentSender<Self>) {
        self.rebuild_cluster_list();
        self.ensure_cluster_summaries_loading(sender);
        self.show_clusters();
    }

    pub(super) fn show_clusters(&self) {
        self.root_stack.set_visible_child_name("clusters");
    }

    pub(super) fn ensure_cluster_summaries_loading(&mut self, sender: ComponentSender<Self>) {
        let pending: Vec<String> = self
            .visible_contexts()
            .iter()
            .map(|context| context.name.clone())
            .filter(|name| !self.cluster_summaries.contains_key(name))
            .collect();
        for context_name in pending {
            self.cluster_summaries
                .insert(context_name.clone(), ClusterSummaryState::Loading);
            sender.oneshot_command(async move { load_cluster_summary(context_name).await });
        }
    }

    pub(super) fn refresh_cluster_summaries(&mut self, sender: ComponentSender<Self>) {
        let contexts = self
            .visible_contexts()
            .iter()
            .map(|context| context.name.clone())
            .collect::<Vec<_>>();
        for context_name in contexts {
            self.cluster_summaries
                .insert(context_name.clone(), ClusterSummaryState::Loading);
            sender.oneshot_command(async move { load_cluster_summary(context_name).await });
        }
        self.rebuild_cluster_list();
    }

    pub(super) fn switch_to_project(&mut self, sender: ComponentSender<Self>) {
        if !self
            .visible_contexts()
            .iter()
            .any(|context| self.selected_context.as_deref() == Some(context.name.as_str()))
        {
            self.selected_context = None;
        }
        self.sync_dropdowns(Some(sender.clone()));
        self.enter_clusters_page(sender);
        self.present_content_panel();
        self.loading = false;
        self.status = tr("Select a cluster.");
        self.sync_status();
    }

    pub(super) fn show_object_list(&self) {
        self.content_stack.set_visible_child_name("list");
        self.content_header_stack.set_visible_child_name("search");
        self.detail.back_button.set_visible(false);
        self.detail.delete_button.set_visible(false);
        self.detail.favorite_button.set_visible(false);
        self.detail.terminal_button.set_visible(false);
    }

    pub(super) fn sync_object_columns(&self) {
        rebuild_column_filter_list(
            &self.column_filter_list,
            &self.offerable_object_columns(),
            &self.projects.visible_object_columns,
        );
        let offerable = self.offerable_object_columns();
        for (table_column, view_column) in &self.object_columns {
            match table_column {
                ObjectTableColumn::Name => {
                    view_column.set_fixed_width(self.projects.object_name_width());
                }
                ObjectTableColumn::Data(column) => {
                    view_column.set_visible(
                        offerable.contains(column)
                            && self.projects.visible_object_columns.contains(column),
                    );
                    view_column.set_fixed_width(self.projects.object_column_width(*column));
                }
            }
        }
    }

    pub(super) fn offerable_object_columns(&self) -> Vec<ObjectColumn> {
        offerable_columns_for(self.selected_resource_kind())
    }

    pub(super) fn sync_status_filter(&self) {
        rebuild_status_filter_list(&self.status_filter_list, &self.selected_status_filters);
    }

    pub(super) fn show_detail_page(&self, title: &str) {
        self.content_stack.set_visible_child_name("detail");
        self.content_title_label.set_label(title);
        self.content_header_stack.set_visible_child_name("title");
        self.detail.back_button.set_visible(true);
        self.detail.delete_button.set_visible(true);
        self.detail.favorite_button.set_visible(true);
        self.sync_detail_favorite_button();
        self.sync_terminal_controls();
    }

    // Nautilus behavior: picking something in the overlay sidebar dismisses
    // it so the content it drives is immediately visible; when the sidebar
    // sits side-by-side there is nothing to dismiss.
    pub(super) fn present_content_panel(&self) {
        if self.split_view.is_collapsed() {
            self.split_view.set_show_sidebar(false);
        }
    }

    pub(super) fn project_contexts(&self) -> Vec<&ContextInfo> {
        if self.projects.projects.is_empty() {
            return Vec::new();
        }

        let Some(project) = self.projects.selected_project() else {
            return self.contexts.iter().collect();
        };
        let allowed_contexts = project
            .contexts
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        self.contexts
            .iter()
            .filter(|context| allowed_contexts.contains(context.name.as_str()))
            .collect()
    }

    pub(super) fn visible_contexts(&self) -> Vec<&ContextInfo> {
        self.project_contexts()
    }

    pub(super) fn save_projects_or_toast(&self) {
        if let Err(error) = self.projects.save() {
            self.toaster.add_toast(adw::Toast::new(&error));
        }
    }

    pub(super) fn show_custom_namespace_dialog(&self, root: &<Self as Component>::Root) {
        self.custom_namespace_entry.set_text("");
        self.custom_namespace_entry.grab_focus();
        self.custom_namespace_dialog.present(Some(root));
    }

    /// The add and edit flows share one cluster form; opening "add" must
    /// not show whatever the last add/edit left behind.
    pub(super) fn reset_cluster_dialog_form(&self) {
        self.setup_name_entry.set_text("");
        self.setup_server_entry.set_text("");
        self.setup_token_entry.set_text("");
        self.setup_ca_entry.set_text("");
        self.setup_insecure_check.set_active(false);
    }

    pub(super) fn open_rename_namespace_dialog(
        &mut self,
        namespace: &str,
        root: &<Self as Component>::Root,
    ) {
        self.renaming_namespace = Some(namespace.to_owned());
        self.rename_namespace_entry.set_text(namespace);
        self.rename_namespace_entry.grab_focus();
        self.rename_namespace_dialog.present(Some(root));
    }

    pub(super) fn set_cluster_dialog_editing(&mut self, editing: bool) {
        self.editing_cluster = editing;
        if editing {
            self.cluster_dialog.set_title(&tr("Edit Cluster"));
            self.cluster_token_title_label
                .set_label(&tr("Edit cluster"));
            self.cluster_token_back_button.set_visible(false);
            self.setup_button.set_label(&tr("Save"));
        } else {
            self.cluster_dialog.set_title(&tr("Add Cluster"));
            self.cluster_token_title_label
                .set_label(&tr("Connect with token"));
            self.cluster_token_back_button.set_visible(true);
            self.setup_button.set_label(&tr("Add Cluster"));
            self.editing_context_name = None;
        }
    }

    pub(super) fn open_cluster_edit_dialog(
        &mut self,
        context_name: &str,
        server: &str,
        insecure_skip_tls_verify: bool,
        root: &<Self as Component>::Root,
    ) {
        self.setup_name_entry.set_text(context_name);
        self.setup_server_entry.set_text(server);
        self.setup_token_entry.set_text("");
        self.setup_ca_entry.set_text("");
        self.setup_insecure_check
            .set_active(insecure_skip_tls_verify);
        self.editing_context_name = Some(context_name.to_owned());
        self.set_cluster_dialog_editing(true);
        self.cluster_dialog_stack.set_visible_child_name("token");
        self.cluster_dialog.present(Some(root));
    }

    pub(super) fn load_cluster(&mut self, sender: ComponentSender<Self>) {
        let Some(context) = self.selected_context.clone() else {
            self.loading = false;
            self.status = tr("Select a Kubernetes context.");
            self.sync_status();
            return;
        };

        self.show_object_list();
        self.stop_object_watch();
        self.stop_log_stream();
        self.stop_port_forward();
        self.detail.exec_target = None;
        self.detail.port_forward_target = None;
        self.loading = true;
        self.namespaces = with_all_namespace(Vec::new());
        self.resources.clear();
        self.objects.clear();
        self.selected_namespace = String::from("all");
        self.selected_resource = None;
        self.status = tr_format(
            "Discovering resources in {context}...",
            &[("{context}", context.clone())],
        );
        self.sync_dropdowns(Some(sender.clone()));
        self.rebuild_resource_list(Some(sender.clone()));
        self.rebuild_object_list();
        self.sync_terminal_controls();
        self.sync_port_forward_controls();
        self.sync_status();
        sender.oneshot_command(async move { load_cluster(context).await });
    }

    pub(super) fn refresh_objects(&mut self, sender: ComponentSender<Self>) {
        let Some(context) = self.selected_context.clone() else {
            self.loading = false;
            self.status = tr("Select a Kubernetes context.");
            self.sync_status();
            return;
        };
        let Some(resource) = self.selected_resource_kind().cloned() else {
            self.loading = false;
            self.status = tr("Select a resource.");
            self.sync_status();
            return;
        };

        let namespace = if resource.is_namespaced() {
            Some(self.selected_namespace.clone())
        } else {
            None
        };
        self.stop_object_watch();
        self.loading = true;
        self.status = tr_format("Loading {resource}...", &[("{resource}", resource.label())]);
        self.sync_status();
        sender.oneshot_command(async move { list_objects(context, resource, namespace).await });
    }

    pub(super) fn selected_resource_kind(&self) -> Option<&ResourceKind> {
        self.selected_resource
            .and_then(|index| self.resources.get(index))
    }

    pub(super) fn namespace_choices(&self) -> Vec<String> {
        let mut choices = self.namespaces.clone();
        if let Some(project) = self.projects.selected_project() {
            choices.extend(project.custom_namespaces_for_context(self.selected_context.as_deref()));
        }

        if !self.selected_namespace.is_empty()
            && !choices
                .iter()
                .any(|namespace| namespace == &self.selected_namespace)
        {
            choices.push(self.selected_namespace.clone());
        }

        choices.sort();
        choices.dedup();
        if let Some(index) = choices.iter().position(|namespace| namespace == "all") {
            let all = choices.remove(index);
            choices.insert(0, all);
        }
        choices
    }

    pub(super) fn namespace_is_known(&self, namespace: &str) -> bool {
        self.namespaces.iter().any(|known| known == namespace)
            || self.projects.selected_project().is_some_and(|project| {
                project.has_custom_namespace(self.selected_context.as_deref(), namespace)
            })
    }

    pub(super) fn preferred_namespace_for_selected_context(&self, fallback: &str) -> String {
        self.projects
            .last_namespace_for_context(self.selected_context.as_deref())
            .filter(|namespace| self.namespace_is_known(namespace))
            .map(str::to_owned)
            .unwrap_or_else(|| {
                self.namespaces
                    .first()
                    .cloned()
                    .unwrap_or_else(|| String::from(fallback))
            })
    }

    pub(super) fn remember_selected_namespace(&mut self) {
        let Some(context) = self.selected_context.clone() else {
            return;
        };
        if self
            .projects
            .set_last_namespace_for_context(&context, &self.selected_namespace)
        {
            self.save_projects_or_toast();
        }
    }

    pub(super) fn remember_namespace(&mut self, namespace: &str) {
        if namespace == "all" || namespace.is_empty() {
            return;
        }

        if !self.namespaces.iter().any(|known| known == namespace) {
            self.namespaces.push(namespace.to_owned());
        }

        if let Some(context) = self.selected_context.clone()
            && let Some(project) = self.projects.selected_project_mut()
            && project.add_custom_namespace(&context, namespace)
        {
            self.save_projects_or_toast();
        }
    }

    pub(super) fn set_object_status(&mut self, total: usize) {
        let resource = self
            .selected_resource_kind()
            .map(ResourceKind::label)
            .unwrap_or_else(|| tr("objects"));
        let filtered = self.filtered_objects().len();

        self.status = if self.search_query.trim().is_empty() {
            format!(
                "{} {} in {}",
                total,
                if total == 1 { "object" } else { "objects" },
                resource
            )
        } else {
            tr_format(
                "{filtered}/{total} objects in {resource}",
                &[
                    ("{filtered}", filtered.to_string()),
                    ("{total}", total.to_string()),
                    ("{resource}", resource.to_string()),
                ],
            )
        };
    }

    pub(super) fn sync_dropdowns(&self, sender: Option<ComponentSender<Self>>) {
        self.project_title_label
            .set_label(self.projects.selected_project_name());
        self.rebuild_project_list();

        self.rebuild_cluster_list();
        let context_label = self
            .selected_context
            .clone()
            .unwrap_or_else(|| tr("No cluster"));
        self.context_selector_label.set_label(&context_label);
        self.context_selector_label
            .set_tooltip_text(Some(&context_label));

        let namespace_choices = self.namespace_choices();
        let custom_namespaces: std::collections::HashSet<String> = self
            .projects
            .selected_project()
            .map(|project| {
                project
                    .custom_namespaces_for_context(self.selected_context.as_deref())
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default();
        while let Some(child) = self.namespace_list.first_child() {
            self.namespace_list.remove(&child);
        }
        for namespace in &namespace_choices {
            self.namespace_list.append(&namespace_selector_row(
                namespace,
                namespace == &self.selected_namespace,
                custom_namespaces.contains(namespace),
                sender.clone(),
            ));
        }
        self.namespace_list.append(&add_namespace_selector_row());
        let namespace_label = if self.selected_namespace.is_empty() {
            "default"
        } else {
            self.selected_namespace.as_str()
        };
        self.namespace_selector_label.set_label(namespace_label);
        self.namespace_selector_label
            .set_tooltip_text(Some(namespace_label));

        rebuild_status_filter_list(&self.status_filter_list, &self.selected_status_filters);
        self.sync_object_columns();
    }

    pub(super) fn sync_status(&self) {
        self.status_label.set_label(&self.status);
        self.spinner.set_spinning(self.loading);
        self.spinner.set_visible(self.loading);
        self.refresh_button
            .set_sensitive(self.selected_context.is_some() && !self.loading);
        self.create_yaml_button.set_sensitive(
            self.selected_context.is_some()
                && self.selected_resource_kind().is_some()
                && !self.loading,
        );
        self.search_entry
            .set_sensitive(self.selected_context.is_some() && !self.loading);
        self.status_filter_list
            .set_sensitive(self.selected_context.is_some() && !self.loading);
        self.column_filter_list
            .set_sensitive(self.selected_context.is_some() && !self.loading);
        self.cluster_back_button.set_sensitive(!self.loading);
        self.cluster_menu_button
            .set_sensitive(self.selected_context.is_some() && !self.loading);
        self.cluster_refresh_button
            .set_sensitive(!self.loading && !self.visible_contexts().is_empty());
        self.add_cluster_button.set_sensitive(!self.loading);
        self.import_cluster_button.set_sensitive(!self.loading);
        self.add_project_button.set_sensitive(!self.loading);
        self.namespace_menu_button.set_sensitive(
            self.selected_context.is_some()
                && !self.loading
                && self
                    .selected_resource_kind()
                    .is_none_or(ResourceKind::is_namespaced),
        );
        self.custom_namespace_button.set_sensitive(!self.loading);
        self.rename_namespace_button.set_sensitive(!self.loading);
        self.project_create_button.set_sensitive(!self.loading);
        self.detail
            .apply_button
            .set_sensitive(self.detail.target.is_some() && !self.loading);
        self.detail
            .download_yaml_button
            .set_sensitive(self.detail.target.is_some() && !self.loading);
        self.detail
            .explain_yaml_button
            .set_sensitive(self.detail.target.is_some() && !self.loading);
        self.detail
            .delete_button
            .set_sensitive(self.detail.target.is_some() && !self.loading);
        self.detail
            .favorite_button
            .set_sensitive(self.detail.target.is_some() && !self.loading);
        self.detail.terminal_button.set_sensitive(
            self.detail
                .exec_target
                .as_ref()
                .is_some_and(|target| !target.containers.is_empty())
                && !self.loading,
        );
        self.detail.scale_button.set_sensitive(
            self.detail
                .target
                .as_ref()
                .is_some_and(|target| is_deployment_resource(&target.resource))
                && !self.loading,
        );
        self.detail.cordon_button.set_sensitive(
            self.detail
                .target
                .as_ref()
                .is_some_and(|target| is_node_resource(&target.resource))
                && !self.loading,
        );
        self.detail.drain_button.set_sensitive(
            self.detail
                .target
                .as_ref()
                .is_some_and(|target| is_node_resource(&target.resource))
                && !self.loading,
        );
        self.create_yaml_apply_button.set_sensitive(!self.loading);
    }

    pub(super) fn grouped_resources(&self) -> Vec<(ResourceSection, Vec<(usize, &ResourceKind)>)> {
        ResourceSection::ALL
            .iter()
            .copied()
            .filter_map(|section| {
                let resources = self
                    .resources
                    .iter()
                    .enumerate()
                    .filter(|(_index, resource)| section.matches(resource))
                    .collect::<Vec<_>>();
                (!resources.is_empty()).then_some((section, resources))
            })
            .collect()
    }

    pub(super) fn rebuild_project_list(&self) {
        rebuild_project_list(&self.project_list, &self.projects);
        self.projects_content_stack
            .set_visible_child_name(if self.projects.projects.is_empty() {
                "empty"
            } else {
                "content"
            });
    }

    pub(super) fn rebuild_cluster_list(&self) {
        let visible_contexts = self.visible_contexts();
        rebuild_cluster_list(
            &self.cluster_list,
            &visible_contexts,
            &self.cluster_summaries,
            self.selected_context.as_deref(),
        );
        self.clusters_content_stack
            .set_visible_child_name(if visible_contexts.is_empty() {
                "empty"
            } else {
                "content"
            });
    }

    pub(super) fn rebuild_resource_list(&self, sender: Option<ComponentSender<Self>>) {
        while let Some(child) = self.resource_list.first_child() {
            self.resource_list.remove(&child);
        }

        let resource_groups = self.grouped_resources();

        if resource_groups.is_empty() {
            let row = adw::ActionRow::builder()
                .title(tr("No resources"))
                .subtitle(tr("Connect to a cluster to load API resources."))
                .build();
            self.resource_list.append(&row);
            self.rebuild_favorite_object_list(sender);
            return;
        }

        for (section, resources) in resource_groups {
            let row = adw::ExpanderRow::builder()
                .title(section.label())
                .subtitle(resource_count_label(resources.len()))
                .expanded(section == self.selected_resource_section)
                .build();
            row.add_prefix(&gtk::Image::from_icon_name(available_icon_name(
                section.icon_name(),
                section.fallback_icon_name(),
            )));

            for (resource_index, resource) in resources {
                let child = resource_row(resource, self.selected_resource == Some(resource_index));
                connect_resource_row(&child, sender.clone(), resource_index, section);
                row.add_row(&child);
            }

            self.resource_list.append(&row);
        }
        self.rebuild_favorite_object_list(sender);
    }

    pub(super) fn rebuild_favorite_object_list(&self, sender: Option<ComponentSender<Self>>) {
        while let Some(child) = self.favorite_object_list.first_child() {
            self.favorite_object_list.remove(&child);
        }

        let Some(context) = self.selected_context.as_deref() else {
            let row = adw::ActionRow::builder()
                .title(tr("No favorites"))
                .subtitle(tr("Select a cluster to show favorite objects."))
                .build();
            self.favorite_object_list.append(&row);
            return;
        };
        let favorites = self.projects.favorite_objects_for_context(context);

        if favorites.is_empty() {
            let row = adw::ActionRow::builder()
                .title(tr("No favorites"))
                .subtitle(tr("Open an object and star it to keep it here."))
                .build();
            self.favorite_object_list.append(&row);
            return;
        }

        for favorite in favorites {
            let row = favorite_object_row(&favorite);
            connect_favorite_object_row(&row, sender.clone(), favorite);
            self.favorite_object_list.append(&row);
        }
    }

    pub(super) fn sync_detail_favorite_button(&self) {
        let favorited = self
            .detail
            .target
            .as_ref()
            .is_some_and(|target| self.projects.is_object_favorite(target));
        self.detail
            .favorite_button
            .set_icon_name(available_icon_name(
                if favorited {
                    "aetheris-object-favorite-symbolic"
                } else {
                    "aetheris-object-favorite-outline-symbolic"
                },
                if favorited {
                    "starred-symbolic"
                } else {
                    "non-starred-symbolic"
                },
            ));
        let tooltip = if favorited {
            tr("Remove from favorites")
        } else {
            tr("Add to favorites")
        };
        self.detail.favorite_button.set_tooltip_text(Some(&tooltip));
    }

    /// Replaces the object table's backing model with the current filtered
    /// objects. The `ColumnView` is virtualized, so this is O(model) data
    /// work with only the on-screen row widgets ever being (re)built —
    /// tens of thousands of objects stay cheap.
    pub(super) fn rebuild_object_list(&mut self) {
        let items: Vec<gtk::glib::BoxedAnyObject> = self
            .filtered_objects()
            .into_iter()
            .map(boxed_object)
            .collect();

        if items.is_empty() {
            self.object_store.remove_all();
            self.object_list_stack.set_visible_child_name("empty");
            return;
        }

        self.object_list_stack.set_visible_child_name("table");
        // One splice = one items-changed signal, instead of one per row.
        self.object_store
            .splice(0, self.object_store.n_items(), &items);
    }

    /// Merges one watch event into `objects` without sorting or repainting;
    /// both are deferred to `flush_object_list_refresh` so an event burst
    /// costs one refresh instead of thousands.
    pub(super) fn upsert_object(&mut self, object: ObjectSummary) {
        if let Some(existing) = self
            .objects
            .iter_mut()
            .find(|existing| same_object(existing, &object))
        {
            *existing = object;
        } else {
            self.objects.push(object);
        }
    }

    /// Schedules a coalesced object-list refresh. Any number of calls while
    /// one is pending collapse into a single `ObjectListRefreshTick`.
    pub(super) fn schedule_object_list_refresh(&mut self, sender: &ComponentSender<Self>) {
        if self.object_list_refresh_scheduled {
            return;
        }
        self.object_list_refresh_scheduled = true;
        let sender = sender.clone();
        gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
            sender.input(AppMsg::ObjectListRefreshTick);
        });
    }

    pub(super) fn flush_object_list_refresh(&mut self) {
        self.object_list_refresh_scheduled = false;
        if self.loading {
            return;
        }
        sort_objects(&mut self.objects);
        self.set_object_status(self.objects.len());
        self.sync_status();
        self.rebuild_object_list();
    }

    /// Persists `projects` after a short delay, collapsing bursts (e.g. the
    /// per-pixel width updates of a column-resize drag) into one disk write.
    pub(super) fn schedule_project_save(&mut self, sender: &ComponentSender<Self>) {
        if self.project_save_scheduled {
            return;
        }
        self.project_save_scheduled = true;
        let sender = sender.clone();
        gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(600), move || {
            sender.input(AppMsg::ProjectSaveTick);
        });
    }

    pub(super) fn remove_object(&mut self, object: &ObjectSummary) {
        self.objects
            .retain(|existing| !same_object(existing, object));
    }

    pub(super) fn filtered_objects(&self) -> Vec<&ObjectSummary> {
        let query = self.search_query.trim().to_ascii_lowercase();
        self.objects
            .iter()
            .filter(|object| {
                StatusFilter::matches_any(&object.status, &self.selected_status_filters)
            })
            .filter(|object| query.is_empty() || object_matches(object, &query))
            .collect()
    }

    pub(super) fn detail_request(
        &self,
        index: i32,
    ) -> Option<(String, ResourceKind, Option<String>, String)> {
        let object = sorted_model_object(&self.object_sorted, index)?;
        let context = self.selected_context.clone()?;
        let resource = self.selected_resource_kind()?.clone();
        let namespace = resource.is_namespaced().then_some(object.namespace);

        Some((context, resource, namespace, object.name))
    }

    pub(super) fn related_pod_at(&self, index: i32) -> Option<ObjectSummary> {
        sorted_model_object(&self.detail.related_pods_sorted, index)
    }

    pub(super) fn open_object_detail(
        &mut self,
        context: String,
        resource: ResourceKind,
        namespace: Option<String>,
        name: String,
        sender: ComponentSender<Self>,
    ) {
        self.stop_log_stream();
        self.stop_port_forward();
        self.detail.log_buffer.set_text("");
        self.reset_detail_overview_layout();
        // Don't switch tabs yet: the previous object's detail page (and
        // whichever tab the user was on) stays on screen until the new
        // object's data actually arrives, so resetting here would flash
        // back to YAML on the OLD object before the new one loads.
        // `sync_detail_tabs` (run once new data is in) already falls back
        // to "yaml" if the current tab isn't valid for the new object.
        self.detail.target = Some(DetailTarget {
            context: context.clone(),
            resource: resource.clone(),
            namespace: namespace.clone(),
            name: name.clone(),
        });
        self.detail.log_target =
            pod_log_target(context.clone(), &resource, namespace.clone(), name.clone());
        self.detail.exec_target =
            pod_log_target(context.clone(), &resource, namespace.clone(), name.clone());
        self.detail.port_forward_target =
            pod_log_target(context.clone(), &resource, namespace.clone(), name.clone());
        self.detail.request_token = self.detail.request_token.saturating_add(1);
        let detail_token = self.detail.request_token;
        self.sync_log_controls();
        self.sync_terminal_controls();
        self.sync_port_forward_controls();

        self.loading = true;
        self.status = tr_format("Loading details for {name}...", &[("{name}", name.clone())]);
        self.sync_status();
        sender.oneshot_command(async move {
            load_object_detail(detail_token, context, resource, namespace, name).await
        });
    }

    pub(super) fn reset_detail_overview_layout(&self) {
        self.detail.overview_section.set_visible(true);
        self.detail
            .expand_logs_button
            .set_icon_name("view-fullscreen-symbolic");
        self.detail
            .expand_logs_button
            .set_tooltip_text(Some(&tr("Hide summary to see more of this tab")));
    }

    pub(super) fn populate_detail_dialog(&mut self, detail: &ObjectDetail) {
        self.detail.name_label.set_label(&detail.name);
        self.detail.namespace_label.set_label(&detail.namespace);
        self.detail.status_label.set_label(&detail.status);
        self.detail.kind_label.set_label(&detail.kind);
        self.detail.api_label.set_label(&detail.api_version);
        self.detail.age_label.set_label(&detail.age);
        self.detail.cpu_label.set_label(
            detail
                .metrics
                .as_ref()
                .map(|usage| usage.cpu.as_str())
                .unwrap_or("-"),
        );
        self.detail.memory_label.set_label(
            detail
                .metrics
                .as_ref()
                .map(|usage| usage.memory.as_str())
                .unwrap_or("-"),
        );
        self.detail.yaml_buffer.set_text(&detail.yaml);
        self.sync_detail_favorite_button();
        self.detail.node_unschedulable = detail.node_unschedulable;
        self.detail
            .scale_spin
            .set_value(detail.replicas.unwrap_or_default().into());
        self.detail
            .scale_spin
            .set_visible(detail.replicas.is_some());
        self.detail
            .scale_button
            .set_visible(detail.replicas.is_some());
        self.detail
            .cordon_button
            .set_visible(detail.node_unschedulable.is_some());
        self.detail
            .drain_button
            .set_visible(detail.node_unschedulable.is_some());
        self.detail.explain_yaml_button.set_sensitive(true);
        if let Some(unschedulable) = detail.node_unschedulable {
            let label = if unschedulable {
                tr("Uncordon")
            } else {
                tr("Cordon")
            };
            self.detail.cordon_button.set_label(&label);
        }
        rebuild_detail_events(&self.detail.events_list, detail);
        rebuild_detail_conditions(&self.detail.conditions_list, detail);
        rebuild_related_pods(
            &self.detail.related_pods_store,
            &self.detail.related_pods_stack,
            &self.detail.related_pods_message,
            detail,
        );
        rebuild_container_metrics(&self.detail.container_metrics_list, detail);
        self.sync_detail_tabs(detail);
        self.sync_terminal_controls();
        self.sync_port_forward_controls();
    }

    pub(super) fn sync_detail_tabs(&self, detail: &ObjectDetail) {
        let show_logs = detail.kind == "Pod" && !detail.containers.is_empty();
        self.detail
            .port_forward_group
            .set_visible(detail.kind == "Pod");
        let show_pods = detail.kind == "Deployment";
        let show_conditions = !detail.conditions.is_empty();
        let show_containers = detail.kind == "Pod";

        set_stack_page(
            &self.detail.stack,
            "pods",
            show_pods,
            &tr_format(
                "Pods ({count})",
                &[("{count}", detail.related_pods.len().to_string())],
            ),
        );
        set_stack_page(
            &self.detail.stack,
            "conditions",
            show_conditions,
            &tr_format(
                "Conditions ({count})",
                &[("{count}", detail.conditions.len().to_string())],
            ),
        );
        set_stack_page(
            &self.detail.stack,
            "containers",
            show_containers,
            &tr_format(
                "Containers ({count})",
                &[("{count}", detail.containers.len().to_string())],
            ),
        );
        set_stack_page(
            &self.detail.stack,
            "events",
            true,
            &tr_format(
                "Recent Events ({count})",
                &[("{count}", detail.events.len().to_string())],
            ),
        );
        set_stack_page(&self.detail.stack, "logs", show_logs, &tr("Logs"));
        set_stack_page(&self.detail.stack, "yaml", true, &tr("YAML"));

        let visible_name = self.detail.stack.visible_child_name();
        let visible_child_is_hidden = visible_name
            .as_deref()
            .and_then(|name| self.detail.stack.child_by_name(name))
            .is_some_and(|child| !child.is_visible());
        if visible_child_is_hidden {
            self.detail.stack.set_visible_child_name("yaml");
        }
    }
}

/// The object at a view position, resolved against the sorted model the
/// `ColumnView` actually displays (positions differ from the backing store
/// whenever a header-click sort is active).
fn sorted_model_object(model: &gtk::SortListModel, index: i32) -> Option<ObjectSummary> {
    let item = model
        .item(u32::try_from(index).ok()?)
        .and_downcast::<gtk::glib::BoxedAnyObject>()?;
    let object = item.borrow::<ObjectSummary>().clone();
    Some(object)
}

fn same_object(left: &ObjectSummary, right: &ObjectSummary) -> bool {
    left.name == right.name && left.namespace == right.namespace
}

fn sort_objects(objects: &mut [ObjectSummary]) {
    objects.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.name.cmp(&right.name))
    });
}
