use anyhow::{Context as AnyhowContext, Result};
use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::Api;
use kube::api::LogParams;

use crate::{KubeSession, PodLogRequest};

impl KubeSession {
    pub async fn stream_pod_logs<F>(&self, request: PodLogRequest, mut on_line: F) -> Result<()>
    where
        F: FnMut(String) + Send + 'static,
    {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &request.namespace);
        let params = LogParams {
            container: request.container,
            follow: request.follow,
            timestamps: request.timestamps,
            tail_lines: request.tail_lines,
            ..LogParams::default()
        };
        let mut lines = pods
            .log_stream(&request.pod, &params)
            .await
            .with_context(|| {
                format!(
                    "Could not open logs for Pod {} in namespace {} using context {}.",
                    request.pod, request.namespace, self.context
                )
            })?
            .lines();

        while let Some(line) = lines.try_next().await.with_context(|| {
            format!(
                "Could not read log stream for Pod {} in namespace {}.",
                request.pod, request.namespace
            )
        })? {
            on_line(line);
        }

        Ok(())
    }
}
