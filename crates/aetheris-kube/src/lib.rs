use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context as AnyhowContext, Result};
use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::{Event, Namespace, Node, Pod};
use k8s_openapi::jiff::Timestamp;
use kube::api::{DeleteParams, DynamicObject, ListParams, LogParams, Patch, PatchParams};
use kube::config::{
    AuthInfo, Cluster, Config, Context as KubeContext, KubeConfigOptions, Kubeconfig,
    NamedAuthInfo, NamedCluster, NamedContext,
};
use kube::discovery::{verbs, ApiResource, Discovery, Scope};
use kube::{Api, Client, ResourceExt};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::copy_bidirectional;
use tokio::net::TcpListener;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextInfo {
    pub name: String,
    pub cluster: String,
    pub server: String,
    pub host: String,
    pub user: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodSummary {
    pub name: String,
    pub namespace: String,
    pub phase: String,
    pub node: String,
    pub age: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceScope {
    Cluster,
    Namespaced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceKind {
    pub group: String,
    pub version: String,
    pub api_version: String,
    pub kind: String,
    pub plural: String,
    pub scope: ResourceScope,
}

impl ResourceKind {
    pub fn label(&self) -> String {
        if self.group.is_empty() {
            self.kind.clone()
        } else {
            format!("{} ({})", self.kind, self.group)
        }
    }

    pub fn is_namespaced(&self) -> bool {
        self.scope == ResourceScope::Namespaced
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSummary {
    pub name: String,
    pub namespace: String,
    pub status: String,
    /// The `(ready, desired)` counts backing `status`, when the resource
    /// kind exposes one (Deployments, StatefulSets, Jobs, ...).
    pub status_ratio: Option<(i64, i64)>,
    pub api_version: String,
    pub age: String,
    pub metrics: Option<ResourceUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectDetail {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub api_version: String,
    pub kind: String,
    pub age: String,
    pub metrics: Option<ResourceUsage>,
    pub container_metrics: Vec<ContainerUsage>,
    pub yaml: String,
    pub containers: Vec<String>,
    pub related_pods: Vec<ObjectSummary>,
    pub replicas: Option<i32>,
    pub node_unschedulable: Option<bool>,
    pub conditions: Vec<ObjectCondition>,
    pub events: Vec<ObjectEvent>,
    pub events_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectCondition {
    pub type_: String,
    pub status: String,
    pub reason: String,
    pub message: String,
    pub last_transition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectEvent {
    pub type_: String,
    pub reason: String,
    pub message: String,
    pub count: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu: String,
    pub memory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerUsage {
    pub name: String,
    pub cpu: String,
    pub memory: String,
}

/// A best-effort snapshot of a cluster used by the Clusters list page.
/// Every field beyond connectivity itself is optional: metrics-server may
/// not be installed, and provider detection is a heuristic guess, not an
/// authoritative source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub version: Option<String>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddClusterRequest {
    pub context_name: String,
    pub server: String,
    pub bearer_token: String,
    pub certificate_authority_data: Option<String>,
    pub insecure_skip_tls_verify: bool,
    /// The context's name before this edit, if this request renames an
    /// existing cluster. `None` when adding a brand-new cluster.
    pub original_context_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodLogRequest {
    pub namespace: String,
    pub pod: String,
    pub container: Option<String>,
    pub follow: bool,
    pub timestamps: bool,
    pub tail_lines: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodPortForwardRequest {
    pub namespace: String,
    pub pod: String,
    pub local_port: u16,
    pub remote_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodPortForwardEvent {
    Ready { local_port: u16 },
    ConnectionOpened,
    ConnectionClosed,
}

#[derive(Debug, Clone)]
pub struct KubeManager {
    kubeconfig: Kubeconfig,
}

impl KubeManager {
    pub fn load() -> Result<Self> {
        let kubeconfig = Kubeconfig::read().context("failed to read kubeconfig")?;
        Ok(Self { kubeconfig })
    }

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

    pub fn import_kubeconfig(path: PathBuf) -> Result<PathBuf> {
        let imported = Kubeconfig::read_from(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if imported.contexts.is_empty() {
            bail!("selected kubeconfig has no contexts");
        }

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

        Ok(target)
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

fn cluster_server(kubeconfig: &Kubeconfig, cluster_name: &str) -> Option<String> {
    kubeconfig
        .clusters
        .iter()
        .find(|cluster| cluster.name == cluster_name)
        .and_then(|cluster| cluster.cluster.as_ref())
        .and_then(|cluster| cluster.server.clone())
}

fn server_host(server: &str) -> String {
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

#[derive(Clone)]
pub struct KubeSession {
    context: String,
    client: Client,
}

impl KubeSession {
    pub fn context(&self) -> &str {
        &self.context
    }

    pub async fn list_pods(&self, namespace: Option<&str>) -> Result<Vec<PodSummary>> {
        let pods: Api<Pod> = match namespace {
            Some(namespace) if !namespace.is_empty() && namespace != "all" => {
                Api::namespaced(self.client.clone(), namespace)
            }
            _ => Api::all(self.client.clone()),
        };

        let mut summaries = pods
            .list(&ListParams::default())
            .await
            .with_context(|| {
                format!(
                    "Could not list Pods {} using context {}.",
                    namespace_scope(namespace),
                    self.context
                )
            })?
            .items
            .into_iter()
            .map(pod_summary)
            .collect::<Vec<_>>();

        summaries.sort_by(|left, right| {
            left.namespace
                .cmp(&right.namespace)
                .then_with(|| left.name.cmp(&right.name))
        });

        Ok(summaries)
    }

    pub async fn discover_resources(&self) -> Result<Vec<ResourceKind>> {
        let discovery = Discovery::new(self.client.clone())
            .run()
            .await
            .with_context(|| {
                format!(
                    "Could not discover Kubernetes resource types using context {}.",
                    self.context
                )
            })?;
        let mut resources = Vec::new();

        for group in discovery.groups_alphabetical() {
            for (resource, capabilities) in group.recommended_resources() {
                if !capabilities.supports_operation(verbs::LIST)
                    || resource.plural.contains('/')
                    || resource.kind.ends_with("List")
                {
                    continue;
                }

                resources.push(ResourceKind {
                    group: resource.group,
                    version: resource.version,
                    api_version: resource.api_version,
                    kind: resource.kind,
                    plural: resource.plural,
                    scope: match capabilities.scope {
                        Scope::Cluster => ResourceScope::Cluster,
                        Scope::Namespaced => ResourceScope::Namespaced,
                    },
                });
            }
        }

        resources.sort_by(|left, right| {
            resource_group_order(left)
                .cmp(&resource_group_order(right))
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.api_version.cmp(&right.api_version))
        });

        Ok(resources)
    }

    pub async fn list_namespaces(&self) -> Result<Vec<String>> {
        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        let mut names = namespaces
            .list(&ListParams::default())
            .await
            .with_context(|| {
                format!(
                    "Could not list Namespaces at cluster scope using context {}.",
                    self.context
                )
            })?
            .items
            .into_iter()
            .map(|namespace| namespace.name_any())
            .collect::<Vec<_>>();

        names.sort();
        names.dedup();

        if names.is_empty() {
            names.push(String::from("default"));
        }

        Ok(names)
    }

    pub async fn list_objects(
        &self,
        resource: &ResourceKind,
        namespace: Option<&str>,
    ) -> Result<Vec<ObjectSummary>> {
        let api_resource = api_resource(resource);
        let objects: Api<DynamicObject> = match (resource.is_namespaced(), namespace) {
            (true, Some(namespace)) if !namespace.is_empty() && namespace != "all" => {
                Api::namespaced_with(self.client.clone(), namespace, &api_resource)
            }
            _ => Api::all_with(self.client.clone(), &api_resource),
        };

        let metrics = self
            .resource_metrics(resource, namespace)
            .await
            .unwrap_or_default();

        let mut summaries = objects
            .list(&ListParams::default())
            .await
            .with_context(|| {
                format!(
                    "Could not list {} {} using context {}.",
                    resource.kind,
                    resource_scope(resource, namespace),
                    self.context
                )
            })?
            .items
            .into_iter()
            .map(|object| object_summary(object, resource, &metrics))
            .collect::<Vec<_>>();

        summaries.sort_by(|left, right| {
            left.namespace
                .cmp(&right.namespace)
                .then_with(|| left.name.cmp(&right.name))
        });

        Ok(summaries)
    }

    pub async fn object_detail(
        &self,
        resource: &ResourceKind,
        namespace: Option<&str>,
        name: &str,
    ) -> Result<ObjectDetail> {
        let api_resource = api_resource(resource);
        let objects: Api<DynamicObject> = match (resource.is_namespaced(), namespace) {
            (true, Some(namespace)) if !namespace.is_empty() && namespace != "-" => {
                Api::namespaced_with(self.client.clone(), namespace, &api_resource)
            }
            _ => Api::all_with(self.client.clone(), &api_resource),
        };
        let object = objects.get(name).await.with_context(|| {
            format!(
                "Could not load {} {name} {} using context {}.",
                resource.kind,
                resource_scope(resource, namespace),
                self.context
            )
        })?;
        let containers = object_containers(&object, resource);
        let replicas = object_replicas(&object, resource);
        let node_unschedulable = object_node_unschedulable(&object, resource);
        let conditions = object_conditions(&object);
        let metrics = self
            .resource_metrics(resource, namespace)
            .await
            .unwrap_or_default();
        let summary = object_summary(object.clone(), resource, &metrics);
        let container_metrics = if resource.kind == "Pod" && resource.group.is_empty() {
            self.pod_container_metrics(&summary.namespace, &summary.name)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let yaml = serde_yaml::to_string(&object).context("failed to serialize object YAML")?;
        let related_pods = self
            .deployment_pods(resource, &summary.namespace, &object)
            .await
            .unwrap_or_default();
        let events = self
            .object_events(resource, &summary.namespace, name)
            .await
            .map_err(|error| error.to_string());
        let (events, events_error) = match events {
            Ok(events) => (events, None),
            Err(error) => (Vec::new(), Some(error)),
        };

        Ok(ObjectDetail {
            name: summary.name,
            namespace: summary.namespace,
            status: summary.status,
            api_version: summary.api_version,
            kind: resource.kind.clone(),
            age: summary.age,
            metrics: summary.metrics,
            container_metrics,
            yaml,
            containers,
            related_pods,
            replicas,
            node_unschedulable,
            conditions,
            events,
            events_error,
        })
    }

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

    pub async fn apply_object_yaml(
        &self,
        resource: &ResourceKind,
        namespace: Option<&str>,
        name: &str,
        yaml: &str,
    ) -> Result<ObjectDetail> {
        let api_resource = api_resource(resource);
        let objects: Api<DynamicObject> = match (resource.is_namespaced(), namespace) {
            (true, Some(namespace)) if !namespace.is_empty() && namespace != "-" => {
                Api::namespaced_with(self.client.clone(), namespace, &api_resource)
            }
            _ => Api::all_with(self.client.clone(), &api_resource),
        };
        let mut value: Value = serde_yaml::from_str(yaml).context("failed to parse YAML")?;
        sanitize_apply_value(&mut value);
        let params = PatchParams::apply("aetheris").force();
        objects
            .patch(name, &params, &Patch::Apply(&value))
            .await
            .with_context(|| {
                format!(
                    "Could not apply YAML to {} {name} {} using context {}.",
                    resource.kind,
                    resource_scope(resource, namespace),
                    self.context
                )
            })?;

        self.object_detail(resource, namespace, name).await
    }

    pub async fn create_object_yaml(
        &self,
        resource: &ResourceKind,
        default_namespace: Option<&str>,
        yaml: &str,
    ) -> Result<ObjectDetail> {
        let mut value: Value = serde_yaml::from_str(yaml).context("failed to parse YAML")?;
        let name = value
            .get("metadata")
            .and_then(|metadata| metadata.get("name"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .context("YAML metadata.name is required")?;
        let namespace = value
            .get("metadata")
            .and_then(|metadata| metadata.get("namespace"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| default_namespace.map(ToOwned::to_owned))
            .filter(|namespace| !namespace.is_empty() && namespace != "all" && namespace != "-");

        if resource.is_namespaced() {
            let namespace = namespace
                .clone()
                .context("namespace is required for this resource")?;
            ensure_yaml_namespace(&mut value, &namespace);
        }

        let yaml = serde_yaml::to_string(&value).context("failed to serialize YAML")?;
        self.apply_object_yaml(resource, namespace.as_deref(), &name, &yaml)
            .await
    }

    pub async fn delete_object(
        &self,
        resource: &ResourceKind,
        namespace: Option<&str>,
        name: &str,
    ) -> Result<()> {
        let api_resource = api_resource(resource);
        let objects: Api<DynamicObject> = match (resource.is_namespaced(), namespace) {
            (true, Some(namespace)) if !namespace.is_empty() && namespace != "-" => {
                Api::namespaced_with(self.client.clone(), namespace, &api_resource)
            }
            _ => Api::all_with(self.client.clone(), &api_resource),
        };
        objects
            .delete(name, &DeleteParams::background())
            .await
            .with_context(|| {
                format!(
                    "Could not delete {} {name} {} using context {}.",
                    resource.kind,
                    resource_scope(resource, namespace),
                    self.context
                )
            })?;

        Ok(())
    }

    pub async fn scale_deployment(&self, namespace: &str, name: &str, replicas: i32) -> Result<()> {
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), namespace);
        let params = PatchParams::apply("aetheris").force();
        let patch = serde_json::json!({
            "apiVersion": "autoscaling/v1",
            "kind": "Scale",
            "metadata": { "name": name, "namespace": namespace },
            "spec": { "replicas": replicas }
        });
        deployments
            .patch_scale(name, &params, &Patch::Apply(&patch))
            .await
            .with_context(|| {
                format!(
                    "Could not scale Deployment {name} in namespace {namespace} using context {}.",
                    self.context
                )
            })?;

        Ok(())
    }

    pub async fn set_node_unschedulable(&self, name: &str, unschedulable: bool) -> Result<()> {
        let nodes: Api<Node> = Api::all(self.client.clone());
        let params = PatchParams::default();
        let patch = serde_json::json!({
            "spec": { "unschedulable": unschedulable }
        });
        nodes
            .patch(name, &params, &Patch::Merge(&patch))
            .await
            .with_context(|| {
                format!(
                    "Could not update scheduling state for Node {name} using context {}.",
                    self.context
                )
            })?;

        Ok(())
    }

    pub async fn drain_node(&self, name: &str) -> Result<usize> {
        let pods: Api<Pod> = Api::all(self.client.clone());
        let params = ListParams::default().fields(&format!("spec.nodeName={name}"));
        let items = pods
            .list(&params)
            .await
            .with_context(|| {
                format!(
                    "Could not list Pods scheduled on Node {name} using context {}.",
                    self.context
                )
            })?
            .items;
        let unmanaged = items
            .iter()
            .filter(|pod| {
                !is_terminal_pod(pod)
                    && !is_daemonset_pod(pod)
                    && !is_mirror_pod(pod)
                    && is_unmanaged_pod(pod)
            })
            .map(|pod| pod.name_any())
            .collect::<Vec<_>>();
        if !unmanaged.is_empty() {
            bail!(
                "drain blocked because these Pods do not have a controller: {}",
                unmanaged.join(", ")
            );
        }

        let mut deleted = 0;
        for pod in items {
            if is_terminal_pod(&pod) || is_daemonset_pod(&pod) || is_mirror_pod(&pod) {
                continue;
            }
            let Some(namespace) = pod.namespace() else {
                continue;
            };
            let namespaced_pods: Api<Pod> = Api::namespaced(self.client.clone(), &namespace);
            namespaced_pods
                .delete(&pod.name_any(), &DeleteParams::default())
                .await
                .with_context(|| {
                    format!(
                        "Could not evict/delete Pod {} in namespace {namespace} while draining Node {name}.",
                        pod.name_any()
                    )
                })?;
            deleted += 1;
        }

        Ok(deleted)
    }

    async fn object_events(
        &self,
        resource: &ResourceKind,
        namespace: &str,
        name: &str,
    ) -> Result<Vec<ObjectEvent>> {
        let events: Api<Event> = if namespace.is_empty() || namespace == "-" {
            Api::all(self.client.clone())
        } else {
            Api::namespaced(self.client.clone(), namespace)
        };
        let field_selector = format!(
            "involvedObject.name={name},involvedObject.kind={}",
            resource.kind
        );
        let params = ListParams::default().fields(&field_selector);
        let mut items = events
            .list(&params)
            .await
            .with_context(|| {
                format!(
                    "Could not list Events for {} {name} {}.",
                    resource.kind,
                    resource_scope(resource, Some(namespace))
                )
            })?
            .items;

        items.sort_by(|left, right| {
            event_timestamp(right)
                .cmp(&event_timestamp(left))
                .then_with(|| left.name_any().cmp(&right.name_any()))
        });

        Ok(items.into_iter().map(object_event).collect())
    }

    async fn deployment_pods(
        &self,
        resource: &ResourceKind,
        namespace: &str,
        object: &DynamicObject,
    ) -> Result<Vec<ObjectSummary>> {
        if resource.kind != "Deployment" || resource.group != "apps" || namespace == "-" {
            return Ok(Vec::new());
        }
        let Some(selector) = deployment_label_selector(object) else {
            return Ok(Vec::new());
        };

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        let pod_resource = ResourceKind {
            group: String::new(),
            version: String::from("v1"),
            api_version: String::from("v1"),
            kind: String::from("Pod"),
            plural: String::from("pods"),
            scope: ResourceScope::Namespaced,
        };
        let metrics = self
            .resource_metrics(&pod_resource, Some(namespace))
            .await
            .unwrap_or_default();
        let mut summaries = pods
            .list(&ListParams::default().labels(&selector))
            .await
            .with_context(|| {
                format!(
                    "Could not list Pods owned by Deployment {} in namespace {namespace}.",
                    object.name_any()
                )
            })?
            .items
            .into_iter()
            .map(|pod| {
                let object = serde_json::to_value(&pod)
                    .ok()
                    .and_then(|value| serde_json::from_value::<DynamicObject>(value).ok());
                object
                    .map(|object| object_summary(object, &pod_resource, &metrics))
                    .unwrap_or_else(|| ObjectSummary {
                        name: pod.name_any(),
                        namespace: namespace.to_owned(),
                        status: String::from("-"),
                        status_ratio: None,
                        api_version: String::from("v1"),
                        age: String::from("-"),
                        metrics: None,
                    })
            })
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(summaries)
    }

    async fn resource_metrics(
        &self,
        resource: &ResourceKind,
        namespace: Option<&str>,
    ) -> Result<BTreeMap<(String, String), ResourceUsage>> {
        let metrics_resource = match (resource.group.as_str(), resource.kind.as_str()) {
            ("", "Pod") => metrics_api_resource("PodMetrics", "pods"),
            ("", "Node") => metrics_api_resource("NodeMetrics", "nodes"),
            _ => return Ok(BTreeMap::new()),
        };
        let metrics: Api<DynamicObject> = match (resource.kind.as_str(), namespace) {
            ("Pod", Some(namespace)) if !namespace.is_empty() && namespace != "all" => {
                Api::namespaced_with(self.client.clone(), namespace, &metrics_resource)
            }
            _ => Api::all_with(self.client.clone(), &metrics_resource),
        };

        let items = metrics.list(&ListParams::default()).await?.items;
        Ok(items
            .into_iter()
            .filter_map(|object| {
                let name = object.metadata.name.clone()?;
                let namespace = object.namespace().unwrap_or_else(|| String::from("-"));
                let usage = usage_from_value(object.data.get("usage")?)?;
                Some(((namespace, name), usage))
            })
            .collect())
    }

    async fn pod_container_metrics(
        &self,
        namespace: &str,
        pod: &str,
    ) -> Result<Vec<ContainerUsage>> {
        if namespace == "-" {
            return Ok(Vec::new());
        }
        let metrics_resource = metrics_api_resource("PodMetrics", "pods");
        let metrics: Api<DynamicObject> =
            Api::namespaced_with(self.client.clone(), namespace, &metrics_resource);
        let object = metrics.get(pod).await?;
        let mut containers = object
            .data
            .get("containers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|container| {
                let name = container.get("name")?.as_str()?.to_owned();
                let usage = usage_from_value(container.get("usage")?)?;
                Some(ContainerUsage {
                    name,
                    cpu: usage.cpu,
                    memory: usage.memory,
                })
            })
            .collect::<Vec<_>>();
        containers.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(containers)
    }

    /// A best-effort snapshot for the Clusters list page. Only the initial
    /// connectivity check (the version endpoint) is fatal; node listing,
    /// pod counting and metrics-server queries each degrade to `None`
    /// independently so a cluster without metrics-server (or with a slow
    /// node list) still reports as reachable.
    /// Only the version endpoint is queried: it's a discovery call granted
    /// to `system:authenticated` in every stock RBAC setup, unlike listing
    /// Nodes or cluster-wide Pods, which regularly aren't (the same reason
    /// listing all Namespaces can fail for a scoped kubeconfig context even
    /// though Rancher's own UI can see them).
    pub async fn cluster_summary(&self) -> Result<ClusterSummary> {
        let version_info = self
            .client
            .apiserver_version()
            .await
            .with_context(|| format!("Could not reach context {}.", self.context))?;
        let provider = detect_provider(&version_info.git_version);

        Ok(ClusterSummary {
            version: Some(version_info.git_version),
            provider,
        })
    }
}

fn api_resource(resource: &ResourceKind) -> ApiResource {
    ApiResource {
        group: resource.group.clone(),
        version: resource.version.clone(),
        api_version: resource.api_version.clone(),
        kind: resource.kind.clone(),
        plural: resource.plural.clone(),
    }
}

fn metrics_api_resource(kind: &str, plural: &str) -> ApiResource {
    ApiResource {
        group: String::from("metrics.k8s.io"),
        version: String::from("v1beta1"),
        api_version: String::from("metrics.k8s.io/v1beta1"),
        kind: kind.to_owned(),
        plural: plural.to_owned(),
    }
}

/// Best-effort distribution guess from the server version string alone
/// (e.g. `-eks-`, `+k3s`). Not authoritative — Kubernetes has no generic
/// "who built this cluster" API — but unlike node-label based detection,
/// this needs no extra RBAC permissions beyond the version endpoint.
fn detect_provider(version: &str) -> Option<String> {
    let lower = version.to_ascii_lowercase();
    if lower.contains("-eks-") {
        return Some(String::from("EKS"));
    }
    if lower.contains("-gke.") || lower.contains("-gke-") {
        return Some(String::from("GKE"));
    }
    if lower.contains("+k3s") {
        return Some(String::from("k3s"));
    }
    if lower.contains("+rke2") {
        return Some(String::from("RKE2"));
    }
    None
}

fn namespace_scope(namespace: Option<&str>) -> String {
    match namespace {
        Some(namespace) if !namespace.is_empty() && namespace != "all" && namespace != "-" => {
            format!("in namespace {namespace}")
        }
        _ => String::from("across all namespaces"),
    }
}

fn resource_scope(resource: &ResourceKind, namespace: Option<&str>) -> String {
    if resource.is_namespaced() {
        namespace_scope(namespace)
    } else {
        String::from("at cluster scope")
    }
}

fn resource_group_order(resource: &ResourceKind) -> (u8, &str) {
    let rank = match resource.group.as_str() {
        "" | "apps" | "batch" => 0,
        "networking.k8s.io" | "discovery.k8s.io" => 1,
        "storage.k8s.io" => 2,
        "rbac.authorization.k8s.io" => 3,
        "apiextensions.k8s.io" => 4,
        _ => 5,
    };

    (rank, resource.group.as_str())
}

fn pod_summary(pod: Pod) -> PodSummary {
    let namespace = pod.namespace().unwrap_or_else(|| String::from("<cluster>"));
    let status = pod.status.as_ref();
    let phase = status
        .and_then(|status| status.phase.clone())
        .unwrap_or_else(|| String::from("Unknown"));
    let node = pod
        .spec
        .as_ref()
        .and_then(|spec| spec.node_name.clone())
        .unwrap_or_else(|| String::from("-"));
    let age = pod
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|timestamp| age_label(timestamp.0))
        .unwrap_or_else(|| String::from("-"));

    PodSummary {
        name: pod.name_any(),
        namespace,
        phase,
        node,
        age,
    }
}

fn object_summary(
    object: DynamicObject,
    resource: &ResourceKind,
    metrics: &BTreeMap<(String, String), ResourceUsage>,
) -> ObjectSummary {
    let namespace = object.namespace().unwrap_or_else(|| String::from("-"));
    let (status, status_ratio) = status_label(&object, resource);
    let age = object
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|timestamp| age_label(timestamp.0))
        .unwrap_or_else(|| String::from("-"));

    ObjectSummary {
        name: object.name_any(),
        metrics: metrics
            .get(&(namespace.clone(), object.name_any()))
            .cloned(),
        namespace,
        status,
        status_ratio,
        api_version: resource.api_version.clone(),
        age,
    }
}

fn usage_from_value(value: &Value) -> Option<ResourceUsage> {
    Some(ResourceUsage {
        cpu: value
            .get("cpu")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or("-")
            .to_owned(),
        memory: value
            .get("memory")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or("-")
            .to_owned(),
    })
}

fn object_containers(object: &DynamicObject, resource: &ResourceKind) -> Vec<String> {
    if resource.kind != "Pod" || !resource.group.is_empty() {
        return Vec::new();
    }

    let mut containers = Vec::new();
    let Some(spec) = object.data.get("spec") else {
        return containers;
    };

    for field in ["containers", "initContainers", "ephemeralContainers"] {
        if let Some(items) = spec.get(field).and_then(serde_json::Value::as_array) {
            containers.extend(items.iter().filter_map(|container| {
                container
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
            }));
        }
    }

    containers
}

fn object_replicas(object: &DynamicObject, resource: &ResourceKind) -> Option<i32> {
    (resource.kind == "Deployment" && resource.group == "apps")
        .then(|| {
            object
                .data
                .get("spec")
                .and_then(|spec| spec.get("replicas"))
                .and_then(Value::as_i64)
                .and_then(|replicas| i32::try_from(replicas).ok())
        })
        .flatten()
}

fn object_node_unschedulable(object: &DynamicObject, resource: &ResourceKind) -> Option<bool> {
    (resource.kind == "Node" && resource.group.is_empty()).then(|| {
        object
            .data
            .get("spec")
            .and_then(|spec| spec.get("unschedulable"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn object_conditions(object: &DynamicObject) -> Vec<ObjectCondition> {
    object
        .data
        .get("status")
        .and_then(|status| status.get("conditions"))
        .and_then(Value::as_array)
        .map(|conditions| {
            conditions
                .iter()
                .map(|condition| ObjectCondition {
                    type_: condition_string(condition, "type"),
                    status: condition_string(condition, "status"),
                    reason: condition_string(condition, "reason"),
                    message: condition_string(condition, "message"),
                    last_transition: condition_string(condition, "lastTransitionTime"),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn condition_string(condition: &Value, field: &str) -> String {
    condition
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_owned()
}

fn is_terminal_pod(pod: &Pod) -> bool {
    pod.status
        .as_ref()
        .and_then(|status| status.phase.as_deref())
        .is_some_and(|phase| matches!(phase, "Succeeded" | "Failed"))
}

fn is_daemonset_pod(pod: &Pod) -> bool {
    pod.metadata
        .owner_references
        .as_deref()
        .unwrap_or_default()
        .iter()
        .any(|owner| owner.kind == "DaemonSet")
}

fn is_mirror_pod(pod: &Pod) -> bool {
    pod.metadata
        .annotations
        .as_ref()
        .is_some_and(|annotations| annotations.contains_key("kubernetes.io/config.mirror"))
}

fn is_unmanaged_pod(pod: &Pod) -> bool {
    pod.metadata
        .owner_references
        .as_deref()
        .unwrap_or_default()
        .is_empty()
}

fn deployment_label_selector(object: &DynamicObject) -> Option<String> {
    let labels = object
        .data
        .get("spec")?
        .get("selector")?
        .get("matchLabels")?
        .as_object()?;
    let mut parts = labels
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|value| format!("{key}={value}")))
        .collect::<Vec<_>>();
    parts.sort();
    (!parts.is_empty()).then(|| parts.join(","))
}

fn ensure_yaml_namespace(value: &mut Value, namespace: &str) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    let metadata = object
        .entry("metadata")
        .or_insert_with(|| Value::Object(Default::default()));
    if let Some(metadata) = metadata.as_object_mut() {
        metadata.insert(
            String::from("namespace"),
            Value::String(namespace.to_owned()),
        );
    }
}

fn sanitize_apply_value(value: &mut Value) {
    if let Some(object) = value.as_object_mut() {
        object.remove("status");
        if let Some(metadata) = object.get_mut("metadata").and_then(Value::as_object_mut) {
            for field in [
                "creationTimestamp",
                "deletionGracePeriodSeconds",
                "deletionTimestamp",
                "generation",
                "managedFields",
                "resourceVersion",
                "selfLink",
                "uid",
            ] {
                metadata.remove(field);
            }
        }
    }
}

fn object_event(event: Event) -> ObjectEvent {
    let last_seen = event_timestamp(&event)
        .map(age_label)
        .unwrap_or_else(|| String::from("-"));
    let fallback_name = event.name_any();

    ObjectEvent {
        type_: event.type_.unwrap_or_else(|| String::from("-")),
        reason: event.reason.unwrap_or(fallback_name),
        message: event.message.unwrap_or_else(|| String::from("-")),
        count: event
            .count
            .map(|count| count.to_string())
            .unwrap_or_else(|| String::from("-")),
        last_seen,
    }
}

fn event_timestamp(event: &Event) -> Option<Timestamp> {
    event
        .last_timestamp
        .as_ref()
        .map(|timestamp| timestamp.0)
        .or_else(|| event.event_time.as_ref().map(|timestamp| timestamp.0))
        .or_else(|| event.first_timestamp.as_ref().map(|timestamp| timestamp.0))
        .or_else(|| {
            event
                .metadata
                .creation_timestamp
                .as_ref()
                .map(|timestamp| timestamp.0)
        })
}

fn status_label(object: &DynamicObject, resource: &ResourceKind) -> (String, Option<(i64, i64)>) {
    match (resource.group.as_str(), resource.kind.as_str()) {
        ("apps", "Deployment") => return deployment_status_label(object),
        ("apps", "StatefulSet") => return ready_replicas_status_label(object),
        ("apps", "ReplicaSet") => return ready_replicas_status_label(object),
        ("apps", "DaemonSet") => return daemonset_status_label(object),
        ("batch", "Job") => return job_status_label(object),
        ("batch", "CronJob") => return (cronjob_status_label(object), None),
        ("", "Pod") => return (pod_status_label(object), None),
        ("", "Node") => return (node_status_label(object), None),
        ("", "Service") => {
            return (
                spec_string(object, "type").unwrap_or_else(|| String::from("ClusterIP")),
                None,
            )
        }
        ("networking.k8s.io", "Ingress") => return (ingress_status_label(object), None),
        ("", "ConfigMap") => return (data_entries_status_label(object), None),
        ("", "Secret") => return (data_entries_status_label(object), None),
        _ => {}
    }

    let Some(status) = object.data.get("status") else {
        return (String::from("-"), None);
    };

    if let Some(phase) = status.get("phase").and_then(|value| value.as_str()) {
        return (phase.to_owned(), None);
    }

    if let Some(conditions) = status.get("conditions").and_then(|value| value.as_array()) {
        if let Some(ready) = conditions.iter().find(|condition| {
            condition.get("type").and_then(|value| value.as_str()) == Some("Ready")
        }) {
            let label = ready
                .get("status")
                .and_then(|value| value.as_str())
                .map(|status| format!("Ready={status}"))
                .unwrap_or_else(|| String::from("Ready"));
            return (label, None);
        }
    }

    let ready_replicas = status.get("readyReplicas").and_then(|value| value.as_i64());
    let replicas = status.get("replicas").and_then(|value| value.as_i64());
    if ready_replicas.is_some() || replicas.is_some() {
        let ready = ready_replicas.unwrap_or(0);
        let total = replicas.unwrap_or(0);
        return (format!("{ready}/{total}"), Some((ready, total)));
    }

    (String::from("-"), None)
}

fn deployment_status_label(object: &DynamicObject) -> (String, Option<(i64, i64)>) {
    let desired = spec_i64(object, "replicas").unwrap_or(1);
    let ready = status_i64(object, "readyReplicas").unwrap_or(0);
    let updated = status_i64(object, "updatedReplicas").unwrap_or(0);
    let available = status_i64(object, "availableReplicas").unwrap_or(0);

    if desired == 0 {
        return (String::from("Scaled 0"), Some((0, 0)));
    }
    let ratio = Some((ready, desired));
    if ready >= desired && available >= desired {
        return (format!("Ready {ready}/{desired}"), ratio);
    }
    if updated < desired {
        return (format!("Updating {ready}/{desired}"), ratio);
    }
    (format!("Unavailable {ready}/{desired}"), ratio)
}

fn ready_replicas_status_label(object: &DynamicObject) -> (String, Option<(i64, i64)>) {
    let desired = status_i64(object, "replicas")
        .or_else(|| spec_i64(object, "replicas"))
        .unwrap_or(0);
    let ready = status_i64(object, "readyReplicas").unwrap_or(0);

    if desired == 0 {
        (String::from("Scaled 0"), Some((0, 0)))
    } else if ready >= desired {
        (format!("Ready {ready}/{desired}"), Some((ready, desired)))
    } else {
        (
            format!("Progressing {ready}/{desired}"),
            Some((ready, desired)),
        )
    }
}

fn daemonset_status_label(object: &DynamicObject) -> (String, Option<(i64, i64)>) {
    let desired = status_i64(object, "desiredNumberScheduled").unwrap_or(0);
    let ready = status_i64(object, "numberReady").unwrap_or(0);
    let unavailable = status_i64(object, "numberUnavailable").unwrap_or(0);

    if desired == 0 {
        (String::from("No nodes"), Some((0, 0)))
    } else if ready >= desired && unavailable == 0 {
        (format!("Ready {ready}/{desired}"), Some((ready, desired)))
    } else {
        (
            format!("Unavailable {ready}/{desired}"),
            Some((ready, desired)),
        )
    }
}

fn job_status_label(object: &DynamicObject) -> (String, Option<(i64, i64)>) {
    let completions = spec_i64(object, "completions").unwrap_or(1);
    let succeeded = status_i64(object, "succeeded").unwrap_or(0);
    let failed = status_i64(object, "failed").unwrap_or(0);
    let active = status_i64(object, "active").unwrap_or(0);
    let ratio = Some((succeeded, completions));

    if succeeded >= completions {
        (format!("Complete {succeeded}/{completions}"), ratio)
    } else if failed > 0 {
        (format!("Failed {failed}"), ratio)
    } else if active > 0 {
        (format!("Running {succeeded}/{completions}"), ratio)
    } else {
        (format!("Pending {succeeded}/{completions}"), ratio)
    }
}

fn cronjob_status_label(object: &DynamicObject) -> String {
    if spec_bool(object, "suspend").unwrap_or(false) {
        return String::from("Suspended");
    }
    let active = object
        .data
        .get("status")
        .and_then(|status| status.get("active"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if active > 0 {
        format!("Active {active}")
    } else {
        String::from("Scheduled")
    }
}

fn pod_status_label(object: &DynamicObject) -> String {
    let phase = object
        .data
        .get("status")
        .and_then(|status| status.get("phase"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown");
    if phase == "Running" {
        if condition_status(object, "Ready").as_deref() == Some("True") {
            String::from("Ready")
        } else {
            String::from("Running")
        }
    } else {
        phase.to_owned()
    }
}

fn node_status_label(object: &DynamicObject) -> String {
    let ready = match condition_status(object, "Ready").as_deref() {
        Some("True") => "Ready",
        Some("False") => "NotReady",
        _ => "Unknown",
    };
    if spec_bool(object, "unschedulable").unwrap_or(false) {
        format!("{ready} SchedulingDisabled")
    } else {
        ready.to_owned()
    }
}

fn ingress_status_label(object: &DynamicObject) -> String {
    let ingress_count = object
        .data
        .get("status")
        .and_then(|status| status.get("loadBalancer"))
        .and_then(|load_balancer| load_balancer.get("ingress"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if ingress_count > 0 {
        format!("Ready {ingress_count}")
    } else {
        String::from("Pending")
    }
}

fn data_entries_status_label(object: &DynamicObject) -> String {
    let entries = object
        .data
        .get("data")
        .and_then(Value::as_object)
        .map(serde_json::Map::len)
        .unwrap_or(0);
    format!("{entries} keys")
}

fn condition_status(object: &DynamicObject, condition_type: &str) -> Option<String> {
    object
        .data
        .get("status")
        .and_then(|status| status.get("conditions"))
        .and_then(Value::as_array)
        .and_then(|conditions| {
            conditions.iter().find(|condition| {
                condition.get("type").and_then(Value::as_str) == Some(condition_type)
            })
        })
        .and_then(|condition| condition.get("status"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn status_i64(object: &DynamicObject, key: &str) -> Option<i64> {
    object
        .data
        .get("status")
        .and_then(|status| status.get(key))
        .and_then(Value::as_i64)
}

fn spec_i64(object: &DynamicObject, key: &str) -> Option<i64> {
    object
        .data
        .get("spec")
        .and_then(|spec| spec.get(key))
        .and_then(Value::as_i64)
}

fn spec_bool(object: &DynamicObject, key: &str) -> Option<bool> {
    object
        .data
        .get("spec")
        .and_then(|spec| spec.get(key))
        .and_then(Value::as_bool)
}

fn spec_string(object: &DynamicObject, key: &str) -> Option<String> {
    object
        .data
        .get("spec")
        .and_then(|spec| spec.get(key))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn age_label(created_at: Timestamp) -> String {
    let age = Timestamp::now().duration_since(created_at);
    let seconds = age.as_secs().max(0);

    if seconds >= 86_400 {
        format!("{}d", seconds / 86_400)
    } else if seconds >= 3_600 {
        format!("{}h", seconds / 3_600)
    } else if seconds >= 60 {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

#[cfg(test)]
mod tests {
    use k8s_openapi::jiff::{SignedDuration, Timestamp};

    use std::sync::Mutex;

    use secrecy::ExposeSecret;

    use kube::api::DynamicObject;

    use super::{
        age_label, deployment_status_label, detect_provider, job_status_label, server_host,
        AddClusterRequest, KubeManager, Kubeconfig,
    };

    // `add_token_cluster` reads the process-wide `KUBECONFIG` env var, so tests
    // that set it must not run concurrently with each other.
    static KUBECONFIG_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn age_label_uses_largest_relevant_unit() {
        let now = Timestamp::now();

        assert!(age_label(now - SignedDuration::from_hours(48)).ends_with('d'));
        assert!(age_label(now - SignedDuration::from_hours(3)).ends_with('h'));
        assert!(age_label(now - SignedDuration::from_mins(12)).ends_with('m'));
    }

    #[test]
    fn deployment_status_label_reports_ready_ratio() {
        let object: DynamicObject = serde_json::from_value(serde_json::json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": { "name": "web" },
            "spec": { "replicas": 20 },
            "status": {
                "readyReplicas": 20,
                "updatedReplicas": 20,
                "availableReplicas": 20
            }
        }))
        .unwrap();

        let (label, ratio) = deployment_status_label(&object);

        assert_eq!(label, "Ready 20/20");
        assert_eq!(ratio, Some((20, 20)));
    }

    #[test]
    fn deployment_status_label_reports_ratio_while_updating() {
        let object: DynamicObject = serde_json::from_value(serde_json::json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": { "name": "web" },
            "spec": { "replicas": 10 },
            "status": {
                "readyReplicas": 3,
                "updatedReplicas": 3,
                "availableReplicas": 3
            }
        }))
        .unwrap();

        let (label, ratio) = deployment_status_label(&object);

        assert_eq!(label, "Updating 3/10");
        assert_eq!(ratio, Some((3, 10)));
    }

    #[test]
    fn job_status_label_reports_completions_ratio() {
        let object: DynamicObject = serde_json::from_value(serde_json::json!({
            "apiVersion": "batch/v1",
            "kind": "Job",
            "metadata": { "name": "migrate" },
            "spec": { "completions": 3 },
            "status": { "succeeded": 1, "active": 1 }
        }))
        .unwrap();

        let (label, ratio) = job_status_label(&object);

        assert_eq!(label, "Running 1/3");
        assert_eq!(ratio, Some((1, 3)));
    }

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
        let path = std::env::temp_dir().join(format!(
            "aetheris-kube-test-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        // SAFETY: test-only, runs in an isolated process with no concurrent KUBECONFIG readers.
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
        use kube::config::{AuthInfo, Cluster, Context as KubeContext, NamedAuthInfo, NamedCluster, NamedContext};
        use secrecy::SecretString;

        let _guard = KUBECONFIG_ENV_LOCK.lock().unwrap();
        let path = std::env::temp_dir().join(format!(
            "aetheris-kube-test-imported-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        // Simulate a kubeconfig brought in via "Import" where the auth info name
        // does not follow the app's own "{context}-user" naming convention.
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

        // SAFETY: test-only, runs in an isolated process with no concurrent KUBECONFIG readers.
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
        use kube::config::{AuthInfo, Cluster, Context as KubeContext, NamedAuthInfo, NamedCluster, NamedContext};
        use secrecy::SecretString;

        let _guard = KUBECONFIG_ENV_LOCK.lock().unwrap();
        let path = std::env::temp_dir().join(format!(
            "aetheris-kube-test-cert-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        // Simulate a kubeconfig brought in via "Import" that authenticates
        // with a client certificate instead of a bearer token.
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

        // SAFETY: test-only, runs in an isolated process with no concurrent KUBECONFIG readers.
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
        KubeManager::add_token_cluster(edit).expect(
            "editing a cert-based cluster without entering a token should not require one",
        );

        let kubeconfig = Kubeconfig::read_from(&path).expect("kubeconfig should be readable");
        let auth_info = kubeconfig
            .auth_infos
            .iter()
            .find(|auth| auth.name == "cert-user")
            .and_then(|auth| auth.auth_info.as_ref())
            .expect("cert-user auth info should still exist");

        assert_eq!(auth_info.client_certificate_data.as_deref(), Some("cert-data"));
        assert_eq!(
            auth_info.client_key_data.as_ref().map(|key| key.expose_secret()),
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
        let path = std::env::temp_dir().join(format!(
            "aetheris-kube-test-rename-{}-{}.yaml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        // SAFETY: test-only, runs in an isolated process with no concurrent KUBECONFIG readers.
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
            kubeconfig.contexts.iter().all(|context| context.name != "old-name"),
            "the old context name should not linger as a duplicate"
        );
        assert!(
            kubeconfig.clusters.iter().all(|cluster| cluster.name != "old-name"),
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

    #[test]
    fn detect_provider_reads_eks_version_suffix() {
        assert_eq!(
            detect_provider("v1.34.8-eks-0247562"),
            Some(String::from("EKS"))
        );
    }

    #[test]
    fn detect_provider_reads_k3s_version_suffix() {
        assert_eq!(
            detect_provider("v1.27.3+k3s1"),
            Some(String::from("k3s"))
        );
    }

    #[test]
    fn detect_provider_reports_none_when_unrecognized() {
        assert_eq!(detect_provider("v1.28.0"), None);
    }
}
