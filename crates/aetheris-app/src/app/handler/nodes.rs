use super::super::commands::*;
use super::super::utils::*;
use super::super::*;

pub(super) fn handle_drain_node(
    app: &mut App,
    sender: ComponentSender<App>,
    root: &<App as Component>::Root,
) {
    let Some(target) = app.detail.target.clone() else {
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

pub(super) fn handle_confirm_drain_node(app: &mut App, sender: ComponentSender<App>) {
    let Some(target) = app.detail.target.clone() else {
        return;
    };
    if !is_node_resource(&target.resource) {
        return;
    }
    app.detail.request_token = app.detail.request_token.saturating_add(1);
    let token = app.detail.request_token;
    app.loading = true;
    app.status = tr_format("Draining {name}...", &[("{name}", target.name.clone())]);
    app.sync_status();
    sender.oneshot_command(async move { drain_node(token, target).await });
}

pub(super) fn handle_node_drained_ok(
    app: &mut App,
    token: u64,
    detail: ObjectDetail,
    count: usize,
) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.clear_object_cache();
    app.status = tr_format("Drained {name}", &[("{name}", detail.name.clone())]);
    app.populate_detail_dialog(&detail);
    app.update_log_target_containers(&detail);
    app.update_exec_target_containers(&detail);
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&tr_format(
        "Drain started for {count} Pods.",
        &[("{count}", count.to_string())],
    )));
}

pub(super) fn handle_node_drained_err(app: &mut App, token: u64, error: String) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.status = tr("Unable to drain node.");
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&error));
}

pub(super) fn handle_toggle_node_scheduling(app: &mut App, sender: ComponentSender<App>) {
    let Some(target) = app.detail.target.clone() else {
        return;
    };
    if !is_node_resource(&target.resource) {
        return;
    }
    let unschedulable = !app.detail.node_unschedulable.unwrap_or(false);
    app.detail.request_token = app.detail.request_token.saturating_add(1);
    let token = app.detail.request_token;
    app.loading = true;
    app.status = tr_format("Updating {name}...", &[("{name}", target.name.clone())]);
    app.sync_status();
    sender
        .oneshot_command(async move { set_node_unschedulable(token, target, unschedulable).await });
}

pub(super) fn handle_node_scheduling_updated_ok(app: &mut App, token: u64, detail: ObjectDetail) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.clear_object_cache();
    app.status = tr_format("Updated {name}", &[("{name}", detail.name.clone())]);
    app.populate_detail_dialog(&detail);
    app.update_log_target_containers(&detail);
    app.update_exec_target_containers(&detail);
    app.sync_status();
    app.toaster
        .add_toast(adw::Toast::new(&tr("Node scheduling updated.")));
}

pub(super) fn handle_node_scheduling_updated_err(app: &mut App, token: u64, error: String) {
    if token != app.detail.request_token {
        return;
    }
    app.loading = false;
    app.status = tr("Unable to update node scheduling.");
    app.sync_status();
    app.toaster.add_toast(adw::Toast::new(&error));
}
