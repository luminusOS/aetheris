use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::PathBuf,
};

use aetheris_kube::{
    AddClusterRequest, ClusterSummary, ContainerUsage, ContextInfo, KubeManager, ObjectCondition,
    ObjectDetail, ObjectEvent, ObjectSummary, ObjectWatchEvent, PodExecEvent, PodExecRequest,
    PodLogRequest, PodPortForwardEvent, PodPortForwardRequest, ResourceKind, ResourceUsage,
};
use futures::FutureExt;
use futures::future::{AbortHandle, Abortable};
use relm4::adw::prelude::*;
use relm4::prelude::*;
use relm4::{adw, gtk};
use serde::{Deserialize, Serialize};
use sourceview5::prelude::*;
#[cfg(not(target_os = "windows"))]
use vte4::prelude::*;

mod ansi;
mod commands;
mod component;
mod dialogs;
mod handler;
pub(crate) mod i18n;
mod layout;
mod methods;
mod object_detail;
mod projects;
mod streams;
mod style;
mod utils;
mod widgets;
mod yaml;

use i18n::{tr, tr_format, trn};
use projects::{
    DetailTarget, ObjectColumn, ObjectTableColumn, PodLogTarget, Project, ProjectStore,
    ResourceSection, StatusFilter,
};
use widgets::{
    is_access_resource, is_cluster_resource, is_configuration_resource, is_network_resource,
    is_storage_resource, is_workload_resource,
};

const DEFAULT_PROJECT_NAME: &str = "Default";
const OBJECT_NAME_WIDTH: i32 = 244;
const OBJECT_NAMESPACE_WIDTH: i32 = 132;
const OBJECT_STATUS_WIDTH: i32 = 64;
const OBJECT_METRIC_WIDTH: i32 = 64;
const OBJECT_API_WIDTH: i32 = 96;
const OBJECT_AGE_WIDTH: i32 = 56;
const OBJECT_COLUMN_MIN_WIDTH: i32 = 48;
const OBJECT_COLUMN_MAX_WIDTH: i32 = 260;
const OBJECT_NAME_MIN_WIDTH: i32 = 160;
const OBJECT_NAME_MAX_WIDTH: i32 = 520;

#[derive(Debug, Clone)]
pub(super) enum ClusterSummaryState {
    Loading,
    Loaded(ClusterSummary),
    Error(String),
}

pub(super) struct DetailPane {
    back_button: gtk::Button,
    stack: gtk::Stack,
    name_label: gtk::Label,
    namespace_label: gtk::Label,
    status_label: gtk::Label,
    kind_label: gtk::Label,
    api_label: gtk::Label,
    age_label: gtk::Label,
    cpu_label: gtk::Label,
    memory_label: gtk::Label,
    container_metrics_list: gtk::ListBox,
    scale_spin: gtk::SpinButton,
    scale_button: gtk::Button,
    cordon_button: gtk::Button,
    drain_button: gtk::Button,
    explain_yaml_button: gtk::Button,
    apply_button: gtk::Button,
    download_yaml_button: gtk::Button,
    delete_button: gtk::Button,
    terminal_button: gtk::Button,
    yaml_buffer: sourceview5::Buffer,
    events_list: gtk::ListBox,
    conditions_list: gtk::ListBox,
    related_pods_store: gtk::gio::ListStore,
    related_pods_sorted: gtk::SortListModel,
    related_pods_stack: gtk::Stack,
    related_pods_message: adw::StatusPage,
    log_container_dropdown: gtk::DropDown,
    log_follow_check: gtk::CheckButton,
    log_timestamps_check: gtk::CheckButton,
    log_start_button: gtk::Button,
    log_stop_button: gtk::Button,
    log_status_label: gtk::Label,
    log_buffer: gtk::TextBuffer,
    log_view: gtk::TextView,
    port_local_spin: gtk::SpinButton,
    port_remote_spin: gtk::SpinButton,
    port_start_button: gtk::Button,
    port_stop_button: gtk::Button,
    port_status_label: gtk::Label,
    port_forward_group: gtk::Box,
    overview_section: gtk::Box,
    expand_logs_button: gtk::Button,
    target: Option<DetailTarget>,
    log_target: Option<PodLogTarget>,
    exec_target: Option<PodLogTarget>,
    port_forward_target: Option<PodLogTarget>,
    node_unschedulable: Option<bool>,
    request_token: u64,
}

