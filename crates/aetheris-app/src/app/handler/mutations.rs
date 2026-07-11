use super::super::commands::*;
use super::super::utils::*;
use super::super::*;

pub(super) fn handle_object_detail_loaded_ok(
    app: &mut App,
    sender: ComponentSender<App>,
    token: u64,
    detail: ObjectDetail,
) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.status = tr_format(
        "Showing details for {name}",
        &[("{name}", detail.name.clone())],
    );
    app.populate_detail_dialog(&detail);
    app.update_log_target_containers(&detail);
    app.update_exec_target_containers(&detail);
    app.show_detail_page(&detail.name);
    app.maybe_start_visible_logs(sender);
    app.sync_status();
}

pub(super) fn handle_object_detail_loaded_err(app: &mut App, token: u64, error: String) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.detail.log_target = None;
    app.detail.exec_target = None;
    app.detail.port_forward_target = None;
    app.sync_log_controls();
    // Keep `target` set (rather than clearing it) and still open
    // the detail page: the object is gone, but this is the only
    // place with a favorite/star button, so a stale favorite
    // must be reachable here to unfavorite it.
    if let Some(target) = app.detail.target.clone() {
        let placeholder = unavailable_object_detail(&target);
        app.populate_detail_dialog(&placeholder);
        app.show_detail_page(&placeholder.name);
    }
    app.status = tr("Unable to load object detail.");
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&error));
}

pub(super) fn handle_toggle_detail_overview(app: &mut App) {
    let collapsed = app.detail.overview_section.is_visible();
    app.detail.overview_section.set_visible(!collapsed);
    app.detail.expand_logs_button.set_icon_name(if collapsed {
        "view-restore-symbolic"
    } else {
        "view-fullscreen-symbolic"
    });
    let tooltip = if collapsed {
        tr("Show summary")
    } else {
        tr("Hide summary to see more of this tab")
    };
    app.detail
        .expand_logs_button
        .set_tooltip_text(Some(&tooltip));
}

pub(super) fn handle_detail_tab_changed(app: &mut App, sender: ComponentSender<App>, name: String) {
    if name == "logs" {
        app.maybe_start_visible_logs(sender);
    }
}

pub(super) fn handle_show_create_yaml_dialog(app: &mut App, root: &<App as Component>::Root) {
    if app.selected_resource_kind().is_none() {
        app.toaster.add_toast(adw::Toast::new(&tr(
            "Select a resource before creating YAML.",
        )));
        return;
    }
    app.create_yaml_dialog.present(Some(root));
}

pub(super) fn handle_create_yaml(app: &mut App, sender: ComponentSender<App>) {
    let Some(context) = app.selected_context.clone() else {
        return;
    };
    let Some(resource) = app.selected_resource_kind().cloned() else {
        return;
    };
    let namespace = resource
        .is_namespaced()
        .then(|| app.selected_namespace.clone());
    let yaml = text_buffer_text(&app.create_yaml_buffer);
    app.loading = true;
    app.status = tr_format(
        "Creating {resource}...",
        &[("{resource}", resource.label())],
    );
    app.sync_status();
    sender.oneshot_command(
        async move { create_object_yaml(context, resource, namespace, yaml).await },
    );
}

pub(super) fn handle_object_created_ok(app: &mut App, sender: ComponentSender<App>, name: String) {
    app.loading = false;
    app.clear_object_cache();
    app.create_yaml_dialog.close();
    app.create_yaml_buffer.set_text("");
    app.toaster.add_toast(adw::Toast::new(&tr_format(
        "Created {name}.",
        &[("{name}", name)],
    )));
    app.refresh_objects(sender);
}

pub(super) fn handle_object_created_err(app: &mut App, error: String) {
    app.loading = false;
    app.status = tr("Unable to create object.");
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&error));
}

pub(super) fn handle_scale_deployment(app: &mut App, sender: ComponentSender<App>) {
    let Some(target) = app.detail.target.clone() else {
        return;
    };
    if !is_deployment_resource(&target.resource) {
        return;
    }
    let replicas = app.detail.scale_spin.value_as_int();
    app.detail.request_token = app.detail.request_token.saturating_add(1);
    let token = app.detail.request_token;
    app.loading = true;
    app.status = tr_format("Scaling {name}...", &[("{name}", target.name.clone())]);
    app.sync_status();
    sender.oneshot_command(async move { scale_deployment(token, target, replicas).await });
}

