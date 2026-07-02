use anyhow::{bail, Context as AnyhowContext, Result};
use k8s_openapi::api::core::v1::Pod;
use kube::Api;
use tokio::io::copy_bidirectional;
use tokio::net::TcpListener;

use crate::{KubeSession, PodPortForwardEvent, PodPortForwardRequest};

impl KubeSession {
    pub async fn port_forward_pod<F>(
        &self,
        request: PodPortForwardRequest,
        mut on_event: F,
    ) -> Result<()>
    where
        F: FnMut(PodPortForwardEvent) + Send + 'static,
    {
        if request.remote_port == 0 {
            bail!("remote port is required");
        }

        let listener = TcpListener::bind(("127.0.0.1", request.local_port))
            .await
            .with_context(|| format!("failed to bind local port {}", request.local_port))?;
        let local_port = listener
            .local_addr()
            .context("failed to read local listener address")?
            .port();
        on_event(PodPortForwardEvent::Ready { local_port });

        loop {
            let (mut local_stream, _) = listener
                .accept()
                .await
                .context("failed to accept local port-forward connection")?;
            on_event(PodPortForwardEvent::ConnectionOpened);

            let pods: Api<Pod> = Api::namespaced(self.client.clone(), &request.namespace);
            let mut forwarder = pods
                .portforward(&request.pod, &[request.remote_port])
                .await
                .with_context(|| {
                    format!(
                        "failed to open port-forward to pod {}:{}",
                        request.pod, request.remote_port
                    )
                })?;
            let mut remote_stream = forwarder
                .take_stream(request.remote_port)
                .context("failed to open remote port stream")?;
            let remote_error = forwarder
                .take_error(request.remote_port)
                .context("failed to open remote port error stream")?;

            tokio::select! {
                result = copy_bidirectional(&mut local_stream, &mut remote_stream) => {
                    result.context("failed to proxy port-forward connection")?;
                }
                error = remote_error => {
                    if let Some(error) = error {
                        bail!("port-forward failed: {error}");
                    }
                }
            }

            forwarder.abort();
            let _ = forwarder.join().await;
            on_event(PodPortForwardEvent::ConnectionClosed);
        }
    }
}
