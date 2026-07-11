use super::super::*;

pub(super) fn handle_start_pod_port_forward(app: &mut App, sender: ComponentSender<App>) {
    app.start_pod_port_forward(sender);
}

pub(super) fn handle_stop_pod_port_forward(app: &mut App) {
    app.stop_port_forward();
    app.sync_port_forward_controls();
}

pub(super) fn handle_pod_port_forward_event(app: &mut App, token: u64, event: PodPortForwardEvent) {
    if token == app.port_forward_token {
        app.handle_port_forward_event(event);
    }
}

pub(super) fn handle_pod_port_forward_finished(
    app: &mut App,
    token: u64,
    result: Result<(), String>,
) {
    if token == app.port_forward_token {
        app.port_forwarding = false;
        app.port_forward_abort_handle = None;
        app.sync_port_forward_controls();
        if let Err(error) = result {
            app.detail
                .port_status_label
                .set_label(&tr("Port-forward stopped."));
            app.toaster.add_toast(adw::Toast::new(&error));
        }
    }
}
