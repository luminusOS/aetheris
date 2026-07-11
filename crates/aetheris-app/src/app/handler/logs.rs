use super::super::utils::*;
use super::super::*;

pub(super) fn handle_start_pod_logs(app: &mut App, sender: ComponentSender<App>) {
    app.start_pod_logs(sender);
}

pub(super) fn handle_stop_pod_logs(app: &mut App) {
    app.stop_log_stream();
    app.sync_log_controls();
}

pub(super) fn handle_clear_pod_logs(app: &mut App) {
    app.detail.log_buffer.set_text("");
}

pub(super) fn handle_pod_log_line(app: &mut App, token: u64, line: String) {
    if token == app.log_stream_token {
        app.append_log_line(&line);
    }
}

pub(super) fn handle_pod_log_finished(app: &mut App, token: u64, result: Result<(), String>) {
    if token == app.log_stream_token {
        app.log_streaming = false;
        app.log_abort_handle = None;
        app.sync_log_controls();
        if let Err(error) = result {
            app.toaster.add_toast(adw::Toast::new(&error));
        }
    }
}

pub(super) fn handle_download_logs(
    app: &mut App,
    sender: ComponentSender<App>,
    root: &<App as Component>::Root,
) {
    let logs = text_buffer_text(&app.detail.log_buffer);
    let name = app
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

pub(super) fn handle_save_logs_to(app: &mut App, path: PathBuf, logs: String) {
    match fs::write(&path, logs) {
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
