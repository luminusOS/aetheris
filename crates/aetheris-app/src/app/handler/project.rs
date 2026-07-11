use super::super::*;

pub(super) fn handle_show_projects(app: &mut App) {
    app.stop_object_watch();
    app.stop_log_stream();
    app.stop_port_forward();
    app.show_object_list();
    // The store may have changed since the page was last built
    // (e.g. removing a cluster only rebuilds the clusters page),
    // so refresh the rows before presenting them.
    app.rebuild_project_list();
    app.show_projects();
    app.loading = false;
    app.status = tr("Select a project.");
    app.sync_terminal_controls();
    app.sync_port_forward_controls();
    app.sync_status();
}

pub(super) fn handle_project_changed(app: &mut App, sender: ComponentSender<App>, index: u32) {
    let Some(project_name) = app
        .projects
        .projects
        .get(index as usize)
        .map(|project| project.name.clone())
    else {
        return;
    };
    if app.projects.selected_project.as_deref() != Some(project_name.as_str()) {
        app.projects.selected_project = Some(project_name);
        app.save_projects_or_toast();
    }
    app.switch_to_project(sender);
}

pub(super) fn handle_show_add_project_dialog(app: &mut App, root: &<App as Component>::Root) {
    app.editing_project_name = None;
    app.project_dialog.set_title(&tr("New Project"));
    app.project_dialog_description
        .set_label(&tr("Separate clusters by environment or company"));
    app.project_create_button.set_label(&tr("Create"));
    app.project_name_entry.set_text("");
    app.project_dialog.present(Some(root));
}

pub(super) fn handle_show_rename_project_dialog(app: &mut App, root: &<App as Component>::Root) {
    let current = app.projects.selected_project_name().to_owned();
    app.editing_project_name = Some(current.clone());
    app.project_dialog.set_title(&tr("Rename Project"));
    app.project_dialog_description
        .set_label(&tr("Choose a new name for this project"));
    app.project_create_button.set_label(&tr("Rename"));
    app.project_name_entry.set_text(&current);
    app.project_dialog.present(Some(root));
}

pub(super) fn handle_add_project(app: &mut App, sender: ComponentSender<App>) {
    let name = app.project_name_entry.text().trim().to_owned();
    if name.is_empty() {
        return;
    }

    if let Some(original) = app.editing_project_name.clone() {
        if name != original && app.projects.has_project(&name) {
            app.toaster.add_toast(adw::Toast::new(&tr(
                "A project with this name already exists.",
            )));
            return;
        }
        if let Some(project) = app
            .projects
            .projects
            .iter_mut()
            .find(|project| project.name == original)
        {
            project.name = name.clone();
        }
        if app.projects.selected_project.as_deref() == Some(original.as_str()) {
            app.projects.selected_project = Some(name);
        }
        app.save_projects_or_toast();
        app.editing_project_name = None;
        app.project_dialog.close();
        app.sync_dropdowns(Some(sender.clone()));
        app.sync_status();
        return;
    }

    if app.projects.has_project(&name) {
        app.toaster.add_toast(adw::Toast::new(&tr(
            "A project with this name already exists.",
        )));
        return;
    }

    app.projects.projects.push(Project {
        name: name.clone(),
        contexts: Vec::new(),
        custom_namespaces_by_context: Vec::new(),
    });
    app.projects.selected_project = Some(name.clone());
    app.save_projects_or_toast();
    app.selected_context = None;
    app.project_dialog.close();
    app.switch_to_project(sender);
}

pub(super) fn handle_duplicate_project(app: &mut App, sender: ComponentSender<App>) {
    let Some(source) = app.projects.selected_project().cloned() else {
        return;
    };
    let mut new_name = tr_format("{name} copy", &[("{name}", source.name.clone())]);
    let mut suffix = 2;
    while app.projects.has_project(&new_name) {
        new_name = tr_format(
            "{name} copy {suffix}",
            &[
                ("{name}", source.name.clone()),
                ("{suffix}", suffix.to_string()),
            ],
        );
        suffix += 1;
    }
    app.projects.projects.push(Project {
        name: new_name.clone(),
        contexts: source.contexts,
        custom_namespaces_by_context: source.custom_namespaces_by_context,
    });
    app.projects.selected_project = Some(new_name.clone());
    app.save_projects_or_toast();
    app.toaster.add_toast(adw::Toast::new(&tr_format(
        "Duplicated as {name}",
        &[("{name}", new_name.clone())],
    )));
    app.switch_to_project(sender);
}

pub(super) fn handle_delete_project(
    app: &mut App,
    sender: ComponentSender<App>,
    root: &<App as Component>::Root,
) {
    if app.projects.projects.len() <= 1 {
        app.toaster
            .add_toast(adw::Toast::new(&tr("At least one project must remain.")));
        return;
    }
    let name = app.projects.selected_project_name().to_owned();
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

pub(super) fn handle_confirm_delete_project(app: &mut App, sender: ComponentSender<App>) {
    let name = app.projects.selected_project_name().to_owned();
    app.projects.projects.retain(|project| project.name != name);
    app.projects.selected_project = app
        .projects
        .projects
        .first()
        .map(|project| project.name.clone());
    app.selected_context = None;
    app.save_projects_or_toast();
    app.stop_object_watch();
    app.stop_log_stream();
    app.stop_port_forward();
    app.sync_dropdowns(Some(sender.clone()));
    app.show_object_list();
    app.show_projects();
    app.loading = false;
    app.status = tr("Select a project.");
    app.sync_terminal_controls();
    app.sync_port_forward_controls();
    app.sync_status();
}

pub(super) fn handle_remove_cluster_from_project(app: &mut App, sender: ComponentSender<App>) {
    let Some(context) = app.selected_context.clone() else {
        return;
    };
    app.projects.remove_context_from_selected_project(&context);
    app.save_projects_or_toast();
    app.selected_context = None;
    app.stop_object_watch();
    app.stop_log_stream();
    app.stop_port_forward();
    app.resources.clear();
    app.objects.clear();
    app.selected_resource = None;
    app.show_object_list();
    app.enter_clusters_page(sender);
    app.status = tr_format(
        "Removed {context} from this project.",
        &[("{context}", context)],
    );
    app.sync_terminal_controls();
    app.sync_port_forward_controls();
    app.sync_status();
}

pub(super) fn handle_back_to_objects(app: &mut App) {
    app.show_object_list();
    app.stop_log_stream();
    app.stop_port_forward();
    app.detail.target = None;
    app.detail.log_target = None;
    app.detail.exec_target = None;
    app.detail.port_forward_target = None;
    app.sync_log_controls();
    app.sync_terminal_controls();
    app.sync_port_forward_controls();
}

pub(super) fn handle_project_save_tick(app: &mut App) {
    app.project_save_scheduled = false;
    app.save_projects_or_toast();
}
