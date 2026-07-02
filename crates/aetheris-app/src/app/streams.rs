use super::ansi::*;
use super::commands::*;
use super::utils::*;
use super::yaml::*;
use super::*;

type TerminalStart = (
    String,
    String,
    String,
    String,
    tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    futures::future::AbortRegistration,
);

impl App {
    pub(super) fn start_object_watch(&mut self, sender: ComponentSender<Self>) {
        let Some(context) = self.selected_context.clone() else {
            return;
        };
        let Some(resource) = self.selected_resource_kind().cloned() else {
            return;
        };
        let namespace = if resource.is_namespaced() {
            Some(self.selected_namespace.clone())
        } else {
            None
        };

        self.stop_object_watch();
        self.object_watch_token = self.object_watch_token.saturating_add(1);
        let token = self.object_watch_token;
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        self.object_watch_abort_handle = Some(abort_handle);

        sender.command(move |out, shutdown| {
            shutdown
                .register(
                    Abortable::new(
                        async move {
                            let result = stream_object_watch(
                                context,
                                resource,
                                namespace,
                                token,
                                out.clone(),
                            )
                            .await
                            .map_err(format_error);
                            let _ = out.send(AppMsg::ObjectWatchFinished(token, result));
                        },
                        abort_registration,
                    )
                    .map(|_| ()),
                )
                .drop_on_shutdown()
                .boxed()
        });
    }

    pub(super) fn stop_object_watch(&mut self) {
        if let Some(handle) = self.object_watch_abort_handle.take() {
            handle.abort();
        }
    }

    pub(super) fn update_log_target_containers(&mut self, detail: &ObjectDetail) {
        let mut selected = None;
        if let Some(target) = &mut self.detail_log_target {
            target.containers.clone_from(&detail.containers);
            selected = Some(default_log_container_index(&target.pod, &target.containers));
        }
        self.sync_log_controls_with_selection(selected);
    }

    pub(super) fn update_exec_target_containers(&mut self, detail: &ObjectDetail) {
        if let Some(target) = &mut self.detail_exec_target {
            target.containers.clone_from(&detail.containers);
        }
        self.sync_terminal_controls();
    }

    pub(super) fn start_pod_logs(&mut self, sender: ComponentSender<Self>) {
        let Some(target) = self.detail_log_target.clone() else {
            self.toaster
                .add_toast(adw::Toast::new("Logs are available for Pods."));
            return;
        };
        let Some(container) = selected_log_container(&self.detail_log_container_dropdown, &target)
        else {
            self.toaster
                .add_toast(adw::Toast::new("Select a container before starting logs."));
            return;
        };

        self.stop_log_stream();
        self.log_stream_token = self.log_stream_token.saturating_add(1);
        let token = self.log_stream_token;
        let follow = self.detail_log_follow_check.is_active();
        let timestamps = self.detail_log_timestamps_check.is_active();
        let request = PodLogRequest {
            namespace: target.namespace,
            pod: target.pod,
            container: Some(container),
            follow,
            timestamps,
            tail_lines: Some(if follow { 200 } else { 500 }),
        };
        let context = target.context;
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        self.log_abort_handle = Some(abort_handle);
        self.log_streaming = true;
        self.sync_log_controls();

        sender.command(move |out, shutdown| {
            shutdown
                .register(
                    Abortable::new(
                        async move {
                            let result = stream_pod_logs(context, request, token, out.clone())
                                .await
                                .map_err(format_error);
                            let _ = out.send(AppMsg::PodLogFinished(token, result));
                        },
                        abort_registration,
                    )
                    .map(|_| ()),
                )
                .drop_on_shutdown()
                .boxed()
        });
    }

    pub(super) fn maybe_start_visible_logs(&mut self, sender: ComponentSender<Self>) {
        if self.log_streaming
            || self.detail_stack.visible_child_name().as_deref() != Some("logs")
            || self
                .detail_log_target
                .as_ref()
                .is_none_or(|target| target.containers.is_empty())
        {
            return;
        }

        self.start_pod_logs(sender);
    }

    pub(super) fn stop_log_stream(&mut self) {
        if let Some(handle) = self.log_abort_handle.take() {
            handle.abort();
        }
        self.log_streaming = false;
    }

