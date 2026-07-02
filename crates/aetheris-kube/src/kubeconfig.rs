use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context as AnyhowContext, Result};
use kube::config::{
    AuthInfo, Cluster, Context as KubeContext, Kubeconfig, NamedAuthInfo, NamedCluster,
    NamedContext,
};
use secrecy::SecretString;

use crate::{AddClusterRequest, KubeManager};

impl KubeManager {
    pub fn add_token_cluster(request: AddClusterRequest) -> Result<PathBuf> {
        let request = normalize_add_cluster_request(request)?;
        let path = kubeconfig_write_path()?;
        let mut kubeconfig = if path.exists() {
            Kubeconfig::read_from(&path)
                .with_context(|| format!("failed to read {}", path.display()))?
        } else {
            Kubeconfig::default()
        };

        let credential_source_name = request
            .original_context_name
            .clone()
            .unwrap_or_else(|| request.context_name.clone());

        let existing_user_name = kubeconfig
            .contexts
            .iter()
            .find(|context| context.name == credential_source_name)
            .and_then(|context| context.context.as_ref())
            .and_then(|context| context.user.clone());

        if request.bearer_token.is_empty() && existing_user_name.is_none() {
            bail!("bearer token is required");
        }

        let user_name = existing_user_name
            .clone()
            .unwrap_or_else(|| format!("{}-user", request.context_name));

        upsert_named_cluster(
            &mut kubeconfig.clusters,
            NamedCluster {
                name: request.context_name.clone(),
                cluster: Some(Cluster {
                    server: Some(request.server),
                    insecure_skip_tls_verify: Some(request.insecure_skip_tls_verify)
                        .filter(|enabled| *enabled),
                    certificate_authority_data: request.certificate_authority_data,
                    ..Cluster::default()
                }),
                other: BTreeMap::new(),
            },
        );
        // A blank token on an existing cluster means "leave credentials
        // alone" (the edit dialog never re-shows secrets). Only touch the
        // auth info when a new token was actually entered, so cert-based or
        // exec-based auth imported from another kubeconfig isn't clobbered.
        if !request.bearer_token.is_empty() {
            upsert_named_auth_info(
                &mut kubeconfig.auth_infos,
                NamedAuthInfo {
                    name: user_name.clone(),
                    auth_info: Some(AuthInfo {
                        token: Some(SecretString::new(request.bearer_token.into())),
                        ..AuthInfo::default()
                    }),
                    other: BTreeMap::new(),
                },
            );
        }
        upsert_named_context(
            &mut kubeconfig.contexts,
            NamedContext {
                name: request.context_name.clone(),
                context: Some(KubeContext {
                    cluster: request.context_name.clone(),
                    user: Some(user_name),
                    namespace: Some(String::from("default")),
                    ..KubeContext::default()
                }),
                other: BTreeMap::new(),
            },
        );

        // Renaming a cluster creates fresh entries under the new name above;
        // drop the stale ones left behind under the old name so it doesn't
        // linger as a duplicate in the cluster list.
        if let Some(original_name) = request.original_context_name.as_deref() {
            if original_name != request.context_name {
                kubeconfig
                    .contexts
                    .retain(|context| context.name != original_name);
                kubeconfig
                    .clusters
                    .retain(|cluster| cluster.name != original_name);
            }
        }

        kubeconfig.current_context = Some(request.context_name);
        kubeconfig.api_version = Some(String::from("v1"));
        kubeconfig.kind = Some(String::from("Config"));

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let yaml = serde_yaml::to_string(&kubeconfig).context("failed to serialize kubeconfig")?;
        fs::write(&path, yaml).with_context(|| format!("failed to write {}", path.display()))?;

        Ok(path)
    }

