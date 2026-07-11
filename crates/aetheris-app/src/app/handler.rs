use super::*;

mod cluster;
mod exec;
mod logs;
mod mutations;
mod namespace;
mod nodes;
mod object_list;
mod port_forward;
mod project;

impl App {
    pub(super) fn handle_msg(
        &mut self,
        msg: AppMsg,
        sender: ComponentSender<Self>,
        root: &<Self as Component>::Root,
    ) {
        match msg {
            AppMsg::Loaded(Ok(state)) => {
                self.contexts = state.contexts;
                self.namespaces = state.namespaces;
                self.projects = state.projects;
                let visible_contexts = self.visible_contexts();
                self.selected_context = visible_contexts
                    .iter()
                    .find(|context| context.is_current)
                    .or_else(|| visible_contexts.first())
                    .map(|context| context.name.clone());
                self.selected_namespace = self.preferred_namespace_for_selected_context("default");
                self.sync_dropdowns(Some(sender.clone()));
                self.loading = false;
                self.status = tr("Select a project.");
                self.show_projects();
                self.sync_status();
            }
            AppMsg::Loaded(Err(error)) => {
                // Kubeconfig exists but won't load. Still land on the
                // projects page — saved projects are independent of the
                // kubeconfig, and the add/import flows can rewrite it.
                self.loading = false;
                self.contexts.clear();
                self.resources.clear();
                self.objects.clear();
                self.selected_context = None;
                self.projects = ProjectStore::load(&[]);
                self.status = tr("Kubeconfig unavailable.");
                self.toaster.add_toast(adw::Toast::new(&error));
                self.show_projects();
                self.sync_dropdowns(Some(sender.clone()));
                self.rebuild_resource_list(Some(sender.clone()));
                self.rebuild_object_list();
                self.sync_status();
            }
            AppMsg::StateLoadedForCluster(context_name, Ok(state)) => {
                self.contexts = state.contexts;
                self.namespaces = state.namespaces;
                self.projects = state.projects;
                self.projects
                    .add_contexts_to_selected_project([context_name.clone()]);
                self.save_projects_or_toast();
                let visible_contexts = self.visible_contexts();
                self.selected_context = visible_contexts
                    .iter()
                    .find(|context| context.name == context_name)
                    .map(|context| context.name.clone());
                self.selected_namespace = self.preferred_namespace_for_selected_context("default");
                self.loading = false;
                self.sync_dropdowns(Some(sender.clone()));
                self.enter_clusters_page(sender);
                self.status = tr("Cluster saved.");
                self.sync_status();
            }
            AppMsg::StateLoadedForCluster(_context_name, Err(error)) => {
                self.loading = false;
                self.status = tr("Unable to reload kubeconfig.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::ShowProjects => project::handle_show_projects(self),
            AppMsg::ShowClusters => cluster::handle_show_clusters(self, sender),
            AppMsg::RefreshClusters => cluster::handle_refresh_clusters(self, sender),
            AppMsg::ClusterSummaryLoaded(context_name, result) => {
                cluster::handle_cluster_summary_loaded(self, context_name, result)
            }
            AppMsg::ProjectChanged(index) => project::handle_project_changed(self, sender, index),
            AppMsg::ShowAddProjectDialog => project::handle_show_add_project_dialog(self, root),
            AppMsg::ShowRenameProjectDialog => {
                project::handle_show_rename_project_dialog(self, root)
            }
            AppMsg::AddProject => project::handle_add_project(self, sender),
            AppMsg::DuplicateProject => project::handle_duplicate_project(self, sender),
            AppMsg::DeleteProject => project::handle_delete_project(self, sender, root),
            AppMsg::ConfirmDeleteProject => project::handle_confirm_delete_project(self, sender),
            AppMsg::ClusterLoaded(Ok(state)) => {
                cluster::handle_cluster_loaded_ok(self, sender, state)
            }
            AppMsg::ClusterLoaded(Err(error)) => {
                cluster::handle_cluster_loaded_err(self, sender, error)
            }
            AppMsg::ClusterChanged(index) => cluster::handle_cluster_changed(self, sender, index),
            AppMsg::EditCurrentCluster => cluster::handle_edit_current_cluster(self, root),
            AppMsg::RemoveClusterFromProject => {
                project::handle_remove_cluster_from_project(self, sender)
            }
            AppMsg::NamespaceChanged(index) => {
                namespace::handle_namespace_changed(self, sender, root, index)
            }
            AppMsg::CustomNamespaceEntered => {
                namespace::handle_custom_namespace_entered(self, sender)
            }
            AppMsg::RemoveCustomNamespace(namespace) => {
                namespace::handle_remove_custom_namespace(self, sender, namespace)
            }
            AppMsg::OpenRenameNamespaceDialog(namespace) => {
                namespace::handle_open_rename_namespace_dialog(self, root, namespace)
            }
            AppMsg::RenameNamespaceConfirmed => {
                namespace::handle_rename_namespace_confirmed(self, sender)
            }
            AppMsg::StatusFilterChanged(index) => {
                object_list::handle_status_filter_changed(self, index)
            }
            AppMsg::ObjectColumnToggled(index) => {
                object_list::handle_object_column_toggled(self, index)
            }
            AppMsg::ObjectColumnResized(column, width) => {
                object_list::handle_object_column_resized(self, &sender, column, width)
            }
            AppMsg::ResourceChanged(index, _section) => {
                mutations::handle_resource_changed(self, sender, index)
            }
            AppMsg::ToggleCurrentObjectFavorite => {
                object_list::handle_toggle_current_object_favorite(self, sender)
            }
            AppMsg::FavoriteObjectActivated(favorite) => {
                object_list::handle_favorite_object_activated(self, sender, favorite)
            }
            AppMsg::SearchChanged(query) => object_list::handle_search_changed(self, query),
            AppMsg::ObjectActivated(index) => {
                object_list::handle_object_activated(self, sender, index)
            }
            AppMsg::RelatedPodActivated(index) => {
                object_list::handle_related_pod_activated(self, sender, index)
            }
            AppMsg::ObjectDetailLoaded(token, Ok(detail)) => {
                mutations::handle_object_detail_loaded_ok(self, sender, token, detail)
            }
            AppMsg::ObjectDetailLoaded(token, Err(error)) => {
                mutations::handle_object_detail_loaded_err(self, token, error)
            }
            AppMsg::StartPodLogs => logs::handle_start_pod_logs(self, sender),
            AppMsg::StopPodLogs => logs::handle_stop_pod_logs(self),
            AppMsg::StartPodPortForward => {
                port_forward::handle_start_pod_port_forward(self, sender)
            }
            AppMsg::StopPodPortForward => port_forward::handle_stop_pod_port_forward(self),
            AppMsg::ClearPodLogs => logs::handle_clear_pod_logs(self),
            AppMsg::ShowPodTerminal => exec::handle_show_pod_terminal(self, sender, root),
            AppMsg::RestartPodTerminal(token) => {
                exec::handle_restart_pod_terminal(self, sender, token)
            }
            AppMsg::StopPodTerminal(token) => exec::handle_stop_pod_terminal(self, token),
            AppMsg::PodTerminalInput(token, text) => {
                exec::handle_pod_terminal_input(self, token, text)
            }
            AppMsg::ToggleDetailOverview => mutations::handle_toggle_detail_overview(self),
            AppMsg::BackToObjects => project::handle_back_to_objects(self),
            AppMsg::DetailTabChanged(name) => {
                mutations::handle_detail_tab_changed(self, sender, name)
            }
            AppMsg::ShowCreateYamlDialog => mutations::handle_show_create_yaml_dialog(self, root),
            AppMsg::CreateYaml => mutations::handle_create_yaml(self, sender),
            AppMsg::ObjectCreated(Ok(name)) => {
                mutations::handle_object_created_ok(self, sender, name)
            }
            AppMsg::ObjectCreated(Err(error)) => mutations::handle_object_created_err(self, error),
            AppMsg::ScaleDeployment => mutations::handle_scale_deployment(self, sender),
            AppMsg::ObjectScaled(token, Ok(detail)) => {
                mutations::handle_object_scaled_ok(self, token, detail)
            }
            AppMsg::ObjectScaled(token, Err(error)) => {
                mutations::handle_object_scaled_err(self, token, error)
            }
            AppMsg::ToggleNodeScheduling => nodes::handle_toggle_node_scheduling(self, sender),
            AppMsg::NodeSchedulingUpdated(token, Ok(detail)) => {
                nodes::handle_node_scheduling_updated_ok(self, token, detail)
            }
            AppMsg::NodeSchedulingUpdated(token, Err(error)) => {
                nodes::handle_node_scheduling_updated_err(self, token, error)
            }
            AppMsg::DrainNode => nodes::handle_drain_node(self, sender, root),
            AppMsg::ConfirmDrainNode => nodes::handle_confirm_drain_node(self, sender),
            AppMsg::NodeDrained(token, Ok((detail, count))) => {
                nodes::handle_node_drained_ok(self, token, detail, count)
            }
            AppMsg::NodeDrained(token, Err(error)) => {
                nodes::handle_node_drained_err(self, token, error)
            }
            AppMsg::ExplainYaml => mutations::handle_explain_yaml(self, root),
            AppMsg::ApplyYaml => mutations::handle_apply_yaml(self, sender),
            AppMsg::ObjectApplied(token, Ok(detail)) => {
                mutations::handle_object_applied_ok(self, token, detail)
            }
            AppMsg::ObjectApplied(token, Err(error)) => {
                mutations::handle_object_applied_err(self, token, error)
            }
            AppMsg::DownloadYaml => mutations::handle_download_yaml(self, sender, root),
            AppMsg::SaveYamlTo(path, yaml) => mutations::handle_save_yaml_to(self, path, yaml),
            AppMsg::DownloadLogs => logs::handle_download_logs(self, sender, root),
            AppMsg::SaveLogsTo(path, logs) => logs::handle_save_logs_to(self, path, logs),
            AppMsg::DeleteObject => mutations::handle_delete_object(self, sender, root),
            AppMsg::ConfirmDeleteObject => mutations::handle_confirm_delete_object(self, sender),
            AppMsg::ObjectDeleted(token, Ok(name)) => {
                mutations::handle_object_deleted_ok(self, sender, token, name)
            }
            AppMsg::ObjectDeleted(token, Err(error)) => {
                mutations::handle_object_deleted_err(self, token, error)
            }
            AppMsg::PodLogLine(token, line) => logs::handle_pod_log_line(self, token, line),
            AppMsg::PodLogFinished(token, result) => {
                logs::handle_pod_log_finished(self, token, result)
            }
            AppMsg::PodExecEvent(token, event) => exec::handle_pod_exec_event(self, token, event),
            AppMsg::PodExecFinished(token, result) => {
                exec::handle_pod_exec_finished(self, token, result)
            }
            AppMsg::PodPortForwardEvent(token, event) => {
                port_forward::handle_pod_port_forward_event(self, token, event)
            }
            AppMsg::PodPortForwardFinished(token, result) => {
                port_forward::handle_pod_port_forward_finished(self, token, result)
            }
            AppMsg::ShowAddClusterDialog => cluster::handle_show_add_cluster_dialog(self, root),
            AppMsg::ShowTokenForm => cluster::handle_show_token_form(self),
            AppMsg::ShowCaFile => cluster::handle_show_ca_file(self, sender, root),
            AppMsg::ShowImportFile => cluster::handle_show_import_file(self, sender, root),
            AppMsg::CaFileLoaded(Ok(data)) => cluster::handle_ca_file_loaded_ok(self, data),
            AppMsg::CaFileLoaded(Err(error)) => cluster::handle_ca_file_loaded_err(self, error),
            AppMsg::Refresh => {
                if self.resources.is_empty() {
                    self.load_cluster(sender);
                } else {
                    self.refresh_objects(sender);
                }
            }
            AppMsg::ObjectsLoaded(token, Ok(objects)) => {
                object_list::handle_objects_loaded_ok(self, sender, token, objects)
            }
            AppMsg::ObjectsLoaded(token, Err(error)) => {
                object_list::handle_objects_loaded_err(self, token, error)
            }
            AppMsg::ObjectWatchEvent(token, event) => {
                object_list::handle_object_watch_event(self, sender, token, event)
            }
            AppMsg::ObjectListRefreshTick => object_list::handle_object_list_refresh_tick(self),
            AppMsg::ProjectSaveTick => project::handle_project_save_tick(self),
            AppMsg::ObjectWatchFinished(token, result) => {
                object_list::handle_object_watch_finished(self, token, result)
            }
            AppMsg::AddCluster => cluster::handle_add_cluster(self, sender),
            AppMsg::ClusterAdded(Ok((path, context_name))) => {
                cluster::handle_cluster_added_ok(self, sender, path, context_name)
            }
            AppMsg::ClusterAdded(Err(error)) => cluster::handle_cluster_added_err(self, error),
            AppMsg::ImportKubeconfig(path) => cluster::handle_import_kubeconfig(self, sender, path),
            AppMsg::KubeconfigImported(Ok((path, context_names))) => {
                cluster::handle_kubeconfig_imported_ok(self, sender, path, context_names)
            }
            AppMsg::StateLoadedForImportedClusters(context_names, Ok(state)) => {
                self.contexts = state.contexts;
                self.namespaces = state.namespaces;
                self.projects = state.projects;
                self.projects
                    .add_contexts_to_selected_project(context_names.clone());
                self.save_projects_or_toast();
                let visible_contexts = self.visible_contexts();
                self.selected_context = visible_contexts
                    .iter()
                    .find(|context| context_names.iter().any(|name| name == &context.name))
                    .or_else(|| visible_contexts.first())
                    .map(|context| context.name.clone());
                self.selected_namespace = self.preferred_namespace_for_selected_context("default");
                self.loading = false;
                self.sync_dropdowns(Some(sender.clone()));
                self.enter_clusters_page(sender);
                self.status = tr("Kubeconfig imported.");
                self.sync_status();
            }
            AppMsg::StateLoadedForImportedClusters(_, Err(error)) => {
                self.loading = false;
                self.status = tr("Unable to reload kubeconfig.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::KubeconfigImported(Err(error)) => {
                cluster::handle_kubeconfig_imported_err(self, error)
            }
            AppMsg::Toast(text) => self.toaster.add_toast(adw::Toast::new(&text)),
        }
    }
}
