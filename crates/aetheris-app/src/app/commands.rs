use super::utils::*;
use super::*;

pub(super) async fn load_state() -> AppMsg {
    let result = async {
        let manager = KubeManager::load()?;
        let contexts = manager.load_contexts();
        let projects = ProjectStore::load(&contexts);
        Ok::<_, anyhow::Error>(LoadedState {
            contexts,
            namespaces: with_all_namespace(manager.namespaces()),
            projects,
        })
    }
    .await
    .map_err(format_error);

    AppMsg::Loaded(result)
}

pub(super) async fn load_cluster(context: String) -> AppMsg {
    let result = async {
        let manager = KubeManager::load()?;
        let session = manager.connect_context(&context).await?;
        let (namespaces, namespace_warning) = match session.list_namespaces().await {
            Ok(namespaces) => (namespaces, None),
            Err(error) => (manager.namespaces(), Some(format_error(error))),
        };
        let resources = session.discover_resources().await?;
        Ok::<_, anyhow::Error>(ClusterState {
            namespaces,
            resources,
            namespace_warning,
        })
    }
    .await
    .map_err(format_error);

    AppMsg::ClusterLoaded(result)
}

pub(super) async fn load_cluster_summary(context: String) -> AppMsg {
    let result = async {
        let manager = KubeManager::load()?;
        let session = manager.connect_context(&context).await?;
        session.cluster_summary().await
    }
    .await
    .map_err(format_error);

    AppMsg::ClusterSummaryLoaded(context, result)
}

pub(super) async fn list_objects(
    context: String,
    resource: ResourceKind,
    namespace: Option<String>,
) -> AppMsg {
    let result = list_objects_snapshot(context, resource, namespace).await;

    AppMsg::ObjectsLoaded(result)
}

pub(super) async fn list_objects_snapshot(
    context: String,
    resource: ResourceKind,
    namespace: Option<String>,
) -> Result<Vec<ObjectSummary>, String> {
    let result = async {
        let manager = KubeManager::load()?;
        let session = manager.connect_context(&context).await?;
        session.list_objects(&resource, namespace.as_deref()).await
    }
    .await
    .map_err(format_error);

    result
}

pub(super) async fn stream_object_watch(
    context: String,
    resource: ResourceKind,
    namespace: Option<String>,
    token: u64,
    out: relm4::Sender<AppMsg>,
) -> anyhow::Result<()> {
    let manager = KubeManager::load()?;
    let session = manager.connect_context(&context).await?;
    session
        .watch_objects(resource, namespace, move |event| {
            let _ = out.send(AppMsg::ObjectWatchEvent(token, event));
        })
        .await
}

pub(super) async fn load_object_detail(
    token: u64,
    context: String,
    resource: ResourceKind,
    namespace: Option<String>,
    name: String,
) -> AppMsg {
    let result = async {
        let manager = KubeManager::load()?;
        let session = manager.connect_context(&context).await?;
        session
            .object_detail(&resource, namespace.as_deref(), &name)
            .await
    }
    .await
    .map_err(format_error);

    AppMsg::ObjectDetailLoaded(token, result)
}

pub(super) async fn stream_pod_logs(
    context: String,
    request: PodLogRequest,
    token: u64,
    out: relm4::Sender<AppMsg>,
) -> anyhow::Result<()> {
    let manager = KubeManager::load()?;
    let session = manager.connect_context(&context).await?;
    session
        .stream_pod_logs(request, move |line| {
            let _ = out.send(AppMsg::PodLogLine(token, line));
        })
        .await
}

pub(super) async fn run_pod_port_forward(
    context: String,
    request: PodPortForwardRequest,
    token: u64,
    out: relm4::Sender<AppMsg>,
) -> anyhow::Result<()> {
    let manager = KubeManager::load()?;
    let session = manager.connect_context(&context).await?;
    session
        .port_forward_pod(request, move |event| {
            let _ = out.send(AppMsg::PodPortForwardEvent(token, event));
        })
        .await
}

pub(super) async fn stream_pod_terminal(
    context: String,
    request: PodExecRequest,
    input_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    token: u64,
    out: relm4::Sender<AppMsg>,
) -> anyhow::Result<()> {
    let manager = KubeManager::load()?;
    let session = manager.connect_context(&context).await?;
    session
        .terminal_pod(request, input_rx, move |event| {
            let _ = out.send(AppMsg::PodExecEvent(token, event));
        })
        .await
}

