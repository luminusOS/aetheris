use std::collections::BTreeSet;

use anyhow::{Context as AnyhowContext, Result};
use kube::config::{Config, KubeConfigOptions, Kubeconfig};
use kube::Client;

use crate::kubeconfig::{cluster_server, server_host};
use crate::{ContextInfo, KubeSession};

#[derive(Debug, Clone)]
pub struct KubeManager {
    pub(crate) kubeconfig: Kubeconfig,
}

impl KubeManager {
    pub fn load() -> Result<Self> {
        let kubeconfig = Kubeconfig::read().context("failed to read kubeconfig")?;
        Ok(Self { kubeconfig })
    }

    pub fn load_contexts(&self) -> Vec<ContextInfo> {
        let current = self.kubeconfig.current_context.as_deref();

        self.kubeconfig
            .contexts
            .iter()
            .map(|named_context| {
                let context = named_context.context.as_ref();
                let server = context
                    .and_then(|context| cluster_server(&self.kubeconfig, &context.cluster))
                    .unwrap_or_default();
                ContextInfo {
                    name: named_context.name.clone(),
                    cluster: context
                        .map(|context| context.cluster.clone())
                        .unwrap_or_default(),
                    host: server_host(&server),
                    server,
                    user: context
                        .and_then(|context| context.user.clone())
                        .unwrap_or_default(),
                    is_current: current == Some(named_context.name.as_str()),
                }
            })
            .collect()
    }

    pub fn namespaces(&self) -> Vec<String> {
        let mut namespaces = BTreeSet::from([String::from("default")]);

        for context in &self.kubeconfig.contexts {
            if let Some(context) = &context.context {
                if let Some(namespace) = &context.namespace {
                    namespaces.insert(namespace.clone());
                }
            }
        }

        namespaces.into_iter().collect()
    }

    pub async fn connect_context(&self, context: &str) -> Result<KubeSession> {
        let options = KubeConfigOptions {
            context: Some(context.to_owned()),
            ..KubeConfigOptions::default()
        };
        let config = Config::from_custom_kubeconfig(self.kubeconfig.clone(), &options)
            .await
            .with_context(|| format!("failed to build Kubernetes client for context {context}"))?;
        let client = Client::try_from(config).context("failed to create Kubernetes client")?;
        Ok(KubeSession {
            context: context.to_owned(),
            client,
        })
    }
}
