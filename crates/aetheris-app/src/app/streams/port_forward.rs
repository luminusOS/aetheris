use super::super::commands::*;
use super::super::utils::*;
use super::super::*;

impl App {
    pub(crate) fn start_pod_port_forward(&mut self, sender: ComponentSender<Self>) {
        let Some(target) = self.detail.port_forward_target.clone() else {
            self.toaster.add_toast(adw::Toast::new(&tr(
                "Port forwarding is available for Pods.",
            )));
            return;
        };

        let local_port = self.detail.port_local_spin.value_as_int().clamp(0, 65535) as u16;
        let remote_port = self.detail.port_remote_spin.value_as_int().clamp(1, 65535) as u16;
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

    pub(crate) fn stop_port_forward(&mut self) {
        if let Some(handle) = self.port_forward_abort_handle.take() {
            handle.abort();
        }
        self.port_forwarding = false;
    }

    pub(crate) fn sync_port_forward_controls(&self) {
        let available = self.detail.port_forward_target.is_some();
        self.detail
            .port_local_spin
            .set_sensitive(available && !self.port_forwarding);
        self.detail
            .port_remote_spin
            .set_sensitive(available && !self.port_forwarding);
        self.detail
            .port_start_button
            .set_sensitive(available && !self.port_forwarding);
        self.detail
            .port_stop_button
            .set_sensitive(self.port_forwarding);

        if !available {
            self.detail
                .port_status_label
                .set_label(&tr("Port forwarding is available for Pods."));
        } else if self.port_forwarding {
            self.detail
                .port_status_label
                .set_label(&tr("Starting port-forward..."));
        } else {
            self.detail
                .port_status_label
                .set_label(&tr("Choose local and remote ports to forward this Pod."));
        }
    }

    pub(crate) fn handle_port_forward_event(&self, event: PodPortForwardEvent) {
        match event {
            PodPortForwardEvent::Ready { local_port } => {
                self.detail.port_status_label.set_label(&tr_format(
                    "Forwarding 127.0.0.1:{local_port} to the selected Pod.",
                    &[("{local_port}", local_port.to_string())],
                ));
            }
            PodPortForwardEvent::ConnectionOpened => {
                self.detail
                    .port_status_label
                    .set_label(&tr("Port-forward connection active."));
            }
            PodPortForwardEvent::ConnectionClosed => {
                self.detail
                    .port_status_label
                    .set_label(&tr("Waiting for the next local connection..."));
            }
        }
    }
}