    pub fn import_kubeconfig(path: PathBuf) -> Result<(PathBuf, Vec<String>)> {
        let imported = Kubeconfig::read_from(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if imported.contexts.is_empty() {
            bail!("selected kubeconfig has no contexts");
        }
        let imported_contexts = imported
            .contexts
            .iter()
            .map(|context| context.name.clone())
            .collect::<Vec<_>>();

        let target = kubeconfig_write_path()?;
        let mut kubeconfig = if target.exists() {
            Kubeconfig::read_from(&target)
                .with_context(|| format!("failed to read {}", target.display()))?
        } else {
            Kubeconfig::default()
        };

        kubeconfig = Kubeconfig::merge(kubeconfig, imported)
            .context("failed to merge imported kubeconfig")?;
        if kubeconfig.current_context.is_none() {
            if let Some(context) = kubeconfig.contexts.first() {
                kubeconfig.current_context = Some(context.name.clone());
            }
        }
        kubeconfig.api_version = Some(String::from("v1"));
        kubeconfig.kind = Some(String::from("Config"));

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let yaml = serde_yaml::to_string(&kubeconfig).context("failed to serialize kubeconfig")?;
        fs::write(&target, yaml)
            .with_context(|| format!("failed to write {}", target.display()))?;

        Ok((target, imported_contexts))
    }
}

fn normalize_add_cluster_request(mut request: AddClusterRequest) -> Result<AddClusterRequest> {
    request.context_name = request.context_name.trim().to_owned();
    request.server = request.server.trim().to_owned();
    request.bearer_token = request.bearer_token.trim().to_owned();
    request.certificate_authority_data = request.certificate_authority_data.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    });

    if request.context_name.is_empty() {
        bail!("cluster name is required");
    }
    if request.server.is_empty() {
        bail!("API server URL is required");
    }
    if !(request.server.starts_with("https://") || request.server.starts_with("http://")) {
        bail!("API server URL must start with http:// or https://");
    }

    Ok(request)
}

fn kubeconfig_write_path() -> Result<PathBuf> {
    if let Some(value) = std::env::var_os("KUBECONFIG") {
        let paths = std::env::split_paths(&value)
            .filter(|path| !path.as_os_str().is_empty())
            .collect::<Vec<_>>();
        if let [path] = paths.as_slice() {
            return Ok(path.clone());
        }
    }

    Ok(dirs::home_dir()
        .context("failed to locate home directory")?
        .join(".kube")
        .join("config"))
}

fn upsert_named_cluster(items: &mut Vec<NamedCluster>, item: NamedCluster) {
    if let Some(existing) = items.iter_mut().find(|existing| existing.name == item.name) {
        *existing = item;
    } else {
        items.push(item);
    }
}

fn upsert_named_auth_info(items: &mut Vec<NamedAuthInfo>, item: NamedAuthInfo) {
    if let Some(existing) = items.iter_mut().find(|existing| existing.name == item.name) {
        *existing = item;
    } else {
        items.push(item);
    }
}

fn upsert_named_context(items: &mut Vec<NamedContext>, item: NamedContext) {
    if let Some(existing) = items.iter_mut().find(|existing| existing.name == item.name) {
        *existing = item;
    } else {
        items.push(item);
    }
}

pub(crate) fn cluster_server(kubeconfig: &Kubeconfig, cluster_name: &str) -> Option<String> {
    kubeconfig
        .clusters
        .iter()
        .find(|cluster| cluster.name == cluster_name)
        .and_then(|cluster| cluster.cluster.as_ref())
        .and_then(|cluster| cluster.server.clone())
}

