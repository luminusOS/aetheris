use anyhow::{bail, Context as AnyhowContext, Result};
use k8s_openapi::api::core::v1::Pod;
use kube::api::AttachParams;
use kube::Api;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

use crate::{KubeSession, PodExecEvent, PodExecRequest};

impl KubeSession {
    pub async fn exec_pod<F>(&self, request: PodExecRequest, mut on_event: F) -> Result<()>
    where
        F: FnMut(PodExecEvent) + Send + 'static,
    {
        if request.command.is_empty() {
            bail!("command is required");
        }

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &request.namespace);
        let mut params = AttachParams::default()
            .stdin(false)
            .stdout(true)
            .stderr(true)
            .tty(false)
            .max_stdout_buf_size(64 * 1024)
            .max_stderr_buf_size(64 * 1024);
        if let Some(container) = request
            .container
            .as_deref()
            .filter(|container| !container.is_empty())
        {
            params = params.container(container);
        }

        let mut process = pods
            .exec(&request.pod, request.command.clone(), &params)
            .await
            .with_context(|| {
                format!(
                    "Could not execute command in Pod {} in namespace {} using context {}.",
                    request.pod, request.namespace, self.context
                )
            })?;
        let stdout = process
            .stdout()
            .context("failed to open exec stdout stream")?;
        let stderr = process
            .stderr()
            .context("failed to open exec stderr stream")?;
        let status = process.take_status();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let stdout_task = tokio::spawn(read_exec_stream(stdout, ExecStream::Stdout, tx.clone()));
        let stderr_task = tokio::spawn(read_exec_stream(stderr, ExecStream::Stderr, tx));

        while let Some(event) = rx.recv().await {
            on_event(event?);
        }

        stdout_task
            .await
            .context("failed to join exec stdout reader")??;
        stderr_task
            .await
            .context("failed to join exec stderr reader")??;
        let status = match status {
            Some(status) => status.await,
            None => None,
        };
        process
            .join()
            .await
            .context("failed to finish exec session")?;

        if let Some(status) = status {
            if status.status.as_deref() == Some("Failure") {
                bail!(
                    "{}",
                    status
                        .message
                        .unwrap_or_else(|| String::from("command failed"))
                );
            }
        }

        Ok(())
    }

    pub async fn terminal_pod<F>(
        &self,
        request: PodExecRequest,
        mut input_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
        mut on_event: F,
    ) -> Result<()>
    where
        F: FnMut(PodExecEvent) + Send + 'static,
    {
        if request.command.is_empty() {
            bail!("command is required");
        }

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &request.namespace);
        let mut params = AttachParams::interactive_tty()
            .max_stdin_buf_size(64 * 1024)
            .max_stdout_buf_size(64 * 1024);
        if let Some(container) = request
            .container
            .as_deref()
            .filter(|container| !container.is_empty())
        {
            params = params.container(container);
        }

        let mut process = pods
            .exec(&request.pod, request.command.clone(), &params)
            .await
            .with_context(|| {
                format!(
                    "Could not open a terminal in Pod {} in namespace {} using context {}.",
                    request.pod, request.namespace, self.context
                )
            })?;
        let mut stdin = process
            .stdin()
            .context("failed to open terminal stdin stream")?;
        let mut stdout = process
            .stdout()
            .context("failed to open terminal stdout stream")?;
        let status = process.take_status();

        let input_task = tokio::spawn(async move {
            while let Some(bytes) = input_rx.recv().await {
                stdin.write_all(&bytes).await?;
            }
            stdin.shutdown().await
        });

        let mut buffer = [0_u8; 8192];
        loop {
            let count = stdout.read(&mut buffer).await?;
            if count == 0 {
                break;
            }
            on_event(PodExecEvent::Stdout(
                String::from_utf8_lossy(&buffer[..count]).to_string(),
            ));
        }

        input_task.abort();
        let _ = input_task.await;
        let status = match status {
            Some(status) => status.await,
            None => None,
        };
        process
            .join()
            .await
            .context("failed to finish terminal session")?;

        if let Some(status) = status {
            if status.status.as_deref() == Some("Failure") {
                bail!(
                    "{}",
                    status
                        .message
                        .unwrap_or_else(|| String::from("terminal session failed"))
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum ExecStream {
    Stdout,
    Stderr,
}

async fn read_exec_stream<R>(
    mut reader: R,
    stream: ExecStream,
    tx: tokio::sync::mpsc::UnboundedSender<Result<PodExecEvent>>,
) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        let text = String::from_utf8_lossy(&buffer[..count]).to_string();
        let event = match stream {
            ExecStream::Stdout => PodExecEvent::Stdout(text),
            ExecStream::Stderr => PodExecEvent::Stderr(text),
        };
        if tx.send(Ok(event)).is_err() {
            break;
        }
    }
    Ok(())
}
