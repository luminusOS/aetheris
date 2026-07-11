use super::super::ansi::*;
use super::super::commands::*;
use super::super::utils::*;
use super::super::*;

impl App {
    pub(crate) fn update_log_target_containers(&mut self, detail: &ObjectDetail) {
        let mut selected = None;
        if let Some(target) = &mut self.detail.log_target {
            target.containers.clone_from(&detail.containers);
            selected = Some(default_log_container_index(&target.pod, &target.containers));
        }
        self.sync_log_controls_with_selection(selected);
    }

    pub(crate) fn start_pod_logs(&mut self, sender: ComponentSender<Self>) {
        let Some(target) = self.detail.log_target.clone() else {
            self.toaster
                .add_toast(adw::Toast::new(&tr("Logs are available for Pods.")));
            return;
        };
        let Some(container) = selected_log_container(&self.detail.log_container_dropdown, &target)
        else {
            self.toaster.add_toast(adw::Toast::new(&tr(
                "Select a container before starting logs.",
            )));
            return;
        };

        self.stop_log_stream();
        self.log_stream_token = self.log_stream_token.saturating_add(1);
        let token = self.log_stream_token;
        let follow = self.detail.log_follow_check.is_active();
        let timestamps = self.detail.log_timestamps_check.is_active();
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

    pub(crate) fn maybe_start_visible_logs(&mut self, sender: ComponentSender<Self>) {
        if self.log_streaming
            || self.detail.stack.visible_child_name().as_deref() != Some("logs")
            || self
                .detail
                .log_target
                .as_ref()
                .is_none_or(|target| target.containers.is_empty())
        {
            return;
        }

        self.start_pod_logs(sender);
    }

    pub(crate) fn stop_log_stream(&mut self) {
        if let Some(handle) = self.log_abort_handle.take() {
            handle.abort();
        }
        self.log_streaming = false;
    }

    pub(crate) fn sync_log_controls(&self) {
        self.sync_log_controls_with_selection(None);
    }

    pub(crate) fn sync_log_controls_with_selection(&self, selected_override: Option<usize>) {
        let Some(target) = &self.detail.log_target else {
            self.detail
                .log_container_dropdown
                .set_model(Some(&gtk::StringList::new(&[&tr("No containers")])));
            self.detail.log_container_dropdown.set_selected(0);
            self.detail.log_container_dropdown.set_sensitive(false);
            self.detail.log_follow_check.set_sensitive(false);
            self.detail.log_timestamps_check.set_sensitive(false);
            self.detail.log_start_button.set_sensitive(false);
            self.detail.log_stop_button.set_sensitive(false);
            self.detail
                .log_status_label
                .set_label(&tr("Logs are available for Pods."));
            return;
        };

        let labels = if target.containers.is_empty() {
            vec![tr("No containers")]
        } else {
            target.containers.clone()
        };
        let selected = if let Some(selected) = selected_override {
            selected.min(labels.len().saturating_sub(1))
        } else if target.containers.is_empty() {
            0
        } else {
            (self.detail.log_container_dropdown.selected() as usize)
                .min(target.containers.len().saturating_sub(1))
        };
        let refs = labels.iter().map(String::as_str).collect::<Vec<_>>();
        self.detail
            .log_container_dropdown
            .set_model(Some(&gtk::StringList::new(&refs)));
        self.detail
            .log_container_dropdown
            .set_selected(selected as u32);
        self.detail
            .log_container_dropdown
            .set_sensitive(!target.containers.is_empty() && !self.log_streaming);
        self.detail
            .log_follow_check
            .set_sensitive(!target.containers.is_empty() && !self.log_streaming);
        self.detail
            .log_timestamps_check
            .set_sensitive(!target.containers.is_empty() && !self.log_streaming);
        self.detail
            .log_start_button
            .set_sensitive(!target.containers.is_empty() && !self.log_streaming);
        self.detail
            .log_stop_button
            .set_sensitive(self.log_streaming);
        let status = if self.log_streaming {
            tr("Streaming pod logs...")
        } else if target.containers.is_empty() {
            tr("This Pod has no containers in its spec.")
        } else {
            tr("Select a container and start logs.")
        };
        self.detail.log_status_label.set_label(&status);
    }

    pub(crate) fn append_log_line(&self, line: &str) {
        insert_ansi_line(&self.detail.log_buffer, line);
        let mut iter = self.detail.log_buffer.end_iter();
        self.detail.log_buffer.insert(&mut iter, "\n");
        let mark =
            self.detail
                .log_buffer
                .create_mark(None, &self.detail.log_buffer.end_iter(), false);
        self.detail.log_view.scroll_mark_onscreen(&mark);
        self.detail.log_buffer.delete_mark(&mark);
    }
}
