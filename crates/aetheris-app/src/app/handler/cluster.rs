use super::super::commands::*;
use super::super::utils::*;
use super::super::*;

pub(super) fn handle_add_cluster(app: &mut App, sender: ComponentSender<App>) {
    let request = AddClusterRequest {
        context_name: app.setup_name_entry.text().to_string(),
        server: app.setup_server_entry.text().to_string(),
        bearer_token: app.setup_token_entry.text().to_string(),
        certificate_authority_data: Some(app.setup_ca_entry.text().to_string()),
        insecure_skip_tls_verify: app.setup_insecure_check.is_active(),
        original_context_name: app.editing_context_name.clone(),
    };
    app.loading = true;
    app.status = if app.editing_cluster {
        tr("Saving cluster...")
    } else {
        tr("Adding cluster...")
    };
    app.setup_button.set_sensitive(false);
    app.sync_status();
    sender.oneshot_command(async move { add_cluster(request).await });
}

pub(super) fn handle_cluster_added_ok(
    app: &mut App,
    sender: ComponentSender<App>,
    path: String,
    context_name: String,
) {
    app.loading = true;
    app.status = tr("Loading kubeconfig...");
    app.setup_button.set_sensitive(true);
    app.setup_token_entry.set_text("");
    app.editing_context_name = None;
    app.cluster_dialog.close();
    app.toaster.add_toast(adw::Toast::new(&tr_format(
        "Cluster saved to {path}",
        &[("{path}", path)],
    )));
    sender.oneshot_command(async move { load_state_for_cluster(context_name).await });
}

pub(super) fn handle_cluster_added_err(app: &mut App, error: String) {
    app.loading = false;
    app.setup_button.set_sensitive(true);
    app.status = tr("Unable to add cluster.");
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&error));
}

pub(super) fn handle_cluster_changed(app: &mut App, sender: ComponentSender<App>, index: u32) {
    let visible_contexts = app.visible_contexts();
    if let Some(context) = visible_contexts.get(index as usize) {
        app.selected_context = Some(context.name.clone());
        app.show_browser();
        app.present_content_panel();
        app.load_cluster(sender);
    }
}

pub(super) fn handle_cluster_loaded_ok(
    app: &mut App,
    sender: ComponentSender<App>,
    state: ClusterState,
) {
    app.loading = false;
    if let Some(warning) = state.namespace_warning {
        app.toaster.add_toast(adw::Toast::new(&warning));
    }
    app.namespaces = with_all_namespace(state.namespaces);
    app.resources = state.resources;
    app.selected_namespace = app.preferred_namespace_for_selected_context("all");
    app.selected_resource = select_default_resource(&app.resources);
    app.selected_resource_section = app
        .selected_resource_kind()
        .map(ResourceSection::for_resource)
        .unwrap_or(ResourceSection::Workloads);
    app.sync_dropdowns(Some(sender.clone()));
    app.rebuild_resource_list(Some(sender.clone()));

    if app.selected_resource.is_some() {
        app.refresh_objects(sender);
    } else {
        app.status = tr("No listable Kubernetes resources found.");
        app.sync_status();
        app.rebuild_object_list();
    }
}

pub(super) fn handle_cluster_loaded_err(
    app: &mut App,
    sender: ComponentSender<App>,
    error: String,
) {
    app.loading = false;
    app.stop_object_watch();
    app.resources.clear();
    app.objects.clear();
    app.selected_resource = None;
    app.status = tr("Unable to discover resources.");
    app.toaster.add_toast(adw::Toast::new(&error));
    app.rebuild_resource_list(Some(sender.clone()));
    app.rebuild_object_list();
    app.sync_status();
}

pub(super) fn handle_cluster_summary_loaded(
    app: &mut App,
    context_name: String,
    result: Result<ClusterSummary, String>,
) {
    let state = match result {
        Ok(summary) => ClusterSummaryState::Loaded(summary),
        Err(error) => ClusterSummaryState::Error(error),
    };
    app.cluster_summaries.insert(context_name, state);
    app.rebuild_cluster_list();
}

pub(super) fn handle_edit_current_cluster(app: &mut App, root: &<App as Component>::Root) {
    let Some((name, server, insecure)) = app
        .selected_context
        .as_deref()
        .and_then(|selected| app.contexts.iter().find(|c| c.name == selected))
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
    app.open_cluster_edit_dialog(&name, &server, insecure, root);
}

pub(super) fn handle_refresh_clusters(app: &mut App, sender: ComponentSender<App>) {
    app.refresh_cluster_summaries(sender);
    app.status = tr("Refreshing clusters.");
    app.sync_status();
}

pub(super) fn handle_show_add_cluster_dialog(app: &mut App, root: &<App as Component>::Root) {
    app.reset_cluster_dialog_form();
    app.set_cluster_dialog_editing(false);
    app.cluster_dialog_stack.set_visible_child_name("options");
    app.cluster_dialog.present(Some(root));
}

pub(super) fn handle_show_clusters(app: &mut App, sender: ComponentSender<App>) {
    app.stop_log_stream();
    app.stop_port_forward();
    app.show_object_list();
    app.enter_clusters_page(sender);
    app.loading = false;
    app.status = tr("Select a cluster.");
    app.sync_terminal_controls();
    app.sync_port_forward_controls();
    app.sync_status();
}

pub(super) fn handle_show_ca_file(
    _app: &mut App,
    sender: ComponentSender<App>,
    root: &<App as Component>::Root,
) {
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

pub(super) fn handle_ca_file_loaded_ok(app: &mut App, data: String) {
    app.setup_ca_entry.set_text(data.trim());
    app.setup_insecure_check.set_active(false);
}

pub(super) fn handle_ca_file_loaded_err(app: &mut App, error: String) {
    app.toaster.add_toast(adw::Toast::new(&error));
}

pub(super) fn handle_show_token_form(app: &mut App) {
    app.set_cluster_dialog_editing(false);
    app.cluster_dialog_stack.set_visible_child_name("token");
}

pub(super) fn handle_import_kubeconfig(app: &mut App, sender: ComponentSender<App>, path: PathBuf) {
    app.loading = true;
    app.status = tr("Importing kubeconfig...");
    app.sync_status();
    sender.oneshot_command(async move { import_kubeconfig(path).await });
}

pub(super) fn handle_kubeconfig_imported_ok(
    app: &mut App,
    sender: ComponentSender<App>,
    path: String,
    context_names: Vec<String>,
) {
    app.loading = true;
    app.status = tr("Loading kubeconfig...");
    app.cluster_dialog.close();
    app.toaster.add_toast(adw::Toast::new(&tr_format(
        "Kubeconfig imported to {path}",
        &[("{path}", path)],
    )));
    sender.oneshot_command(async move { load_state_for_imported_clusters(context_names).await });
}

pub(super) fn handle_kubeconfig_imported_err(app: &mut App, error: String) {
    app.loading = false;
    app.status = tr("Unable to import kubeconfig.");
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&error));
}

pub(super) fn handle_show_import_file(
    _app: &mut App,
    sender: ComponentSender<App>,
    root: &<App as Component>::Root,
) {
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
