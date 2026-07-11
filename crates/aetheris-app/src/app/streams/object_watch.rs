use super::super::commands::*;
use super::super::utils::*;
use super::super::*;

impl App {
    pub(crate) fn start_object_watch(&mut self, sender: ComponentSender<Self>) {
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

    pub(crate) fn stop_object_watch(&mut self) {
        if let Some(handle) = self.object_watch_abort_handle.take() {
            handle.abort();
        }
    }
}