pub struct App {
    projects: ProjectStore,
    contexts: Vec<ContextInfo>,
    namespaces: Vec<String>,
    resources: Vec<ResourceKind>,
    objects: Vec<ObjectSummary>,
    /// True while a coalesced object-list refresh is pending. Watch events
    /// can arrive thousands at a time while a namespace spins up; rebuilding
    /// the list per event would freeze the UI, so events only mark the list
    /// dirty and a single deferred refresh repaints it.
    object_list_refresh_scheduled: bool,
    /// True while a coalesced projects.json save is pending. Dragging a
    /// column border emits a width change per pixel; saving on each would
    /// hammer the disk, so width changes only schedule one deferred save.
    project_save_scheduled: bool,
    selected_context: Option<String>,
    selected_namespace: String,
    selected_resource_section: ResourceSection,
    selected_resource: Option<usize>,
    search_query: String,
    selected_status_filters: BTreeSet<StatusFilter>,
    loading: bool,
    status: String,
    toaster: adw::ToastOverlay,
    root_stack: gtk::Stack,
    split_view: adw::OverlaySplitView,
    project_list: gtk::ListBox,
    project_title_label: gtk::Label,
    add_project_button: gtk::Button,
    projects_content_stack: gtk::Stack,
    cluster_back_button: gtk::Button,
    cluster_menu_button: gtk::MenuButton,
    cluster_refresh_button: gtk::Button,
    context_selector_label: gtk::Label,
    cluster_list: gtk::ListBox,
    add_cluster_button: gtk::Button,
    import_cluster_button: gtk::Button,
    clusters_content_stack: gtk::Stack,
    cluster_summaries: std::collections::HashMap<String, ClusterSummaryState>,
    namespace_menu_button: gtk::MenuButton,
    namespace_selector_label: gtk::Label,
    namespace_list: gtk::ListBox,
    search_entry: gtk::SearchEntry,
    status_filter_list: gtk::FlowBox,
    column_filter_list: gtk::FlowBox,
    create_yaml_button: gtk::Button,
    refresh_button: gtk::Button,
    content_title_label: gtk::Label,
    content_header_stack: gtk::Stack,
    content_stack: gtk::Stack,
    status_label: gtk::Label,
    spinner: gtk::Spinner,
    resource_list: gtk::ListBox,
    /// Backing model for `object_view`. The `ColumnView` only realizes
    /// widgets for on-screen rows, so this can hold tens of thousands of
    /// objects without the widget tree growing with it.
    object_store: gtk::gio::ListStore,
    /// The store as the view displays it (header-click sort applied);
    /// activation positions index into this model.
    object_sorted: gtk::SortListModel,
    object_columns: Vec<(ObjectTableColumn, gtk::ColumnViewColumn)>,
    object_list_stack: gtk::Stack,
    detail: DetailPane,
    object_watch_token: u64,
    object_watch_abort_handle: Option<AbortHandle>,
    log_streaming: bool,
    log_stream_token: u64,
    log_abort_handle: Option<AbortHandle>,
    exec_token: u64,
    terminal_sessions: HashMap<u64, TerminalSession>,
    port_forwarding: bool,
    port_forward_token: u64,
    port_forward_abort_handle: Option<AbortHandle>,
    custom_namespace_dialog: adw::Dialog,
    custom_namespace_entry: adw::EntryRow,
    custom_namespace_button: gtk::Button,
    rename_namespace_dialog: adw::Dialog,
    rename_namespace_entry: adw::EntryRow,
    rename_namespace_button: gtk::Button,
    project_dialog: adw::Dialog,
    project_dialog_description: gtk::Label,
    project_name_entry: adw::EntryRow,
    project_create_button: gtk::Button,
    editing_project_name: Option<String>,
    create_yaml_dialog: adw::Dialog,
    create_yaml_buffer: sourceview5::Buffer,
    create_yaml_apply_button: gtk::Button,
    cluster_dialog: adw::Dialog,
    cluster_dialog_stack: gtk::Stack,
    cluster_token_title_label: gtk::Label,
    cluster_token_back_button: gtk::Button,
    setup_name_entry: adw::EntryRow,
    setup_server_entry: adw::EntryRow,
    setup_token_entry: adw::PasswordEntryRow,
    setup_ca_entry: adw::EntryRow,
    setup_insecure_check: adw::SwitchRow,
    setup_button: gtk::Button,
    editing_cluster: bool,
    editing_context_name: Option<String>,
    renaming_namespace: Option<String>,
}

