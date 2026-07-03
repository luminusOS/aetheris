use super::commands::*;
use super::object_detail::*;
use super::utils::*;
use super::widgets::*;
use super::*;

impl App {
    pub(super) fn show_setup(&self) {
        self.root_stack.set_visible_child_name("setup");
    }

    pub(super) fn show_projects(&self) {
        self.root_stack.set_visible_child_name("projects");
    }

    pub(super) fn show_browser(&self) {
        self.root_stack.set_visible_child_name("browser");
    }

    /// Shows the Clusters page for the currently selected project: rebuilds
    /// the list, kicks off a background summary fetch for any context that
    /// doesn't have one cached yet, and switches `root_stack`. Shared by
    /// every path that lands on this page (picking a project, coming back
    /// from Browser, and after adding/editing a cluster).
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

    /// Loads the clusters view for whichever project is now selected in
    /// `self.projects`. Shared by switching projects, deleting the current
    /// one (falling back to another), and duplicating one.
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
        self.status = String::from("Select a cluster.");
        self.sync_status();
    }

    pub(super) fn show_object_list(&self) {
        self.content_stack.set_visible_child_name("list");
        self.content_header_stack.set_visible_child_name("search");
        self.detail_back_button.set_visible(false);
        self.detail_delete_button.set_visible(false);
        self.detail_terminal_button.set_visible(false);
    }

    pub(super) fn sync_object_columns(&self) {
        rebuild_column_filter_list(
            &self.column_filter_list,
            &self.offerable_object_columns(),
            &self.projects.visible_object_columns,
        );
    }

    /// Columns that make sense to offer/render for the currently selected
    /// resource kind (e.g. "Status" is only meaningful for resources that
    /// expose a ready/desired ratio, like Deployments; "CPU"/"Memory" only
    /// for Pods and Nodes, the only kinds metrics.k8s.io covers).
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
        self.detail_back_button.set_visible(true);
        self.detail_delete_button.set_visible(true);
        self.sync_terminal_controls();
    }

    pub(super) fn present_content_panel(&self) {
        if self.split_view.is_collapsed() {
            self.split_view.set_show_content(true);
        }
    }

    pub(super) fn project_contexts(&self) -> Vec<&ContextInfo> {
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
        self.custom_namespace_entry
            .set_text(custom_namespace_initial_text(&self.selected_namespace));
        self.custom_namespace_entry.grab_focus();
        self.custom_namespace_dialog.present(Some(root));
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
            self.cluster_dialog.set_title("Edit Cluster");
            self.cluster_token_title_label.set_label("Edit cluster");
            self.cluster_token_back_button.set_visible(false);
            self.setup_button.set_label("Save");
        } else {
            self.cluster_dialog.set_title("Add Cluster");
            self.cluster_token_title_label
                .set_label("Connect with token");
            self.cluster_token_back_button.set_visible(true);
            self.setup_button.set_label("Add Cluster");
            self.editing_context_name = None;
        }
    }

    pub(super) fn open_cluster_edit_dialog(
        &mut self,
        context_name: &str,
        server: &str,
        root: &<Self as Component>::Root,
    ) {
        self.setup_name_entry.set_text(context_name);
        self.setup_server_entry.set_text(server);
        self.setup_token_entry.set_text("");
        self.setup_ca_entry.set_text("");
        self.setup_insecure_check.set_active(false);
        self.editing_context_name = Some(context_name.to_owned());
        self.set_cluster_dialog_editing(true);
        self.cluster_dialog_stack.set_visible_child_name("token");
        self.cluster_dialog.present(Some(root));
    }

    pub(super) fn load_cluster(&mut self, sender: ComponentSender<Self>) {
        let Some(context) = self.selected_context.clone() else {
            self.loading = false;
            self.status = String::from("Select a Kubernetes context.");
            self.sync_status();
            return;
        };

        self.show_object_list();
        self.stop_object_watch();
        self.stop_log_stream();
        self.stop_port_forward();
        self.detail_exec_target = None;
        self.detail_port_forward_target = None;
        self.loading = true;
        self.resources.clear();
        self.objects.clear();
        self.selected_resource = None;
        self.status = format!("Discovering resources in {context}...");
        self.rebuild_resource_list(Some(sender.clone()));
        self.rebuild_object_list(Some(sender.clone()));
        self.sync_terminal_controls();
        self.sync_port_forward_controls();
        self.sync_status();
        sender.oneshot_command(async move { load_cluster(context).await });
    }

    pub(super) fn refresh_objects(&mut self, sender: ComponentSender<Self>) {
        let Some(context) = self.selected_context.clone() else {
            self.loading = false;
            self.status = String::from("Select a Kubernetes context.");
            self.sync_status();
            return;
        };
        let Some(resource) = self.selected_resource_kind().cloned() else {
            self.loading = false;
            self.status = String::from("Select a resource.");
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
        self.status = format!("Loading {}...", resource.label());
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

    pub(super) fn remember_namespace(&mut self, namespace: &str) {
        if namespace == "all" || namespace.is_empty() {
            return;
        }

        if !self.namespaces.iter().any(|known| known == namespace) {
            self.namespaces.push(namespace.to_owned());
        }

        if let Some(context) = self.selected_context.clone() {
            if let Some(project) = self.projects.selected_project_mut() {
                if project.add_custom_namespace(&context, namespace) {
                    self.save_projects_or_toast();
                }
            }
        }
    }

    pub(super) fn set_object_status(&mut self, total: usize) {
        let resource = self
            .selected_resource_kind()
            .map(ResourceKind::label)
            .unwrap_or_else(|| String::from("objects"));
        let filtered = self.filtered_objects().len();

        self.status = if self.search_query.trim().is_empty() {
            format!(
                "{} {} in {}",
                total,
                if total == 1 { "object" } else { "objects" },
                resource
            )
        } else {
            format!("{filtered}/{total} objects in {resource}")
        };
    }

    pub(super) fn sync_dropdowns(&self, sender: Option<ComponentSender<Self>>) {
        self.project_title_label
            .set_label(self.projects.selected_project_name());
        self.rebuild_project_list();

        self.rebuild_cluster_list();
        let context_label = self.selected_context.as_deref().unwrap_or("No cluster");
        self.context_selector_label.set_label(context_label);
        self.context_selector_label
            .set_tooltip_text(Some(context_label));

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
        self.detail_apply_button
            .set_sensitive(self.detail_target.is_some() && !self.loading);
        self.detail_download_yaml_button
            .set_sensitive(self.detail_target.is_some() && !self.loading);
        self.detail_explain_yaml_button
            .set_sensitive(self.detail_target.is_some() && !self.loading);
        self.detail_delete_button
            .set_sensitive(self.detail_target.is_some() && !self.loading);
        self.detail_terminal_button.set_sensitive(
            self.detail_exec_target
                .as_ref()
                .is_some_and(|target| !target.containers.is_empty())
                && !self.loading,
        );
        self.detail_scale_button.set_sensitive(
            self.detail_target
                .as_ref()
                .is_some_and(|target| is_deployment_resource(&target.resource))
                && !self.loading,
        );
        self.detail_cordon_button.set_sensitive(
            self.detail_target
                .as_ref()
                .is_some_and(|target| is_node_resource(&target.resource))
                && !self.loading,
        );
        self.detail_drain_button.set_sensitive(
            self.detail_target
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
                .title("No resources")
                .subtitle("Connect to a cluster to load API resources.")
                .build();
            self.resource_list.append(&row);
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
                connect_resource_row(&child, sender.clone(), resource_index);
                row.add_row(&child);
            }

            self.resource_list.append(&row);
        }
    }

    /// Clears and repopulates the object list. With large resources (a
    /// namespace with thousands of Pods, say) building every row up front
    /// would block the main loop long enough to feel frozen, so only the
    /// first chunk is built synchronously; the rest is appended a chunk at
    /// a time via idle callbacks so input and redraws keep happening.
    pub(super) fn rebuild_object_list(&mut self, sender: Option<ComponentSender<Self>>) {
        const CHUNK_SIZE: usize = 150;

        self.object_list_generation
            .set(self.object_list_generation.get().wrapping_add(1));
        let generation = self.object_list_generation.get();
        let generation_cell = self.object_list_generation.clone();

        while let Some(child) = self.object_list.first_child() {
            self.object_list.remove(&child);
        }

        let mut objects: std::collections::VecDeque<ObjectSummary> =
            self.filtered_objects().into_iter().cloned().collect();

        if objects.is_empty() {
            let row = adw::ActionRow::builder()
                .title("No objects")
                .subtitle("The selected resource has no objects or could not be loaded.")
                .build();
            self.object_list.append(&row);
            return;
        }

        let offerable = self.offerable_object_columns();
        let columns: Vec<ObjectColumn> = self
            .projects
            .visible_object_columns
            .iter()
            .copied()
            .filter(|column| offerable.contains(column))
            .collect();
        let name_width = self.projects.object_name_width();
        let widths = self.projects.object_column_widths_for(&columns);

        self.object_list
            .append(&object_header_row_with_column_widths(
                name_width,
                &columns,
                &widths,
                sender.clone(),
                Some(self.object_list.clone()),
            ));
        for _ in 0..CHUNK_SIZE {
            let Some(object) = objects.pop_front() else {
                return;
            };
            self.object_list.append(&object_row_with_column_widths(
                &object, name_width, &columns, &widths,
            ));
        }

        let list = self.object_list.clone();
        gtk::glib::idle_add_local(move || {
            if generation_cell.get() != generation {
                return gtk::glib::ControlFlow::Break;
            }
            for _ in 0..CHUNK_SIZE {
                let Some(object) = objects.pop_front() else {
                    return gtk::glib::ControlFlow::Break;
                };
                list.append(&object_row_with_column_widths(
                    &object, name_width, &columns, &widths,
                ));
            }
            gtk::glib::ControlFlow::Continue
        });
    }

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
        sort_objects(&mut self.objects);
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
        let (name, namespace) = self
            .filtered_objects()
            .get(index as usize)
            .map(|object| (object.name.clone(), object.namespace.clone()))?;
        let context = self.selected_context.clone()?;
        let resource = self.selected_resource_kind()?.clone();
        let namespace = resource.is_namespaced().then_some(namespace);

        Some((context, resource, namespace, name))
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
        self.detail_log_buffer.set_text("");
        self.reset_detail_overview_layout();
        // Don't switch tabs yet: the previous object's detail page (and
        // whichever tab the user was on) stays on screen until the new
        // object's data actually arrives, so resetting here would flash
        // back to YAML on the OLD object before the new one loads.
        // `sync_detail_tabs` (run once new data is in) already falls back
        // to "yaml" if the current tab isn't valid for the new object.
        self.detail_target = Some(DetailTarget {
            context: context.clone(),
            resource: resource.clone(),
            namespace: namespace.clone(),
            name: name.clone(),
        });
        self.detail_log_target =
            pod_log_target(context.clone(), &resource, namespace.clone(), name.clone());
        self.detail_exec_target =
            pod_log_target(context.clone(), &resource, namespace.clone(), name.clone());
        self.detail_port_forward_target =
            pod_log_target(context.clone(), &resource, namespace.clone(), name.clone());
        self.detail_request_token = self.detail_request_token.saturating_add(1);
        let detail_token = self.detail_request_token;
        self.sync_log_controls();
        self.sync_terminal_controls();
        self.sync_port_forward_controls();

        self.loading = true;
        self.status = format!("Loading details for {name}...");
        self.sync_status();
        sender.oneshot_command(async move {
            load_object_detail(detail_token, context, resource, namespace, name).await
        });
    }

    pub(super) fn reset_detail_overview_layout(&self) {
        self.detail_overview_section.set_visible(true);
        self.detail_expand_logs_button
            .set_icon_name("view-fullscreen-symbolic");
        self.detail_expand_logs_button
            .set_tooltip_text(Some("Hide summary to see more of this tab"));
    }

    pub(super) fn populate_detail_dialog(&mut self, detail: &ObjectDetail) {
        self.detail_name_label.set_label(&detail.name);
        self.detail_namespace_label.set_label(&detail.namespace);
        self.detail_status_label.set_label(&detail.status);
        self.detail_kind_label.set_label(&detail.kind);
        self.detail_api_label.set_label(&detail.api_version);
        self.detail_age_label.set_label(&detail.age);
        self.detail_cpu_label.set_label(
            detail
                .metrics
                .as_ref()
                .map(|usage| usage.cpu.as_str())
                .unwrap_or("-"),
        );
        self.detail_memory_label.set_label(
            detail
                .metrics
                .as_ref()
                .map(|usage| usage.memory.as_str())
                .unwrap_or("-"),
        );
        self.detail_yaml_buffer.set_text(&detail.yaml);
        self.detail_related_pods.clone_from(&detail.related_pods);
        self.detail_node_unschedulable = detail.node_unschedulable;
        self.detail_scale_spin
            .set_value(detail.replicas.unwrap_or_default().into());
        self.detail_scale_spin
            .set_visible(detail.replicas.is_some());
        self.detail_scale_button
            .set_visible(detail.replicas.is_some());
        self.detail_cordon_button
            .set_visible(detail.node_unschedulable.is_some());
        self.detail_drain_button
            .set_visible(detail.node_unschedulable.is_some());
        self.detail_explain_yaml_button.set_sensitive(true);
        if let Some(unschedulable) = detail.node_unschedulable {
            self.detail_cordon_button
                .set_label(if unschedulable { "Uncordon" } else { "Cordon" });
        }
        rebuild_detail_events(&self.detail_events_list, detail);
        rebuild_detail_conditions(&self.detail_conditions_list, detail);
        rebuild_related_pods(&self.detail_related_pods_list, detail);
        rebuild_container_metrics(&self.detail_container_metrics_list, detail);
        self.sync_detail_tabs(detail);
        self.sync_terminal_controls();
        self.sync_port_forward_controls();
    }

    pub(super) fn sync_detail_tabs(&self, detail: &ObjectDetail) {
        let show_logs = detail.kind == "Pod" && !detail.containers.is_empty();
        self.detail_port_forward_group
            .set_visible(detail.kind == "Pod");
        let show_pods = detail.kind == "Deployment";
        let show_conditions = !detail.conditions.is_empty();
        let show_containers = detail.kind == "Pod";

        set_stack_page(
            &self.detail_stack,
            "pods",
            show_pods,
            &format!("Pods ({})", detail.related_pods.len()),
        );
        set_stack_page(
            &self.detail_stack,
            "conditions",
            show_conditions,
            &format!("Conditions ({})", detail.conditions.len()),
        );
        set_stack_page(
            &self.detail_stack,
            "containers",
            show_containers,
            &format!("Containers ({})", detail.containers.len()),
        );
        set_stack_page(
            &self.detail_stack,
            "events",
            true,
            &format!("Recent Events ({})", detail.events.len()),
        );
        set_stack_page(&self.detail_stack, "logs", show_logs, "Logs");
        set_stack_page(&self.detail_stack, "yaml", true, "YAML");

        let visible_name = self.detail_stack.visible_child_name();
        let visible_child_is_hidden = visible_name
            .as_deref()
            .and_then(|name| self.detail_stack.child_by_name(name))
            .is_some_and(|child| !child.is_visible());
        if visible_child_is_hidden {
            self.detail_stack.set_visible_child_name("yaml");
        }
    }
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