pub(crate) fn server_host(server: &str) -> String {
    let without_scheme = server
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(server);
    let authority = without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .trim();

    if let Some(rest) = authority.strip_prefix('[') {
        return rest
            .split_once(']')
            .map(|(host, _)| host.to_owned())
            .unwrap_or_else(|| authority.to_owned());
    }

    authority.split(':').next().unwrap_or(authority).to_owned()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use kube::config::{
        AuthInfo, Cluster, Context as KubeContext, Kubeconfig, NamedAuthInfo, NamedCluster,
        NamedContext,
    };
    use secrecy::{ExposeSecret, SecretString};

    use super::server_host;
    use crate::{AddClusterRequest, KubeManager};

    // `add_token_cluster` reads the process-wide `KUBECONFIG` env var, so tests
    // that set it must not run concurrently with each other.
    static KUBECONFIG_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn server_host_extracts_hostname() {
        assert_eq!(
            server_host("https://console-rnds.saude.gov.br/k8s/clusters/local"),
            "console-rnds.saude.gov.br"
        );
        assert_eq!(server_host("https://127.0.0.1:6443"), "127.0.0.1");
        assert_eq!(server_host("https://[::1]:6443"), "::1");
    }

    #[test]
    fn add_token_cluster_reuses_existing_token_when_blank() {
        let _guard = KUBECONFIG_ENV_LOCK.lock().unwrap();
        let path = test_kubeconfig_path("aetheris-kube-test");
        unsafe {
            std::env::set_var("KUBECONFIG", &path);
        }

        let initial = AddClusterRequest {
            context_name: String::from("edit-test"),
            server: String::from("https://api.example.com:6443"),
            bearer_token: String::from("sha256~original"),
            certificate_authority_data: None,
            insecure_skip_tls_verify: false,
            original_context_name: None,
        };
        KubeManager::add_token_cluster(initial).expect("initial add should succeed");

        let edit = AddClusterRequest {
            context_name: String::from("edit-test"),
            server: String::from("https://api.example.com:6443"),
            bearer_token: String::new(),
            certificate_authority_data: None,
            insecure_skip_tls_verify: false,
            original_context_name: Some(String::from("edit-test")),
        };
        KubeManager::add_token_cluster(edit)
            .expect("editing without re-entering the token should reuse the existing one");

        let kubeconfig = Kubeconfig::read_from(&path).expect("kubeconfig should be readable");
        let token = kubeconfig
            .auth_infos
            .iter()
            .find(|auth| auth.name == "edit-test-user")
            .and_then(|auth| auth.auth_info.as_ref())
            .and_then(|info| info.token.as_ref())
            .map(|token| token.expose_secret().to_owned());

        assert_eq!(token.as_deref(), Some("sha256~original"));

        unsafe {
            std::env::remove_var("KUBECONFIG");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn add_token_cluster_reuses_token_for_non_conventional_user_name() {
        let _guard = KUBECONFIG_ENV_LOCK.lock().unwrap();
        let path = test_kubeconfig_path("aetheris-kube-test-imported");

        let mut kubeconfig = Kubeconfig::default();
        kubeconfig.clusters.push(NamedCluster {
            name: String::from("imported-test"),
            cluster: Some(Cluster {
                server: Some(String::from("https://api.example.com:6443")),
                ..Cluster::default()
            }),
            other: Default::default(),
        });
        kubeconfig.auth_infos.push(NamedAuthInfo {
            name: String::from("admin"),
            auth_info: Some(AuthInfo {
                token: Some(SecretString::new(String::from("sha256~imported").into())),
                ..AuthInfo::default()
            }),
            other: Default::default(),
        });
        kubeconfig.contexts.push(NamedContext {
            name: String::from("imported-test"),
            context: Some(KubeContext {
                cluster: String::from("imported-test"),
                user: Some(String::from("admin")),
                ..KubeContext::default()
            }),
            other: Default::default(),
        });
        let yaml = serde_yaml::to_string(&kubeconfig).unwrap();
        std::fs::write(&path, yaml).unwrap();

        unsafe {
            std::env::set_var("KUBECONFIG", &path);
        }

        let edit = AddClusterRequest {
            context_name: String::from("imported-test"),
            server: String::from("https://api.example.com:6443"),
            bearer_token: String::new(),
            certificate_authority_data: None,
            insecure_skip_tls_verify: false,
            original_context_name: Some(String::from("imported-test")),
        };
        KubeManager::add_token_cluster(edit).expect(
            "editing an imported cluster without re-entering the token should reuse the existing one",
        );

        let kubeconfig = Kubeconfig::read_from(&path).expect("kubeconfig should be readable");
        let token = kubeconfig
            .auth_infos
            .iter()
            .find(|auth| auth.name == "admin")
            .and_then(|auth| auth.auth_info.as_ref())
            .and_then(|info| info.token.as_ref())
            .map(|token| token.expose_secret().to_owned());

        assert_eq!(token.as_deref(), Some("sha256~imported"));

        unsafe {
            std::env::remove_var("KUBECONFIG");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn add_token_cluster_preserves_non_token_auth_when_blank() {
        let _guard = KUBECONFIG_ENV_LOCK.lock().unwrap();
        let path = test_kubeconfig_path("aetheris-kube-test-cert");

        let mut kubeconfig = Kubeconfig::default();
        kubeconfig.clusters.push(NamedCluster {
            name: String::from("cert-test"),
            cluster: Some(Cluster {
                server: Some(String::from("https://api.example.com:6443")),
                ..Cluster::default()
            }),
            other: Default::default(),
        });
        kubeconfig.auth_infos.push(NamedAuthInfo {
            name: String::from("cert-user"),
            auth_info: Some(AuthInfo {
                client_certificate_data: Some(String::from("cert-data")),
                client_key_data: Some(SecretString::new(String::from("key-data").into())),
                ..AuthInfo::default()
            }),
            other: Default::default(),
        });
        kubeconfig.contexts.push(NamedContext {
            name: String::from("cert-test"),
            context: Some(KubeContext {
                cluster: String::from("cert-test"),
                user: Some(String::from("cert-user")),
                ..KubeContext::default()
            }),
            other: Default::default(),
        });
        let yaml = serde_yaml::to_string(&kubeconfig).unwrap();
        std::fs::write(&path, yaml).unwrap();

        unsafe {
            std::env::set_var("KUBECONFIG", &path);
        }

        let edit = AddClusterRequest {
            context_name: String::from("cert-test"),
            server: String::from("https://api.example.com:6443"),
            bearer_token: String::new(),
            certificate_authority_data: None,
            insecure_skip_tls_verify: false,
            original_context_name: Some(String::from("cert-test")),
        };
        KubeManager::add_token_cluster(edit)
            .expect("editing a cert-based cluster without entering a token should not require one");

        let kubeconfig = Kubeconfig::read_from(&path).expect("kubeconfig should be readable");
        let auth_info = kubeconfig
            .auth_infos
            .iter()
            .find(|auth| auth.name == "cert-user")
            .and_then(|auth| auth.auth_info.as_ref())
            .expect("cert-user auth info should still exist");

        assert_eq!(
            auth_info.client_certificate_data.as_deref(),
            Some("cert-data")
        );
        assert_eq!(
            auth_info
                .client_key_data
                .as_ref()
                .map(|key| key.expose_secret()),
            Some("key-data")
        );
        assert!(auth_info.token.is_none());

        unsafe {
            std::env::remove_var("KUBECONFIG");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn add_token_cluster_reuses_token_and_drops_old_entry_when_renamed() {
        let _guard = KUBECONFIG_ENV_LOCK.lock().unwrap();
        let path = test_kubeconfig_path("aetheris-kube-test-rename");
        unsafe {
            std::env::set_var("KUBECONFIG", &path);
        }

        let initial = AddClusterRequest {
            context_name: String::from("old-name"),
            server: String::from("https://api.example.com:6443"),
            bearer_token: String::from("sha256~original"),
            certificate_authority_data: None,
            insecure_skip_tls_verify: false,
            original_context_name: None,
        };
        KubeManager::add_token_cluster(initial).expect("initial add should succeed");

        let rename = AddClusterRequest {
            context_name: String::from("new-name"),
            server: String::from("https://api.example.com:6443"),
            bearer_token: String::new(),
            certificate_authority_data: None,
            insecure_skip_tls_verify: false,
            original_context_name: Some(String::from("old-name")),
        };
        KubeManager::add_token_cluster(rename)
            .expect("renaming without re-entering the token should reuse the existing one");

        let kubeconfig = Kubeconfig::read_from(&path).expect("kubeconfig should be readable");

        assert!(
            kubeconfig
                .contexts
                .iter()
                .all(|context| context.name != "old-name"),
            "the old context name should not linger as a duplicate"
        );
        assert!(
            kubeconfig
                .clusters
                .iter()
                .all(|cluster| cluster.name != "old-name"),
            "the old cluster name should not linger as a duplicate"
        );

        let token = kubeconfig
            .auth_infos
            .iter()
            .find(|auth| auth.name == "old-name-user")
            .and_then(|auth| auth.auth_info.as_ref())
            .and_then(|info| info.token.as_ref())
            .map(|token| token.expose_secret().to_owned());
        assert_eq!(token.as_deref(), Some("sha256~original"));

        assert_eq!(kubeconfig.current_context.as_deref(), Some("new-name"));

        unsafe {
            std::env::remove_var("KUBECONFIG");
        }
        let _ = std::fs::remove_file(&path);
    }

    fn test_kubeconfig_path(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "{prefix}-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