    #[cfg(not(target_os = "windows"))]
    pub(super) fn show_pod_terminal(
        &mut self,
        root: &<Self as Component>::Root,
        sender: ComponentSender<Self>,
    ) {
        let Some(target) = self.detail_exec_target.clone() else {
            self.toaster
                .add_toast(adw::Toast::new("Terminal is available for Pods."));
            return;
        };
        if default_terminal_container(&target).is_none() {
            self.toaster
                .add_toast(adw::Toast::new("This Pod has no containers in its spec."));
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
    pub(super) fn show_pod_terminal(
        &mut self,
        _root: &<Self as Component>::Root,
        _sender: ComponentSender<Self>,
    ) {
        self.toaster.add_toast(adw::Toast::new(
            "Terminal windows are not available in the Windows build yet.",
        ));
    }

    #[cfg(not(target_os = "windows"))]
    pub(super) fn start_terminal_session(&mut self, token: u64, sender: ComponentSender<Self>) {
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
    pub(super) fn start_terminal_session(&mut self, _token: u64, _sender: ComponentSender<Self>) {}

    #[cfg(not(target_os = "windows"))]
    fn prepare_terminal_session(&mut self, token: u64) -> Option<TerminalStart> {
        let session = self.terminal_sessions.get_mut(&token)?;
        if let Some(handle) = session.abort_handle.take() {
            handle.abort();
        }
        session.input_tx = None;
        session.view.reset(true, true);
        session.view.feed(b"Connecting to pod terminal...\r\n");

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
    pub(super) fn close_terminal_session(&mut self, token: u64, close_window: bool) {
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
    pub(super) fn close_terminal_session(&mut self, _token: u64, _close_window: bool) {}

    pub(super) fn start_pod_port_forward(&mut self, sender: ComponentSender<Self>) {
        let Some(target) = self.detail_port_forward_target.clone() else {
            self.toaster
                .add_toast(adw::Toast::new("Port forwarding is available for Pods."));
            return;
        };

        let local_port = self.detail_port_local_spin.value_as_int().clamp(0, 65535) as u16;
        let remote_port = self.detail_port_remote_spin.value_as_int().clamp(1, 65535) as u16;
        self.stop_port_forward();
        self.port_forward_token = self.port_forward_token.saturating_add(1);
        let token = self.port_forward_token;
        let request = PodPortForwardRequest {
            namespace: target.namespace,
            pod: target.pod,
            local_port,
            remote_port,
        };
        let context = target.context;
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        self.port_forward_abort_handle = Some(abort_handle);
        self.port_forwarding = true;
        self.sync_port_forward_controls();

        sender.command(move |out, shutdown| {
            shutdown
                .register(
                    Abortable::new(
                        async move {
                            let result = run_pod_port_forward(context, request, token, out.clone())
                                .await
                                .map_err(format_error);
                            let _ = out.send(AppMsg::PodPortForwardFinished(token, result));
                        },
                        abort_registration,
                    )
                    .map(|_| ()),
                )
                .drop_on_shutdown()
                .boxed()
        });
    }

    pub(super) fn stop_port_forward(&mut self) {
        if let Some(handle) = self.port_forward_abort_handle.take() {
            handle.abort();
        }
        self.port_forwarding = false;
    }

    pub(super) fn sync_port_forward_controls(&self) {
        let available = self.detail_port_forward_target.is_some();
        self.detail_port_local_spin
            .set_sensitive(available && !self.port_forwarding);
        self.detail_port_remote_spin
            .set_sensitive(available && !self.port_forwarding);
        self.detail_port_start_button
            .set_sensitive(available && !self.port_forwarding);
        self.detail_port_stop_button
            .set_sensitive(self.port_forwarding);

        if !available {
            self.detail_port_status_label
                .set_label("Port forwarding is available for Pods.");
        } else if self.port_forwarding {
            self.detail_port_status_label
                .set_label("Starting port-forward...");
        } else {
            self.detail_port_status_label
                .set_label("Choose local and remote ports to forward this Pod.");
        }
    }

    pub(super) fn handle_port_forward_event(&self, event: PodPortForwardEvent) {
        match event {
            PodPortForwardEvent::Ready { local_port } => {
                self.detail_port_status_label.set_label(&format!(
                    "Forwarding 127.0.0.1:{local_port} to the selected Pod."
                ));
            }
            PodPortForwardEvent::ConnectionOpened => {
                self.detail_port_status_label
                    .set_label("Port-forward connection active.");
            }
            PodPortForwardEvent::ConnectionClosed => {
                self.detail_port_status_label
                    .set_label("Waiting for the next local connection...");
            }
        }
    }

    pub(super) fn sync_log_controls(&self) {
        self.sync_log_controls_with_selection(None);
    }

    pub(super) fn sync_log_controls_with_selection(&self, selected_override: Option<usize>) {
        let Some(target) = &self.detail_log_target else {
            self.detail_log_container_dropdown
                .set_model(Some(&gtk::StringList::new(&["No containers"])));
            self.detail_log_container_dropdown.set_selected(0);
            self.detail_log_container_dropdown.set_sensitive(false);
            self.detail_log_follow_check.set_sensitive(false);
            self.detail_log_timestamps_check.set_sensitive(false);
            self.detail_log_start_button.set_sensitive(false);
            self.detail_log_stop_button.set_sensitive(false);
            self.detail_log_status_label
                .set_label("Logs are available for Pods.");
            return;
        };

        let labels = if target.containers.is_empty() {
            vec![String::from("No containers")]
        } else {
            target.containers.clone()
        };
        let selected = if let Some(selected) = selected_override {
            selected.min(labels.len().saturating_sub(1))
        } else if target.containers.is_empty() {
            0
        } else {
            (self.detail_log_container_dropdown.selected() as usize)
                .min(target.containers.len().saturating_sub(1))
        };
        let refs = labels.iter().map(String::as_str).collect::<Vec<_>>();
        self.detail_log_container_dropdown
            .set_model(Some(&gtk::StringList::new(&refs)));
        self.detail_log_container_dropdown
            .set_selected(selected as u32);
        self.detail_log_container_dropdown
            .set_sensitive(!target.containers.is_empty() && !self.log_streaming);
        self.detail_log_follow_check
            .set_sensitive(!target.containers.is_empty() && !self.log_streaming);
        self.detail_log_timestamps_check
            .set_sensitive(!target.containers.is_empty() && !self.log_streaming);
        self.detail_log_start_button
            .set_sensitive(!target.containers.is_empty() && !self.log_streaming);
        self.detail_log_stop_button
            .set_sensitive(self.log_streaming);
        self.detail_log_status_label
            .set_label(if self.log_streaming {
                "Streaming pod logs..."
            } else if target.containers.is_empty() {
                "This Pod has no containers in its spec."
            } else {
                "Select a container and start logs."
            });
    }

    #[cfg(not(target_os = "windows"))]
    pub(super) fn sync_terminal_controls(&self) {
        let Some(target) = &self.detail_exec_target else {
            self.detail_terminal_button.set_visible(false);
            self.detail_terminal_button.set_sensitive(false);
            return;
        };

        let available = !target.containers.is_empty();
        self.detail_terminal_button.set_visible(true);
        self.detail_terminal_button.set_sensitive(available);
    }

    #[cfg(target_os = "windows")]
    pub(super) fn sync_terminal_controls(&self) {
        self.detail_terminal_button.set_visible(false);
        self.detail_terminal_button.set_sensitive(false);
    }

    pub(super) fn append_log_line(&self, line: &str) {
        insert_ansi_line(&self.detail_log_buffer, line);
        let mut iter = self.detail_log_buffer.end_iter();
        self.detail_log_buffer.insert(&mut iter, "\n");
        let mark =
            self.detail_log_buffer
                .create_mark(None, &self.detail_log_buffer.end_iter(), false);
        self.detail_log_view.scroll_mark_onscreen(&mark);
        self.detail_log_buffer.delete_mark(&mark);
    }

    #[cfg(not(target_os = "windows"))]
    pub(super) fn feed_terminal_event(&self, token: u64, event: PodExecEvent) {
        let text = match event {
            PodExecEvent::Stdout(text) | PodExecEvent::Stderr(text) => text,
        };
        if let Some(session) = self.terminal_sessions.get(&token) {
            session.view.feed(text.as_bytes());
        }
    }

    #[cfg(target_os = "windows")]
    pub(super) fn feed_terminal_event(&self, _token: u64, _event: PodExecEvent) {}

    #[cfg(not(target_os = "windows"))]
    pub(super) fn show_terminal_error(&self, token: u64, error: &str) {
        if let Some(session) = self.terminal_sessions.get(&token) {
            let message = terminal_error_message(error);
            session.view.feed(format!("\r\n{message}\r\n").as_bytes());
        }
    }

    #[cfg(target_os = "windows")]
    pub(super) fn show_terminal_error(&self, _token: u64, error: &str) {
        self.toaster
            .add_toast(adw::Toast::new(&terminal_error_message(error)));
    }

    #[cfg(not(target_os = "windows"))]
    pub(super) fn finish_terminal_session(&mut self, token: u64) {
        if let Some(session) = self.terminal_sessions.get_mut(&token) {
            session.abort_handle = None;
            session.input_tx = None;
        }
    }

    #[cfg(target_os = "windows")]
    pub(super) fn finish_terminal_session(&mut self, _token: u64) {}

    #[cfg(not(target_os = "windows"))]
    pub(super) fn send_terminal_input(&self, token: u64, text: String) {
        if let Some(input_tx) = self
            .terminal_sessions
            .get(&token)
            .and_then(|session| session.input_tx.as_ref())
        {
            let _ = input_tx.send(text.into_bytes());
        }
    }

    #[cfg(target_os = "windows")]
    pub(super) fn send_terminal_input(&self, _token: u64, _text: String) {}

    pub(super) fn show_yaml_explanation(&self, root: &<Self as Component>::Root) {
        let explanation = build_yaml_explanation_content(
            &text_buffer_text(&self.detail_yaml_buffer),
            self.detail_target.as_ref(),
        );
        let dialog = adw::Dialog::builder()
            .title("YAML Explanation")
            .content_width(640)
            .content_height(620)
            .build();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());

        let scrolled = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();
        scrolled.set_child(Some(&explanation));
        toolbar.set_content(Some(&scrolled));
        dialog.set_child(Some(&toolbar));
        dialog.present(Some(root));
    }
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
    dropdown.set_tooltip_text(Some("Container"));
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