pub(super) fn handle_object_scaled_ok(app: &mut App, token: u64, detail: ObjectDetail) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.clear_object_cache();
    app.status = tr_format("Scaled {name}", &[("{name}", detail.name.clone())]);
    app.populate_detail_dialog(&detail);
    app.update_log_target_containers(&detail);
    app.update_exec_target_containers(&detail);
    app.sync_status();
    app.toaster
        .add_toast(adw::Toast::new(&tr("Deployment scaled.")));
}

pub(super) fn handle_object_scaled_err(app: &mut App, token: u64, error: String) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.status = tr("Unable to scale deployment.");
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&error));
}

pub(super) fn handle_explain_yaml(app: &mut App, root: &<App as Component>::Root) {
    app.show_yaml_explanation(root);
}

pub(super) fn handle_apply_yaml(app: &mut App, sender: ComponentSender<App>) {
    let Some(target) = app.detail.target.clone() else {
        return;
    };
    let yaml = text_buffer_text(&app.detail.yaml_buffer);
    app.detail.request_token = app.detail.request_token.saturating_add(1);
    let token = app.detail.request_token;
    app.loading = true;
    app.status = tr_format("Applying {name}...", &[("{name}", target.name.clone())]);
    app.sync_status();
    sender.oneshot_command(async move { apply_object_yaml(token, target, yaml).await });
}

pub(super) fn handle_object_applied_ok(app: &mut App, token: u64, detail: ObjectDetail) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.clear_object_cache();
    app.status = tr_format("Applied {name}", &[("{name}", detail.name.clone())]);
    app.populate_detail_dialog(&detail);
    app.update_log_target_containers(&detail);
    app.update_exec_target_containers(&detail);
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&tr("YAML applied.")));
}

pub(super) fn handle_object_applied_err(app: &mut App, token: u64, error: String) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.status = tr("Unable to apply YAML.");
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&error));
}

pub(super) fn handle_download_yaml(
    app: &mut App,
    sender: ComponentSender<App>,
    root: &<App as Component>::Root,
) {
    let yaml = text_buffer_text(&app.detail.yaml_buffer);
    let name = app
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

pub(super) fn handle_save_yaml_to(app: &mut App, path: PathBuf, yaml: String) {
    match fs::write(&path, yaml) {
        Ok(()) => app.toaster.add_toast(adw::Toast::new(&tr_format(
            "Saved {path}.",
            &[("{path}", path.display().to_string())],
        ))),
        Err(error) => app.toaster.add_toast(adw::Toast::new(&tr_format(
            "Unable to save {path}: {error}",
            &[
                ("{path}", path.display().to_string()),
                ("{error}", error.to_string()),
            ],
        ))),
    }
}

pub(super) fn handle_delete_object(
    app: &mut App,
    sender: ComponentSender<App>,
    root: &<App as Component>::Root,
) {
    let Some(target) = app.detail.target.clone() else {
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

pub(super) fn handle_confirm_delete_object(app: &mut App, sender: ComponentSender<App>) {
    let Some(target) = app.detail.target.clone() else {
        return;
    };
    app.detail.request_token = app.detail.request_token.saturating_add(1);
    let token = app.detail.request_token;
    app.loading = true;
    app.status = tr_format("Deleting {name}...", &[("{name}", target.name.clone())]);
    app.sync_status();
    sender.oneshot_command(async move { delete_object(token, target).await });
}

pub(super) fn handle_object_deleted_ok(
    app: &mut App,
    sender: ComponentSender<App>,
    token: u64,
    name: String,
) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.clear_object_cache();
    app.detail.target = None;
    app.detail.log_target = None;
    app.detail.exec_target = None;
    app.detail.port_forward_target = None;
    app.stop_log_stream();
    app.stop_port_forward();
    app.show_object_list();
    app.sync_log_controls();
    app.sync_terminal_controls();
    app.sync_port_forward_controls();
    app.toaster.add_toast(adw::Toast::new(&tr_format(
        "Deleted {name}.",
        &[("{name}", name)],
    )));
    app.refresh_objects(sender);
}

pub(super) fn handle_object_deleted_err(app: &mut App, token: u64, error: String) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.status = tr("Unable to delete object.");
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&error));
}

pub(super) fn handle_resource_changed(app: &mut App, sender: ComponentSender<App>, index: usize) {
    let Some(resource) = app.resources.get(index) else {
        return;
    };
    if app.selected_resource == Some(index) {
        return;
    }
    app.selected_resource = Some(index);
    app.selected_resource_section = ResourceSection::for_resource(resource);
    app.rebuild_resource_list(Some(sender.clone()));
    app.present_content_panel();
    app.show_object_list();
    app.sync_object_columns();
    app.stop_log_stream();
    app.stop_port_forward();
    app.refresh_objects(sender);
}
