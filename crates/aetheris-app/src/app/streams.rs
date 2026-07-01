use super::ansi::*;
use super::commands::*;
use super::utils::*;
use super::yaml::*;
use super::*;

impl App {
    pub(super) fn update_log_target_containers(&mut self, detail: &ObjectDetail) {
        let mut selected = None;
        if let Some(target) = &mut self.detail_log_target {
            target.containers.clone_from(&detail.containers);
            selected = Some(default_log_container_index(&target.pod, &target.containers));
        }
        self.sync_log_controls_with_selection(selected);
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
