use super::commands::*;
use super::utils::*;
use super::*;

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
            AppMsg::ShowProjects => {
                self.stop_object_watch();
                self.stop_log_stream();
                self.stop_port_forward();
                self.show_object_list();
                // The store may have changed since the page was last built
                // (e.g. removing a cluster only rebuilds the clusters page),
                // so refresh the rows before presenting them.
                self.rebuild_project_list();
                self.show_projects();
                self.loading = false;
                self.status = tr("Select a project.");
                self.sync_terminal_controls();
                self.sync_port_forward_controls();
                self.sync_status();
            }
            AppMsg::ShowClusters => {
                self.stop_log_stream();
                self.stop_port_forward();
                self.show_object_list();
                self.enter_clusters_page(sender);
                self.loading = false;
                self.status = tr("Select a cluster.");
                self.sync_terminal_controls();
                self.sync_port_forward_controls();
                self.sync_status();
            }
            AppMsg::RefreshClusters => {
                self.refresh_cluster_summaries(sender);
                self.status = tr("Refreshing clusters.");
                self.sync_status();
            }
            AppMsg::ClusterSummaryLoaded(context_name, result) => {
                let state = match result {
                    Ok(summary) => ClusterSummaryState::Loaded(summary),
                    Err(error) => ClusterSummaryState::Error(error),
                };
                self.cluster_summaries.insert(context_name, state);
                self.rebuild_cluster_list();
            }
            AppMsg::ProjectChanged(index) => {
                let Some(project_name) = self
                    .projects
                    .projects
                    .get(index as usize)
                    .map(|project| project.name.clone())
                else {
                    return;
                };
                if self.projects.selected_project.as_deref() != Some(project_name.as_str()) {
                    self.projects.selected_project = Some(project_name);
                    self.save_projects_or_toast();
                }
                self.switch_to_project(sender);
            }
            AppMsg::ShowAddProjectDialog => {
                self.editing_project_name = None;
                self.project_dialog.set_title(&tr("New Project"));
                self.project_dialog_description
                    .set_label(&tr("Separate clusters by environment or company"));
                self.project_create_button.set_label(&tr("Create"));
                self.project_name_entry.set_text("");
                self.project_dialog.present(Some(root));
            }
            AppMsg::ShowRenameProjectDialog => {
                let current = self.projects.selected_project_name().to_owned();
                self.editing_project_name = Some(current.clone());
                self.project_dialog.set_title(&tr("Rename Project"));
                self.project_dialog_description
                    .set_label(&tr("Choose a new name for this project"));
                self.project_create_button.set_label(&tr("Rename"));
                self.project_name_entry.set_text(&current);
                self.project_dialog.present(Some(root));
            }
            AppMsg::AddProject => {
                let name = self.project_name_entry.text().trim().to_owned();
                if name.is_empty() {
                    return;
                }

                if let Some(original) = self.editing_project_name.clone() {
                    if name != original && self.projects.has_project(&name) {
                        self.toaster.add_toast(adw::Toast::new(&tr(
                            "A project with this name already exists.",
                        )));
                        return;
                    }
                    if let Some(project) = self
                        .projects
                        .projects
                        .iter_mut()
                        .find(|project| project.name == original)
                    {
                        project.name = name.clone();
                    }
                    if self.projects.selected_project.as_deref() == Some(original.as_str()) {
                        self.projects.selected_project = Some(name);
                    }
                    self.save_projects_or_toast();
                    self.editing_project_name = None;
                    self.project_dialog.close();
                    self.sync_dropdowns(Some(sender.clone()));
                    self.sync_status();
                    return;
                }

                if self.projects.has_project(&name) {
                    self.toaster.add_toast(adw::Toast::new(&tr(
                        "A project with this name already exists.",
                    )));
                    return;
                }

                self.projects.projects.push(Project {
                    name: name.clone(),
                    contexts: Vec::new(),
                    custom_namespaces_by_context: Vec::new(),
                });
                self.projects.selected_project = Some(name.clone());
                self.save_projects_or_toast();
                self.selected_context = None;
                self.project_dialog.close();
                self.switch_to_project(sender);
            }
            AppMsg::DuplicateProject => {
                let Some(source) = self.projects.selected_project().cloned() else {
                    return;
                };
                let mut new_name = tr_format("{name} copy", &[("{name}", source.name.clone())]);
                let mut suffix = 2;
                while self.projects.has_project(&new_name) {
                    new_name = tr_format(
                        "{name} copy {suffix}",
                        &[
                            ("{name}", source.name.clone()),
                            ("{suffix}", suffix.to_string()),
                        ],
                    );
                    suffix += 1;
                }
                self.projects.projects.push(Project {
                    name: new_name.clone(),
                    contexts: source.contexts,
                    custom_namespaces_by_context: source.custom_namespaces_by_context,
                });
                self.projects.selected_project = Some(new_name.clone());
                self.save_projects_or_toast();
                self.toaster.add_toast(adw::Toast::new(&tr_format(
                    "Duplicated as {name}",
                    &[("{name}", new_name.clone())],
                )));
                self.switch_to_project(sender);
            }
            AppMsg::DeleteProject => {
                if self.projects.projects.len() <= 1 {
                    self.toaster
                        .add_toast(adw::Toast::new(&tr("At least one project must remain.")));
                    return;
                }
                let name = self.projects.selected_project_name().to_owned();
                let dialog = adw::AlertDialog::new(
                    Some(&tr("Delete project?")),
                    Some(&tr_format(
                        "This removes \"{name}\" from Aetheris, including its saved clusters and namespaces. The clusters themselves are not affected.",
                        &[("{name}", name.clone())],
                    )),
                );
                dialog.add_responses(&[("cancel", &tr("Cancel")), ("delete", &tr("Delete"))]);
                dialog.set_close_response("cancel");
                dialog.set_default_response(Some("cancel"));
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                let sender = sender.clone();
                dialog.choose(Some(root), gtk::gio::Cancellable::NONE, move |response| {
                    if response.as_str() == "delete" {
                        sender.input(AppMsg::ConfirmDeleteProject);
                    }
                });
            }
            AppMsg::ConfirmDeleteProject => {
                let name = self.projects.selected_project_name().to_owned();
                self.projects
                    .projects
                    .retain(|project| project.name != name);
                self.projects.selected_project = self
                    .projects
                    .projects
                    .first()
                    .map(|project| project.name.clone());
                self.selected_context = None;
                self.save_projects_or_toast();
                self.stop_object_watch();
                self.stop_log_stream();
                self.stop_port_forward();
                self.sync_dropdowns(Some(sender.clone()));
                self.show_object_list();
                self.show_projects();
                self.loading = false;
                self.status = tr("Select a project.");
                self.sync_terminal_controls();
                self.sync_port_forward_controls();
                self.sync_status();
            }
            AppMsg::ClusterLoaded(Ok(state)) => {
                self.loading = false;
                if let Some(warning) = state.namespace_warning {
                    self.toaster.add_toast(adw::Toast::new(&warning));
                }
                self.namespaces = with_all_namespace(state.namespaces);
                self.resources = state.resources;
                self.selected_namespace = self.preferred_namespace_for_selected_context("all");
                self.selected_resource = select_default_resource(&self.resources);
                self.selected_resource_section = self
                    .selected_resource_kind()
                    .map(ResourceSection::for_resource)
                    .unwrap_or(ResourceSection::Workloads);
                self.sync_dropdowns(Some(sender.clone()));
                self.rebuild_resource_list(Some(sender.clone()));

                if self.selected_resource.is_some() {
                    self.refresh_objects(sender);
                } else {
                    self.status = tr("No listable Kubernetes resources found.");
                    self.sync_status();
                    self.rebuild_object_list();
                }
            }
            AppMsg::ClusterLoaded(Err(error)) => {
                self.loading = false;
                self.stop_object_watch();
                self.resources.clear();
                self.objects.clear();
                self.selected_resource = None;
                self.status = tr("Unable to discover resources.");
                self.toaster.add_toast(adw::Toast::new(&error));
                self.rebuild_resource_list(Some(sender.clone()));
                self.rebuild_object_list();
                self.sync_status();
            }
            AppMsg::ClusterChanged(index) => {
                let visible_contexts = self.visible_contexts();
                if let Some(context) = visible_contexts.get(index as usize) {
                    self.selected_context = Some(context.name.clone());
                    self.show_browser();
                    self.present_content_panel();
                    self.load_cluster(sender);
                }
            }
            AppMsg::EditCurrentCluster => {
                let Some((name, server, insecure)) = self
                    .selected_context
                    .as_deref()
                    .and_then(|selected| self.contexts.iter().find(|c| c.name == selected))
                    .map(|context| {
                        (
                            context.name.clone(),
                            context.server.clone(),
                            context.insecure_skip_tls_verify,
                        )
                    })
                else {
                    return;
                };
                self.open_cluster_edit_dialog(&name, &server, insecure, root);
            }
            AppMsg::RemoveClusterFromProject => {
                let Some(context) = self.selected_context.clone() else {
                    return;
                };
                self.projects.remove_context_from_selected_project(&context);
                self.save_projects_or_toast();
                self.selected_context = None;
                self.stop_object_watch();
                self.stop_log_stream();
                self.stop_port_forward();
                self.resources.clear();
                self.objects.clear();
                self.selected_resource = None;
                self.show_object_list();
                self.enter_clusters_page(sender);
                self.status = tr_format(
                    "Removed {context} from this project.",
                    &[("{context}", context)],
                );
                self.sync_terminal_controls();
                self.sync_port_forward_controls();
                self.sync_status();
            }
            AppMsg::NamespaceChanged(index) => {
                let choices = self.namespace_choices();
                if index as usize == choices.len() {
                    self.show_custom_namespace_dialog(root);
                    return;
                }
                if let Some(namespace) = choices.get(index as usize)
                    && self.selected_namespace != *namespace
                {
                    self.selected_namespace.clone_from(namespace);
                    self.remember_selected_namespace();
                    self.sync_dropdowns(Some(sender.clone()));
                    self.show_object_list();
                    self.stop_log_stream();
                    self.stop_port_forward();
                    self.refresh_objects(sender);
                }
            }
            AppMsg::CustomNamespaceEntered => {
                let namespace = self.custom_namespace_entry.text().trim().to_owned();
                if namespace.is_empty() {
                    return;
                }

                self.remember_namespace(&namespace);
                if self.selected_namespace != namespace {
                    self.selected_namespace = namespace;
                    self.remember_selected_namespace();
                    self.sync_dropdowns(Some(sender.clone()));
                    self.show_object_list();
                    self.stop_log_stream();
                    self.stop_port_forward();
                    self.refresh_objects(sender);
                } else {
                    self.sync_dropdowns(Some(sender.clone()));
                }

                self.custom_namespace_dialog.close();
            }
            AppMsg::RemoveCustomNamespace(namespace) => {
                let Some(context) = self.selected_context.clone() else {
                    return;
                };
                let removed = self
                    .projects
                    .selected_project_mut()
                    .is_some_and(|project| project.remove_custom_namespace(&context, &namespace));
                if !removed {
                    return;
                }
                self.save_projects_or_toast();
                if self.selected_namespace == namespace {
                    self.selected_namespace = String::from("default");
                    self.remember_selected_namespace();
                    self.sync_dropdowns(Some(sender.clone()));
                    self.show_object_list();
                    self.stop_log_stream();
                    self.stop_port_forward();
                    self.refresh_objects(sender);
                } else {
                    self.sync_dropdowns(Some(sender.clone()));
                }
                self.toaster
                    .add_toast(adw::Toast::new(&tr("Namespace removed")));
            }
            AppMsg::OpenRenameNamespaceDialog(namespace) => {
                self.open_rename_namespace_dialog(&namespace, root);
            }
            AppMsg::RenameNamespaceConfirmed => {
                let new_name = self.rename_namespace_entry.text().trim().to_owned();
                let Some(old_name) = self.renaming_namespace.take() else {
                    return;
                };
                if new_name.is_empty() || new_name == old_name {
                    self.rename_namespace_dialog.close();
                    return;
                }
                let Some(context) = self.selected_context.clone() else {
                    self.rename_namespace_dialog.close();
                    return;
                };
                let renamed = self.projects.selected_project_mut().is_some_and(|project| {
                    project.rename_custom_namespace(&context, &old_name, &new_name)
                });
                if renamed {
                    self.save_projects_or_toast();
                    if self.selected_namespace == old_name {
                        self.selected_namespace = new_name;
                        self.remember_selected_namespace();
                        self.sync_dropdowns(Some(sender.clone()));
                        self.show_object_list();
                        self.stop_log_stream();
                        self.stop_port_forward();
                        self.refresh_objects(sender);
                    } else {
                        self.sync_dropdowns(Some(sender.clone()));
                    }
                    self.toaster
                        .add_toast(adw::Toast::new(&tr("Namespace renamed")));
                } else {
                    self.toaster.add_toast(adw::Toast::new(&tr(
                        "A namespace with this name already exists.",
                    )));
                }
                self.rename_namespace_dialog.close();
            }
            AppMsg::StatusFilterChanged(index) => {
                let Some(filter) = StatusFilter::ALL.get(index as usize).copied() else {
                    return;
                };
                if self.selected_status_filters.contains(&filter) {
                    self.selected_status_filters.remove(&filter);
                } else {
                    self.selected_status_filters.insert(filter);
                }
                self.sync_status();
                self.sync_status_filter();
                self.rebuild_object_list();
            }
            AppMsg::ObjectColumnToggled(index) => {
                let Some(column) = self.offerable_object_columns().get(index as usize).copied()
                else {
                    return;
                };
                let visible = !self.projects.visible_object_columns.contains(&column);
                self.projects.set_object_column_visible(column, visible);
                self.save_projects_or_toast();
                self.sync_object_columns();
            }
            AppMsg::ObjectColumnResized(column, width) => {
                if self.projects.set_object_table_column_width(column, width) {
                    self.schedule_project_save(&sender);
                }
            }
            AppMsg::ResourceChanged(index) => {
                if self.resources.get(index).is_some() {
                    let next = Some(index);
                    if self.selected_resource != next {
                        self.selected_resource = next;
                        self.selected_resource_section =
                            ResourceSection::for_resource(&self.resources[index]);
                        self.rebuild_resource_list(Some(sender.clone()));
                        self.present_content_panel();
                        self.show_object_list();
                        self.sync_object_columns();
                        self.stop_log_stream();
                        self.stop_port_forward();
                        self.refresh_objects(sender);
                    }
                }
            }
            AppMsg::SearchChanged(query) => {
                self.search_query = query;
                self.sync_status();
                self.rebuild_object_list();
            }
            AppMsg::ObjectActivated(index) => {
                let Some((context, resource, namespace, name)) = self.detail_request(index) else {
                    return;
                };
                self.open_object_detail(context, resource, namespace, name, sender);
            }
            AppMsg::RelatedPodActivated(index) => {
                let Some(pod) = self.related_pod_at(index) else {
                    return;
                };
                let Some(target) = self.detail.target.clone() else {
                    return;
                };
                let namespace = Some(pod.namespace.clone());
                self.open_object_detail(
                    target.context,
                    pod_resource_kind(),
                    namespace,
                    pod.name,
                    sender,
                );
            }
            AppMsg::ObjectDetailLoaded(token, Ok(detail)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.status = tr_format(
                    "Showing details for {name}",
                    &[("{name}", detail.name.clone())],
                );
                self.populate_detail_dialog(&detail);
                self.update_log_target_containers(&detail);
                self.update_exec_target_containers(&detail);
                self.show_detail_page(&detail.name);
                self.maybe_start_visible_logs(sender);
                self.sync_status();
            }
            AppMsg::ObjectDetailLoaded(token, Err(error)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.detail.target = None;
                self.detail.log_target = None;
                self.detail.exec_target = None;
                self.detail.port_forward_target = None;
                self.sync_log_controls();
                self.sync_terminal_controls();
                self.sync_port_forward_controls();
                self.status = tr("Unable to load object detail.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::StartPodLogs => {
                self.start_pod_logs(sender);
            }
            AppMsg::StopPodLogs => {
                self.stop_log_stream();
                self.sync_log_controls();
            }
            AppMsg::StartPodPortForward => {
                self.start_pod_port_forward(sender);
            }
            AppMsg::StopPodPortForward => {
                self.stop_port_forward();
                self.sync_port_forward_controls();
            }
            AppMsg::ClearPodLogs => {
                self.detail.log_buffer.set_text("");
            }
            AppMsg::ShowPodTerminal => {
                self.show_pod_terminal(root, sender);
            }
            AppMsg::RestartPodTerminal(token) => {
                self.start_terminal_session(token, sender);
            }
            AppMsg::StopPodTerminal(token) => {
                self.close_terminal_session(token, false);
            }
            AppMsg::PodTerminalInput(token, text) => {
                self.send_terminal_input(token, text);
            }
            AppMsg::ToggleDetailOverview => {
                let collapsed = self.detail.overview_section.is_visible();
                self.detail.overview_section.set_visible(!collapsed);
                self.detail.expand_logs_button.set_icon_name(if collapsed {
                    "view-restore-symbolic"
                } else {
                    "view-fullscreen-symbolic"
                });
                let tooltip = if collapsed {
                    tr("Show summary")
                } else {
                    tr("Hide summary to see more of this tab")
                };
                self.detail
                    .expand_logs_button
                    .set_tooltip_text(Some(&tooltip));
            }
            AppMsg::BackToObjects => {
                self.show_object_list();
                self.stop_log_stream();
                self.stop_port_forward();
                self.sync_log_controls();
                self.sync_terminal_controls();
                self.sync_port_forward_controls();
            }
            AppMsg::DetailTabChanged(name) => {
                if name == "logs" {
                    self.maybe_start_visible_logs(sender);
                }
            }
            AppMsg::ShowCreateYamlDialog => {
                if self.selected_resource_kind().is_none() {
                    self.toaster.add_toast(adw::Toast::new(&tr(
                        "Select a resource before creating YAML.",
                    )));
                    return;
                }
                self.create_yaml_dialog.present(Some(root));
            }
            AppMsg::CreateYaml => {
                let Some(context) = self.selected_context.clone() else {
                    return;
                };
                let Some(resource) = self.selected_resource_kind().cloned() else {
                    return;
                };
                let namespace = resource
                    .is_namespaced()
                    .then(|| self.selected_namespace.clone());
                let yaml = text_buffer_text(&self.create_yaml_buffer);
                self.loading = true;
                self.status = tr_format(
                    "Creating {resource}...",
                    &[("{resource}", resource.label())],
                );
                self.sync_status();
                sender.oneshot_command(async move {
                    create_object_yaml(context, resource, namespace, yaml).await
                });
            }
            AppMsg::ObjectCreated(Ok(name)) => {
                self.loading = false;
                self.create_yaml_dialog.close();
                self.create_yaml_buffer.set_text("");
                self.toaster.add_toast(adw::Toast::new(&tr_format(
                    "Created {name}.",
                    &[("{name}", name)],
                )));
                self.refresh_objects(sender);
            }
            AppMsg::ObjectCreated(Err(error)) => {
                self.loading = false;
                self.status = tr("Unable to create object.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::ScaleDeployment => {
                let Some(target) = self.detail.target.clone() else {
                    return;
                };
                if !is_deployment_resource(&target.resource) {
                    return;
                }
                let replicas = self.detail.scale_spin.value_as_int();
                self.detail.request_token = self.detail.request_token.saturating_add(1);
                let token = self.detail.request_token;
                self.loading = true;
                self.status = tr_format("Scaling {name}...", &[("{name}", target.name.clone())]);
                self.sync_status();
                sender.oneshot_command(
                    async move { scale_deployment(token, target, replicas).await },
                );
            }
            AppMsg::ObjectScaled(token, Ok(detail)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.status = tr_format("Scaled {name}", &[("{name}", detail.name.clone())]);
                self.populate_detail_dialog(&detail);
                self.update_log_target_containers(&detail);
                self.update_exec_target_containers(&detail);
                self.sync_status();
                self.toaster
                    .add_toast(adw::Toast::new(&tr("Deployment scaled.")));
            }
            AppMsg::ObjectScaled(token, Err(error)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.status = tr("Unable to scale deployment.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::ToggleNodeScheduling => {
                let Some(target) = self.detail.target.clone() else {
                    return;
                };
                if !is_node_resource(&target.resource) {
                    return;
                }
                let unschedulable = !self.detail.node_unschedulable.unwrap_or(false);
                self.detail.request_token = self.detail.request_token.saturating_add(1);
                let token = self.detail.request_token;
                self.loading = true;
                self.status = tr_format("Updating {name}...", &[("{name}", target.name.clone())]);
                self.sync_status();
                sender.oneshot_command(async move {
                    set_node_unschedulable(token, target, unschedulable).await
                });
            }
            AppMsg::NodeSchedulingUpdated(token, Ok(detail)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.status = tr_format("Updated {name}", &[("{name}", detail.name.clone())]);
                self.populate_detail_dialog(&detail);
                self.update_log_target_containers(&detail);
                self.update_exec_target_containers(&detail);
                self.sync_status();
                self.toaster
                    .add_toast(adw::Toast::new(&tr("Node scheduling updated.")));
            }
            AppMsg::NodeSchedulingUpdated(token, Err(error)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.status = tr("Unable to update node scheduling.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::DrainNode => {
                let Some(target) = self.detail.target.clone() else {
                    return;
                };
                if !is_node_resource(&target.resource) {
                    return;
                }
                let dialog = adw::AlertDialog::new(
                    Some(&tr("Drain node?")),
                    Some(&tr_format(
                        "This will remove eligible Pods from node {name}. DaemonSet, mirror and completed Pods are skipped.",
                        &[("{name}", target.name)],
                    )),
                );
                dialog.add_responses(&[("cancel", &tr("Cancel")), ("drain", &tr("Drain"))]);
                dialog.set_close_response("cancel");
                dialog.set_default_response(Some("cancel"));
                dialog.set_response_appearance("drain", adw::ResponseAppearance::Destructive);
                let sender = sender.clone();
                dialog.choose(Some(root), gtk::gio::Cancellable::NONE, move |response| {
                    if response.as_str() == "drain" {
                        sender.input(AppMsg::ConfirmDrainNode);
                    }
                });
            }
            AppMsg::ConfirmDrainNode => {
                let Some(target) = self.detail.target.clone() else {
                    return;
                };
                if !is_node_resource(&target.resource) {
                    return;
                }
                self.detail.request_token = self.detail.request_token.saturating_add(1);
                let token = self.detail.request_token;
                self.loading = true;
                self.status = tr_format("Draining {name}...", &[("{name}", target.name.clone())]);
                self.sync_status();
                sender.oneshot_command(async move { drain_node(token, target).await });
            }
            AppMsg::NodeDrained(token, Ok((detail, count))) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.status = tr_format("Drained {name}", &[("{name}", detail.name.clone())]);
                self.populate_detail_dialog(&detail);
                self.update_log_target_containers(&detail);
                self.update_exec_target_containers(&detail);
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&tr_format(
                    "Drain started for {count} Pods.",
                    &[("{count}", count.to_string())],
                )));
            }
            AppMsg::NodeDrained(token, Err(error)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.status = tr("Unable to drain node.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::ExplainYaml => {
                self.show_yaml_explanation(root);
            }
            AppMsg::ApplyYaml => {
                let Some(target) = self.detail.target.clone() else {
                    return;
                };
                let yaml = text_buffer_text(&self.detail.yaml_buffer);
                self.detail.request_token = self.detail.request_token.saturating_add(1);
                let token = self.detail.request_token;
                self.loading = true;
                self.status = tr_format("Applying {name}...", &[("{name}", target.name.clone())]);
                self.sync_status();
                sender.oneshot_command(async move { apply_object_yaml(token, target, yaml).await });
            }
            AppMsg::ObjectApplied(token, Ok(detail)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.status = tr_format("Applied {name}", &[("{name}", detail.name.clone())]);
                self.populate_detail_dialog(&detail);
                self.update_log_target_containers(&detail);
                self.update_exec_target_containers(&detail);
                self.sync_status();
                self.toaster
                    .add_toast(adw::Toast::new(&tr("YAML applied.")));
            }
            AppMsg::ObjectApplied(token, Err(error)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.status = tr("Unable to apply YAML.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::DownloadYaml => {
                let yaml = text_buffer_text(&self.detail.yaml_buffer);
                let name = self
                    .detail
                    .target
                    .as_ref()
                    .map(|target| format!("{}.yaml", target.name))
                    .unwrap_or_else(|| String::from("object.yaml"));
                let dialog = gtk::FileDialog::builder()
                    .title(tr("Save YAML"))
                    .accept_label(tr("Save"))
                    .initial_name(name)
                    .modal(true)
                    .build();
                let sender = sender.clone();
                dialog.save(
                    Some(root),
                    gtk::gio::Cancellable::NONE,
                    move |result| match result {
                        Ok(file) => {
                            if let Some(path) = file.path() {
                                sender.input(AppMsg::SaveYamlTo(path, yaml.clone()));
                            } else {
                                sender.input(AppMsg::Toast(tr(
                                    "Selected destination is not available on the local filesystem.",
                                )));
                            }
                        }
                        Err(error) => {
                            if !error.matches(gtk::gio::IOErrorEnum::Cancelled) {
                                sender.input(AppMsg::Toast(error.to_string()));
                            }
                        }
                    },
                );
            }
            AppMsg::SaveYamlTo(path, yaml) => match fs::write(&path, yaml) {
                Ok(()) => self.toaster.add_toast(adw::Toast::new(&tr_format(
                    "Saved {path}.",
                    &[("{path}", path.display().to_string())],
                ))),
                Err(error) => self.toaster.add_toast(adw::Toast::new(&tr_format(
                    "Unable to save {path}: {error}",
                    &[
                        ("{path}", path.display().to_string()),
                        ("{error}", error.to_string()),
                    ],
                ))),
            },
            AppMsg::DownloadLogs => {
                let logs = text_buffer_text(&self.detail.log_buffer);
                let name = self
                    .detail
                    .target
                    .as_ref()
                    .map(|target| format!("{}.log", target.name))
                    .unwrap_or_else(|| String::from("pod.log"));
                let dialog = gtk::FileDialog::builder()
                    .title(tr("Save Logs"))
                    .accept_label(tr("Save"))
                    .initial_name(name)
                    .modal(true)
                    .build();
                let sender = sender.clone();
                dialog.save(
                    Some(root),
                    gtk::gio::Cancellable::NONE,
                    move |result| match result {
                        Ok(file) => {
                            if let Some(path) = file.path() {
                                sender.input(AppMsg::SaveLogsTo(path, logs.clone()));
                            } else {
                                sender.input(AppMsg::Toast(tr(
                                    "Selected destination is not available on the local filesystem.",
                                )));
                            }
                        }
                        Err(error) => {
                            if !error.matches(gtk::gio::IOErrorEnum::Cancelled) {
                                sender.input(AppMsg::Toast(error.to_string()));
                            }
                        }
                    },
                );
            }
            AppMsg::SaveLogsTo(path, logs) => match fs::write(&path, logs) {
                Ok(()) => self.toaster.add_toast(adw::Toast::new(&tr_format(
                    "Saved {path}.",
                    &[("{path}", path.display().to_string())],
                ))),
                Err(error) => self.toaster.add_toast(adw::Toast::new(&tr_format(
                    "Unable to save {path}: {error}",
                    &[
                        ("{path}", path.display().to_string()),
                        ("{error}", error.to_string()),
                    ],
                ))),
            },
            AppMsg::DeleteObject => {
                let Some(target) = self.detail.target.clone() else {
                    return;
                };
                let dialog = adw::AlertDialog::new(
                    Some(&tr("Delete object?")),
                    Some(&tr_format(
                        "This will delete {kind} {name}.",
                        &[("{kind}", target.resource.kind), ("{name}", target.name)],
                    )),
                );
                dialog.add_responses(&[("cancel", &tr("Cancel")), ("delete", &tr("Delete"))]);
                dialog.set_close_response("cancel");
                dialog.set_default_response(Some("cancel"));
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                let sender = sender.clone();
                dialog.choose(Some(root), gtk::gio::Cancellable::NONE, move |response| {
                    if response.as_str() == "delete" {
                        sender.input(AppMsg::ConfirmDeleteObject);
                    }
                });
            }
            AppMsg::ConfirmDeleteObject => {
                let Some(target) = self.detail.target.clone() else {
                    return;
                };
                self.detail.request_token = self.detail.request_token.saturating_add(1);
                let token = self.detail.request_token;
                self.loading = true;
                self.status = tr_format("Deleting {name}...", &[("{name}", target.name.clone())]);
                self.sync_status();
                sender.oneshot_command(async move { delete_object(token, target).await });
            }
            AppMsg::ObjectDeleted(token, Ok(name)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.detail.target = None;
                self.detail.log_target = None;
                self.detail.exec_target = None;
                self.detail.port_forward_target = None;
                self.stop_log_stream();
                self.stop_port_forward();
                self.show_object_list();
                self.sync_log_controls();
                self.sync_terminal_controls();
                self.sync_port_forward_controls();
                self.toaster.add_toast(adw::Toast::new(&tr_format(
                    "Deleted {name}.",
                    &[("{name}", name)],
                )));
                self.refresh_objects(sender);
            }
            AppMsg::ObjectDeleted(token, Err(error)) => {
                if token != self.detail.request_token {
                    return;
                }
                self.loading = false;
                self.status = tr("Unable to delete object.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::PodLogLine(token, line) => {
                if token == self.log_stream_token {
                    self.append_log_line(&line);
                }
            }
            AppMsg::PodLogFinished(token, result) => {
                if token == self.log_stream_token {
                    self.log_streaming = false;
                    self.log_abort_handle = None;
                    self.sync_log_controls();
                    if let Err(error) = result {
                        self.toaster.add_toast(adw::Toast::new(&error));
                    }
                }
            }
            AppMsg::PodExecEvent(token, event) => {
                self.feed_terminal_event(token, event);
            }
            AppMsg::PodExecFinished(token, result) => {
                self.finish_terminal_session(token);
                if let Err(error) = result {
                    let message = terminal_error_message(&error);
                    self.show_terminal_error(token, &error);
                    self.toaster.add_toast(adw::Toast::new(&message));
                }
            }
            AppMsg::PodPortForwardEvent(token, event) => {
                if token == self.port_forward_token {
                    self.handle_port_forward_event(event);
                }
            }
            AppMsg::PodPortForwardFinished(token, result) => {
                if token == self.port_forward_token {
                    self.port_forwarding = false;
                    self.port_forward_abort_handle = None;
                    self.sync_port_forward_controls();
                    if let Err(error) = result {
                        self.detail
                            .port_status_label
                            .set_label(&tr("Port-forward stopped."));
                        self.toaster.add_toast(adw::Toast::new(&error));
                    }
                }
            }
            AppMsg::ShowAddClusterDialog => {
                self.reset_cluster_dialog_form();
                self.set_cluster_dialog_editing(false);
                self.cluster_dialog_stack.set_visible_child_name("options");
                self.cluster_dialog.present(Some(root));
            }
            AppMsg::ShowTokenForm => {
                self.set_cluster_dialog_editing(false);
                self.cluster_dialog_stack.set_visible_child_name("token");
            }
            AppMsg::ShowCaFile => {
                let dialog = gtk::FileDialog::builder()
                    .title(tr("Choose CA Certificate"))
                    .accept_label(tr("Choose"))
                    .modal(true)
                    .build();
                let sender = sender.clone();
                dialog.open(
                    Some(root),
                    gtk::gio::Cancellable::NONE,
                    move |result| match result {
                        Ok(file) => sender.input(read_ca_file(file)),
                        Err(error) => {
                            if !error.matches(gtk::gio::IOErrorEnum::Cancelled) {
                                sender.input(AppMsg::Toast(error.to_string()));
                            }
                        }
                    },
                );
            }
            AppMsg::ShowImportFile => {
                let dialog = gtk::FileDialog::builder()
                    .title(tr("Import Kubeconfig"))
                    .accept_label(tr("Import"))
                    .modal(true)
                    .build();
                let sender = sender.clone();
                dialog.open(
                    Some(root),
                    gtk::gio::Cancellable::NONE,
                    move |result| match result {
                        Ok(file) => {
                            if let Some(path) = file.path() {
                                sender.input(AppMsg::ImportKubeconfig(path));
                            } else {
                                sender.input(AppMsg::Toast(tr(
                                    "Selected file is not available on the local filesystem.",
                                )));
                            }
                        }
                        Err(error) => {
                            if !error.matches(gtk::gio::IOErrorEnum::Cancelled) {
                                sender.input(AppMsg::Toast(error.to_string()));
                            }
                        }
                    },
                );
            }
            AppMsg::CaFileLoaded(Ok(data)) => {
                self.setup_ca_entry.set_text(data.trim());
                self.setup_insecure_check.set_active(false);
            }
            AppMsg::CaFileLoaded(Err(error)) => {
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::Refresh => {
                if self.resources.is_empty() {
                    self.load_cluster(sender);
                } else {
                    self.refresh_objects(sender);
                }
            }
            AppMsg::ObjectsLoaded(Ok(objects)) => {
                self.loading = false;
                let count = objects.len();
                self.objects = objects;
                self.set_object_status(count);
                self.sync_status();
                self.rebuild_object_list();
                self.start_object_watch(sender);
            }
            AppMsg::ObjectsLoaded(Err(error)) => {
                self.loading = false;
                self.stop_object_watch();
                self.objects.clear();
                self.status = tr("Unable to list selected resource.");
                self.sync_status();
                self.rebuild_object_list();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::ObjectWatchEvent(token, event) => {
                if token != self.object_watch_token || self.loading {
                    return;
                }
                match event {
                    ObjectWatchEvent::Restarted(objects) => {
                        self.objects = objects;
                        self.schedule_object_list_refresh(&sender);
                    }
                    ObjectWatchEvent::Applied(object) => {
                        self.upsert_object(object);
                        self.schedule_object_list_refresh(&sender);
                    }
                    ObjectWatchEvent::Deleted(object) => {
                        self.remove_object(&object);
                        self.schedule_object_list_refresh(&sender);
                    }
                    ObjectWatchEvent::Error(error) => {
                        self.status =
                            tr_format("Live watch reconnecting: {error}", &[("{error}", error)]);
                        self.sync_status();
                    }
                }
            }
            AppMsg::ObjectListRefreshTick => {
                self.flush_object_list_refresh();
            }
            AppMsg::ProjectSaveTick => {
                self.project_save_scheduled = false;
                self.save_projects_or_toast();
            }
            AppMsg::ObjectWatchFinished(token, result) => {
                if token != self.object_watch_token {
                    return;
                }
                self.object_watch_abort_handle = None;
                if let Err(error) = result {
                    self.status = tr_format("Live watch stopped: {error}", &[("{error}", error)]);
                    self.sync_status();
                }
            }
            AppMsg::AddCluster => {
                let request = AddClusterRequest {
                    context_name: self.setup_name_entry.text().to_string(),
                    server: self.setup_server_entry.text().to_string(),
                    bearer_token: self.setup_token_entry.text().to_string(),
                    certificate_authority_data: Some(self.setup_ca_entry.text().to_string()),
                    insecure_skip_tls_verify: self.setup_insecure_check.is_active(),
                    original_context_name: self.editing_context_name.clone(),
                };
                self.loading = true;
                self.status = if self.editing_cluster {
                    tr("Saving cluster...")
                } else {
                    tr("Adding cluster...")
                };
                self.setup_button.set_sensitive(false);
                self.sync_status();
                sender.oneshot_command(async move { add_cluster(request).await });
            }
            AppMsg::ClusterAdded(Ok((path, context_name))) => {
                self.loading = true;
                self.status = tr("Loading kubeconfig...");
                self.setup_button.set_sensitive(true);
                self.setup_token_entry.set_text("");
                self.editing_context_name = None;
                self.cluster_dialog.close();
                self.toaster.add_toast(adw::Toast::new(&tr_format(
                    "Cluster saved to {path}",
                    &[("{path}", path)],
                )));
                sender.oneshot_command(async move { load_state_for_cluster(context_name).await });
            }
            AppMsg::ClusterAdded(Err(error)) => {
                self.loading = false;
                self.setup_button.set_sensitive(true);
                self.status = tr("Unable to add cluster.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::ImportKubeconfig(path) => {
                self.loading = true;
                self.status = tr("Importing kubeconfig...");
                self.sync_status();
                sender.oneshot_command(async move { import_kubeconfig(path).await });
            }
            AppMsg::KubeconfigImported(Ok((path, context_names))) => {
                self.loading = true;
                self.status = tr("Loading kubeconfig...");
                self.cluster_dialog.close();
                self.toaster.add_toast(adw::Toast::new(&tr_format(
                    "Kubeconfig imported to {path}",
                    &[("{path}", path)],
                )));
                sender.oneshot_command(async move {
                    load_state_for_imported_clusters(context_names).await
                });
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
                self.loading = false;
                self.status = tr("Unable to import kubeconfig.");
                self.sync_status();
                self.toaster.add_toast(adw::Toast::new(&error));
            }
            AppMsg::Toast(text) => self.toaster.add_toast(adw::Toast::new(&text)),
        }
    }
}

fn read_ca_file(file: gtk::gio::File) -> AppMsg {
    let Some(path) = file.path() else {
        return AppMsg::Toast(tr(
            "Selected file is not available on the local filesystem.",
        ));
    };

    let result = fs::read_to_string(&path).map_err(|error| {
        tr_format(
            "Unable to read {path}: {error}",
            &[
                ("{path}", path.display().to_string()),
                ("{error}", error.to_string()),
            ],
        )
    });
    AppMsg::CaFileLoaded(result)
}
