use super::super::commands::*;
use super::super::utils::*;
use super::super::*;

type TerminalStart = (
    String,
    String,
    String,
    String,
    tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    futures::future::AbortRegistration,
);

impl App {
    pub(crate) fn update_exec_target_containers(&mut self, detail: &ObjectDetail) {
        if let Some(target) = &mut self.detail.exec_target {
            target.containers.clone_from(&detail.containers);
        }
        self.sync_terminal_controls();
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn show_pod_terminal(
        &mut self,
        root: &<Self as Component>::Root,
        sender: ComponentSender<Self>,
    ) {
        let Some(target) = self.detail.exec_target.clone() else {
            self.toaster
                .add_toast(adw::Toast::new(&tr("Terminal is available for Pods.")));
            return;
        };
        if default_terminal_container(&target).is_none() {
            self.toaster.add_toast(adw::Toast::new(&tr(
                "This Pod has no containers in its spec.",
            )));
            return;
        }

        self.exec_token = self.exec_token.saturating_add(1);
        let token = self.exec_token;
        let container_dropdown = terminal_container_dropdown(&target);
        let terminal_view = terminal_view();
        let terminal_window = terminal_window(root, &target, &container_dropdown, &terminal_view);

        terminal_view.connect_commit({
            let sender = sender.clone();
            move |_, text, _| sender.input(AppMsg::PodTerminalInput(token, text.to_string()))
        });
        container_dropdown.connect_selected_notify({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::RestartPodTerminal(token))
        });
        terminal_window.connect_close_request({
            let sender = sender.clone();
            move |_| {
                sender.input(AppMsg::StopPodTerminal(token));
                gtk::glib::Propagation::Proceed
            }
        });

        self.terminal_sessions.insert(
            token,
            TerminalSession {
                window: terminal_window.clone(),
                container_dropdown,
                view: terminal_view.clone(),
                target,
                abort_handle: None,
                input_tx: None,
            },
        );
        terminal_window.present();
        terminal_view.grab_focus();
        self.start_terminal_session(token, sender);
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn show_pod_terminal(
        &mut self,
        _root: &<Self as Component>::Root,
        _sender: ComponentSender<Self>,
    ) {
        self.toaster.add_toast(adw::Toast::new(&tr(
            "Terminal windows are not available in the Windows build yet.",
        )));
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn start_terminal_session(&mut self, token: u64, sender: ComponentSender<Self>) {
        let Some((context, namespace, pod, container, input_rx, abort_registration)) =
            self.prepare_terminal_session(token)
        else {
            return;
        };
        let request = PodExecRequest {
            namespace,
            pod,
            container: Some(container),
            command: vec![
                String::from("sh"),
                String::from("-lc"),
                String::from(
                    "if command -v bash >/dev/null 2>&1; then exec bash -l; else exec sh; fi",
                ),
            ],
        };

        sender.command(move |out, shutdown| {
            shutdown
                .register(
                    Abortable::new(
                        async move {
                            let result =
                                stream_pod_terminal(context, request, input_rx, token, out.clone())
                                    .await
                                    .map_err(format_error);
                            let _ = out.send(AppMsg::PodExecFinished(token, result));
                        },
                        abort_registration,
                    )
                    .map(|_| ()),
                )
                .drop_on_shutdown()
                .boxed()
        });
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn start_terminal_session(&mut self, _token: u64, _sender: ComponentSender<Self>) {}

    #[cfg(not(target_os = "windows"))]
    fn prepare_terminal_session(&mut self, token: u64) -> Option<TerminalStart> {
        let session = self.terminal_sessions.get_mut(&token)?;
        if let Some(handle) = session.abort_handle.take() {
            handle.abort();
        }
        session.input_tx = None;
        session.view.reset(true, true);

        let container = selected_log_container(&session.container_dropdown, &session.target)
            .or_else(|| default_terminal_container(&session.target))?;
        let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        session.abort_handle = Some(abort_handle);
        session.input_tx = Some(input_tx);
        Some((
            session.target.context.clone(),
            session.target.namespace.clone(),
            session.target.pod.clone(),
            container,
            input_rx,
            abort_registration,
        ))
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn close_terminal_session(&mut self, token: u64, close_window: bool) {
        let Some(mut session) = self.terminal_sessions.remove(&token) else {
            return;
        };
        if let Some(handle) = session.abort_handle.take() {
            handle.abort();
        }
        session.input_tx = None;
        if close_window {
            session.window.close();
        }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn close_terminal_session(&mut self, _token: u64, _close_window: bool) {}

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn sync_terminal_controls(&self) {
        let Some(target) = &self.detail.exec_target else {
            self.detail.terminal_button.set_visible(false);
            self.detail.terminal_button.set_sensitive(false);
            return;
        };

        let available = !target.containers.is_empty();
        self.detail.terminal_button.set_visible(true);
        self.detail.terminal_button.set_sensitive(available);
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn sync_terminal_controls(&self) {
        self.detail.terminal_button.set_visible(false);
        self.detail.terminal_button.set_sensitive(false);
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn feed_terminal_event(&self, token: u64, event: PodExecEvent) {
        let text = match event {
            PodExecEvent::Stdout(text) | PodExecEvent::Stderr(text) => text,
        };
        if let Some(session) = self.terminal_sessions.get(&token) {
            session.view.feed(text.as_bytes());
        }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn feed_terminal_event(&self, _token: u64, _event: PodExecEvent) {}

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn show_terminal_error(&self, token: u64, error: &str) {
        if let Some(session) = self.terminal_sessions.get(&token) {
            let message = terminal_error_message(error);
            session.view.feed(format!("\r\n{message}\r\n").as_bytes());
        }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn show_terminal_error(&self, _token: u64, error: &str) {
        self.toaster
            .add_toast(adw::Toast::new(&terminal_error_message(error)));
    }

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn finish_terminal_session(&mut self, token: u64) {
        if let Some(session) = self.terminal_sessions.get_mut(&token) {
            session.abort_handle = None;
            session.input_tx = None;
        }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn finish_terminal_session(&mut self, _token: u64) {}

    #[cfg(not(target_os = "windows"))]
    pub(crate) fn send_terminal_input(&self, token: u64, text: String) {
        if let Some(input_tx) = self
            .terminal_sessions
            .get(&token)
            .and_then(|session| session.input_tx.as_ref())
        {
            let _ = input_tx.send(text.into_bytes());
        }
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn send_terminal_input(&self, _token: u64, _text: String) {}
}

#[cfg(not(target_os = "windows"))]
fn terminal_container_dropdown(target: &PodLogTarget) -> gtk::DropDown {
    let refs = target
        .containers
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let dropdown = gtk::DropDown::from_strings(&refs);
    dropdown.set_selected(default_log_container_index(&target.pod, &target.containers) as u32);
    dropdown.set_tooltip_text(Some(&tr("Container")));
    dropdown.set_width_request(220);
    dropdown
}

#[cfg(not(target_os = "windows"))]
fn terminal_view() -> vte4::Terminal {
    let view = vte4::Terminal::new();
    view.set_hexpand(true);
    view.set_vexpand(true);
    view.set_mouse_autohide(true);
    view.set_scroll_on_output(true);
    view.set_scroll_on_keystroke(true);
    view.set_scrollback_lines(10_000);
    view.set_font(Some(&gtk::pango::FontDescription::from_string(
        "Monospace 10",
    )));
    view
}

#[cfg(not(target_os = "windows"))]
fn terminal_window(
    root: &<App as Component>::Root,
    target: &PodLogTarget,
    container_dropdown: &gtk::DropDown,
    terminal_view: &vte4::Terminal,
) -> adw::Window {
    let window = adw::Window::builder()
        .title(target.pod.as_str())
        .default_width(920)
        .default_height(620)
        .build();
    if let Some(application) = root.application() {
        window.set_application(Some(&application));
    }
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::builder()
        .title(target.pod.as_str())
        .build();
    header.set_title_widget(Some(&title));
    header.pack_start(container_dropdown);
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(terminal_view));
    window.set_content(Some(&toolbar));
    window
}