#[cfg(not(target_os = "windows"))]
struct TerminalSession {
    window: adw::Window,
    container_dropdown: gtk::DropDown,
    view: vte4::Terminal,
    target: PodLogTarget,
    abort_handle: Option<AbortHandle>,
    input_tx: Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>>,
}

#[cfg(target_os = "windows")]
struct TerminalSession;

#[derive(Debug)]
pub enum AppMsg {
    Loaded(Result<LoadedState, String>),
    ClusterLoaded(Result<ClusterState, String>),
    ProjectChanged(u32),
    ShowAddProjectDialog,
    ShowRenameProjectDialog,
    AddProject,
    DuplicateProject,
    DeleteProject,
    ConfirmDeleteProject,
    ShowClusters,
    RefreshClusters,
    ClusterChanged(u32),
    ClusterSummaryLoaded(String, Result<ClusterSummary, String>),
    NamespaceChanged(u32),
    CustomNamespaceEntered,
    RemoveCustomNamespace(String),
    OpenRenameNamespaceDialog(String),
    RenameNamespaceConfirmed,
    StatusFilterChanged(u32),
    ObjectColumnToggled(u32),
    ObjectColumnResized(ObjectTableColumn, i32),
    EditCurrentCluster,
    ResourceChanged(usize),
    SearchChanged(String),
    ObjectActivated(i32),
    RelatedPodActivated(i32),
    ObjectDetailLoaded(u64, Result<ObjectDetail, String>),
    ShowProjects,
    BackToObjects,
    DetailTabChanged(String),
    ShowCreateYamlDialog,
    CreateYaml,
    ObjectCreated(Result<String, String>),
    ObjectWatchEvent(u64, ObjectWatchEvent),
    ObjectWatchFinished(u64, Result<(), String>),
    ObjectListRefreshTick,
    ProjectSaveTick,
    ScaleDeployment,
    ObjectScaled(u64, Result<ObjectDetail, String>),
    ToggleNodeScheduling,
    NodeSchedulingUpdated(u64, Result<ObjectDetail, String>),
    DrainNode,
    ConfirmDrainNode,
    NodeDrained(u64, Result<(ObjectDetail, usize), String>),
    ExplainYaml,
    ApplyYaml,
    ObjectApplied(u64, Result<ObjectDetail, String>),
    DownloadYaml,
    SaveYamlTo(PathBuf, String),
    DownloadLogs,
    SaveLogsTo(PathBuf, String),
    DeleteObject,
    ConfirmDeleteObject,
    ObjectDeleted(u64, Result<String, String>),
    StartPodLogs,
    StopPodLogs,
    ClearPodLogs,
    ToggleDetailOverview,
    PodLogLine(u64, String),
    PodLogFinished(u64, Result<(), String>),
    ShowPodTerminal,
    RestartPodTerminal(u64),
    StopPodTerminal(u64),
    PodTerminalInput(u64, String),
    PodExecEvent(u64, PodExecEvent),
    PodExecFinished(u64, Result<(), String>),
    StartPodPortForward,
    StopPodPortForward,
    PodPortForwardEvent(u64, PodPortForwardEvent),
    PodPortForwardFinished(u64, Result<(), String>),
    ShowAddClusterDialog,
    ShowTokenForm,
    ShowImportFile,
    Refresh,
    ObjectsLoaded(Result<Vec<ObjectSummary>, String>),
    AddCluster,
    ClusterAdded(Result<(String, String), String>),
    StateLoadedForCluster(String, Result<LoadedState, String>),
    RemoveClusterFromProject,
    ImportKubeconfig(PathBuf),
    KubeconfigImported(Result<(String, Vec<String>), String>),
    StateLoadedForImportedClusters(Vec<String>, Result<LoadedState, String>),
    Toast(String),
}

#[derive(Debug)]
pub struct LoadedState {
    contexts: Vec<ContextInfo>,
    namespaces: Vec<String>,
    projects: ProjectStore,
}

#[derive(Debug)]
pub struct ClusterState {
    namespaces: Vec<String>,
    resources: Vec<ResourceKind>,
    namespace_warning: Option<String>,
}
