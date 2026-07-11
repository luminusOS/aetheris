use super::super::*;

pub(super) fn handle_namespace_changed(
    app: &mut App,
    sender: ComponentSender<App>,
    root: &<App as Component>::Root,
    index: u32,
) {
    let choices = app.namespace_choices();
    if index as usize == choices.len() {
        app.show_custom_namespace_dialog(root);
        return;
    }
    if let Some(namespace) = choices.get(index as usize)
        && app.selected_namespace != *namespace
    {
        app.selected_namespace.clone_from(namespace);
        app.remember_selected_namespace();
        app.sync_dropdowns(Some(sender.clone()));
        app.show_object_list();
        app.stop_log_stream();
        app.stop_port_forward();
        app.refresh_objects(sender);
    }
}

pub(super) fn handle_custom_namespace_entered(app: &mut App, sender: ComponentSender<App>) {
    let namespace = app.custom_namespace_entry.text().trim().to_owned();
    if namespace.is_empty() {
        return;
    }

    app.remember_namespace(&namespace);
    if app.selected_namespace != namespace {
        app.selected_namespace = namespace;
        app.remember_selected_namespace();
        app.sync_dropdowns(Some(sender.clone()));
        app.show_object_list();
        app.stop_log_stream();
        app.stop_port_forward();
        app.refresh_objects(sender);
    } else {
        app.sync_dropdowns(Some(sender.clone()));
    }

    app.custom_namespace_dialog.close();
}

pub(super) fn handle_remove_custom_namespace(
    app: &mut App,
    sender: ComponentSender<App>,
    namespace: String,
) {
    let Some(context) = app.selected_context.clone() else {
        return;
    };
    let removed = app
        .projects
        .selected_project_mut()
        .is_some_and(|project| project.remove_custom_namespace(&context, &namespace));
    if !removed {
        return;
    }
    app.save_projects_or_toast();
    if app.selected_namespace == namespace {
        app.selected_namespace = String::from("default");
        app.remember_selected_namespace();
        app.sync_dropdowns(Some(sender.clone()));
        app.show_object_list();
        app.stop_log_stream();
        app.stop_port_forward();
        app.refresh_objects(sender);
    } else {
        app.sync_dropdowns(Some(sender.clone()));
    }
    app.toaster
        .add_toast(adw::Toast::new(&tr("Namespace removed")));
}

pub(super) fn handle_open_rename_namespace_dialog(
    app: &mut App,
    root: &<App as Component>::Root,
    namespace: String,
) {
    app.open_rename_namespace_dialog(&namespace, root);
}

pub(super) fn handle_rename_namespace_confirmed(app: &mut App, sender: ComponentSender<App>) {
    let new_name = app.rename_namespace_entry.text().trim().to_owned();
    let Some(old_name) = app.renaming_namespace.take() else {
        return;
    };
    if new_name.is_empty() || new_name == old_name {
        app.rename_namespace_dialog.close();
        return;
    }
    let Some(context) = app.selected_context.clone() else {
        app.rename_namespace_dialog.close();
        return;
    };
    let renamed = app
        .projects
        .selected_project_mut()
        .is_some_and(|project| project.rename_custom_namespace(&context, &old_name, &new_name));
    if renamed {
        app.save_projects_or_toast();
        if app.selected_namespace == old_name {
            app.selected_namespace = new_name;
            app.remember_selected_namespace();
            app.sync_dropdowns(Some(sender.clone()));
            app.show_object_list();
            app.stop_log_stream();
            app.stop_port_forward();
            app.refresh_objects(sender);
        } else {
            app.sync_dropdowns(Some(sender.clone()));
        }
        app.toaster
            .add_toast(adw::Toast::new(&tr("Namespace renamed")));
    } else {
        app.toaster.add_toast(adw::Toast::new(&tr(
            "A namespace with this name already exists.",
        )));
    }
    app.rename_namespace_dialog.close();
}