pub(super) async fn apply_object_yaml(token: u64, target: DetailTarget, yaml: String) -> AppMsg {
    let result = async {
        let manager = KubeManager::load()?;
        let session = manager.connect_context(&target.context).await?;
        session
            .apply_object_yaml(
                &target.resource,
                target.namespace.as_deref(),
                &target.name,
                &yaml,
            )
            .await
    }
    .await
    .map_err(format_error);

    AppMsg::ObjectApplied(token, result)
}

pub(super) async fn create_object_yaml(
    context: String,
    resource: ResourceKind,
    namespace: Option<String>,
    yaml: String,
) -> AppMsg {
    let result = async {
        let manager = KubeManager::load()?;
        let session = manager.connect_context(&context).await?;
        let detail = session
            .create_object_yaml(&resource, namespace.as_deref(), &yaml)
            .await?;
        Ok::<_, anyhow::Error>(detail.name)
    }
    .await
    .map_err(format_error);

    AppMsg::ObjectCreated(result)
}

pub(super) async fn scale_deployment(token: u64, target: DetailTarget, replicas: i32) -> AppMsg {
    let result = async {
        let namespace = target
            .namespace
            .as_deref()
            .filter(|namespace| !namespace.is_empty() && *namespace != "-")
            .ok_or_else(|| anyhow::anyhow!("namespace is required for deployments"))?;
        let manager = KubeManager::load()?;
        let session = manager.connect_context(&target.context).await?;
        session
            .scale_deployment(namespace, &target.name, replicas)
            .await?;
        session
            .object_detail(&target.resource, target.namespace.as_deref(), &target.name)
            .await
    }
    .await
    .map_err(format_error);

    AppMsg::ObjectScaled(token, result)
}

pub(super) async fn set_node_unschedulable(
    token: u64,
    target: DetailTarget,
    unschedulable: bool,
) -> AppMsg {
    let result = async {
        let manager = KubeManager::load()?;
        let session = manager.connect_context(&target.context).await?;
        session
            .set_node_unschedulable(&target.name, unschedulable)
            .await?;
        session
            .object_detail(&target.resource, target.namespace.as_deref(), &target.name)
            .await
    }
    .await
    .map_err(format_error);

    AppMsg::NodeSchedulingUpdated(token, result)
}

pub(super) async fn drain_node(token: u64, target: DetailTarget) -> AppMsg {
    let result = async {
        let manager = KubeManager::load()?;
        let session = manager.connect_context(&target.context).await?;
        let count = session.drain_node(&target.name).await?;
        let detail = session
            .object_detail(&target.resource, target.namespace.as_deref(), &target.name)
            .await?;
        Ok::<_, anyhow::Error>((detail, count))
    }
    .await
    .map_err(format_error);

    AppMsg::NodeDrained(token, result)
}

pub(super) async fn delete_object(token: u64, target: DetailTarget) -> AppMsg {
    let name = target.name.clone();
    let result = async {
        let manager = KubeManager::load()?;
        let session = manager.connect_context(&target.context).await?;
        session
            .delete_object(&target.resource, target.namespace.as_deref(), &target.name)
            .await
    }
    .await
    .map(|()| name)
    .map_err(format_error);

    AppMsg::ObjectDeleted(token, result)
}

pub(super) async fn add_cluster(request: AddClusterRequest) -> AppMsg {
    let context_name = request.context_name.trim().to_owned();
    let result = KubeManager::add_token_cluster(request)
        .map(|path| (path.display().to_string(), context_name))
        .map_err(format_error);

    AppMsg::ClusterAdded(result)
}

pub(super) async fn load_state_for_cluster(context: String) -> AppMsg {
    let result = async {
        let manager = KubeManager::load()?;
        let contexts = manager.load_contexts();
        let projects = ProjectStore::load(&contexts);
        Ok::<_, anyhow::Error>(LoadedState {
            contexts,
            namespaces: with_all_namespace(manager.namespaces()),
            projects,
        })
    }
    .await
    .map_err(format_error);

    AppMsg::StateLoadedForCluster(context, result)
}

pub(super) async fn load_state_for_imported_clusters(context_names: Vec<String>) -> AppMsg {
    let result = async {
        let manager = KubeManager::load()?;
        let contexts = manager.load_contexts();
        let projects = ProjectStore::load(&contexts);
        Ok::<_, anyhow::Error>(LoadedState {
            contexts,
            namespaces: with_all_namespace(manager.namespaces()),
            projects,
        })
    }
    .await
    .map_err(format_error);

    AppMsg::StateLoadedForImportedClusters(context_names, result)
}

pub(super) async fn import_kubeconfig(path: PathBuf) -> AppMsg {
    let result = KubeManager::import_kubeconfig(path)
        .map(|(path, contexts)| (path.display().to_string(), contexts))
        .map_err(format_error);

    AppMsg::KubeconfigImported(result)
}
