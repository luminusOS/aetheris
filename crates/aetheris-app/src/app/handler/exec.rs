use super::super::utils::*;
use super::super::*;

pub(super) fn handle_show_pod_terminal(
    app: &mut App,
    sender: ComponentSender<App>,
    root: &<App as Component>::Root,
) {
    app.show_pod_terminal(root, sender);
}

pub(super) fn handle_restart_pod_terminal(app: &mut App, sender: ComponentSender<App>, token: u64) {
    app.start_terminal_session(token, sender);
}

pub(super) fn handle_stop_pod_terminal(app: &mut App, token: u64) {
    app.close_terminal_session(token, false);
}

pub(super) fn handle_pod_terminal_input(app: &mut App, token: u64, text: String) {
    app.send_terminal_input(token, text);
}

pub(super) fn handle_pod_exec_event(app: &mut App, token: u64, event: PodExecEvent) {
    app.feed_terminal_event(token, event);
}

pub(super) fn handle_pod_exec_finished(app: &mut App, token: u64, result: Result<(), String>) {
    app.finish_terminal_session(token);
    if let Err(error) = result {
        let message = terminal_error_message(&error);
        app.show_terminal_error(token, &error);
        app.toaster.add_toast(adw::Toast::new(&message));
    }
}
