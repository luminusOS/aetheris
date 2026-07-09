use std::collections::BTreeSet;

use anyhow::{Context as AnyhowContext, Result};
use kube::Client;
use kube::config::{Config, KubeConfigOptions, Kubeconfig};

use crate::kubeconfig::{cluster_server, cluster_skips_tls_verify, server_host};
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
                let insecure_skip_tls_verify = context
                    .map(|context| cluster_skips_tls_verify(&self.kubeconfig, &context.cluster))
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
                    insecure_skip_tls_verify,
                }
            })
            .collect()
    }

    /// Default-namespace guess for the kubeconfig's current context, used
    /// only as a seed before any specific cluster has been connected to.
    pub fn namespaces(&self) -> Vec<String> {
        self.namespace_for_context(
            self.kubeconfig
                .current_context
                .as_deref()
                .unwrap_or_default(),
        )
    }

    /// Default-namespace guess for one specific context, used as a fallback
    /// when live namespace listing fails (e.g. RBAC denies `list
    /// namespaces` cluster-wide) for that context's cluster. Only looks at
    /// that one context's own `namespace:` field — other contexts in the
    /// kubeconfig (including ones for entirely different clusters) must
    /// never leak into this cluster's namespace list.
    pub fn namespace_for_context(&self, context: &str) -> Vec<String> {
        let mut namespaces = BTreeSet::from([String::from("default")]);

        if let Some(named_context) = self
            .kubeconfig
            .contexts
            .iter()
            .find(|named_context| named_context.name == context)
            && let Some(context) = &named_context.context
            && let Some(namespace) = &context.namespace
        {
            namespaces.insert(namespace.clone());
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
        let server = config.cluster_url.to_string();
        let client = Client::try_from(config).context("failed to create Kubernetes client")?;
        Ok(KubeSession {
            context: context.to_owned(),
            client,
            server,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use kube::config::{Context as KubeContext, NamedContext};

    use super::*;

    fn context_with_namespace(name: &str, namespace: &str) -> NamedContext {
        NamedContext {
            name: name.to_owned(),
            context: Some(KubeContext {
                namespace: Some(namespace.to_owned()),
                ..KubeContext::default()
            }),
            other: BTreeMap::new(),
        }
    }

    #[test]
    fn namespace_for_context_does_not_leak_other_contexts_namespaces() {
        let manager = KubeManager {
            kubeconfig: Kubeconfig {
                contexts: vec![
                    context_with_namespace("local", "default"),
                    context_with_namespace(
                        "payroll-hml/api-example-cluster-example-com:6443/user",
                        "payroll-hml",
                    ),
                ],
                current_context: Some(String::from("local")),
                ..Kubeconfig::default()
            },
        };

        assert_eq!(
            manager.namespace_for_context("local"),
            vec![String::from("default")]
        );
    }

    #[test]
    fn namespace_for_context_includes_that_contexts_own_namespace() {
        let manager = KubeManager {
            kubeconfig: Kubeconfig {
                contexts: vec![context_with_namespace("prod", "billing")],
                current_context: Some(String::from("prod")),
                ..Kubeconfig::default()
            },
        };

        assert_eq!(
            manager.namespace_for_context("prod"),
            vec![String::from("billing"), String::from("default")]
        );
    }

    #[test]
    fn namespace_for_context_falls_back_to_default_when_context_unknown() {
        let manager = KubeManager {
            kubeconfig: Kubeconfig {
                contexts: vec![context_with_namespace("prod", "billing")],
                current_context: Some(String::from("prod")),
                ..Kubeconfig::default()
            },
        };

        assert_eq!(
            manager.namespace_for_context("unknown"),
            vec![String::from("default")]
        );
    }

    #[test]
    fn namespaces_uses_current_context_only() {
        let manager = KubeManager {
            kubeconfig: Kubeconfig {
                contexts: vec![
                    context_with_namespace("local", "default"),
                    context_with_namespace(
                        "payroll-hml/api-example-cluster-example-com:6443/user",
                        "payroll-hml",
                    ),
                ],
                current_context: Some(String::from("local")),
                ..Kubeconfig::default()
            },
        };

        assert_eq!(manager.namespaces(), vec![String::from("default")]);
    }
}
