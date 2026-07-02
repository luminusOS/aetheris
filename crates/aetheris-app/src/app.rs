use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::PathBuf,
};

use aetheris_kube::{
    AddClusterRequest, ClusterSummary, ContainerUsage, ContextInfo, KubeManager, ObjectCondition,
    ObjectDetail, ObjectEvent, ObjectSummary, ObjectWatchEvent, PodExecEvent, PodExecRequest,
    PodLogRequest, PodPortForwardEvent, PodPortForwardRequest, ResourceKind,
};
use futures::future::{AbortHandle, Abortable};
use futures::FutureExt;
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
mod layout;
mod methods;
mod object_detail;
mod projects;
mod streams;
mod utils;
mod widgets;
mod yaml;

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
const APP_CSS: &str = r#"
.resource-row-selected {
  background-color: alpha(currentColor, 0.08);
}

.resource-row-selected:hover {
  background-color: alpha(currentColor, 0.11);
}

.project-icon {
  border-radius: 999px;
  background-color: alpha(currentColor, 0.08);
}

.status-chip {
  border-radius: 999px;
  padding: 2px 8px;
  font-weight: 700;
  font-size: 0.82em;
}

.status-good {
  background-color: alpha(@success_color, 0.16);
  color: @success_color;
}

.status-warning {
  background-color: alpha(@warning_color, 0.18);
  color: @warning_color;
}

.status-bad {
  background-color: alpha(@error_color, 0.16);
  color: @error_color;
}

.status-info {
  background-color: alpha(@accent_color, 0.15);
  color: @accent_color;
}

.status-neutral {
  background-color: alpha(currentColor, 0.10);
}

.status-icon-good {
  color: @success_color;
}

.status-icon-warning {
  color: @warning_color;
}

.status-icon-bad {
  color: @error_color;
}

.status-icon-info {
  color: @accent_color;
}

.search-toolbar {
  margin-left: 32px;
  margin-right: 32px;
}

.content-clamp {
  margin-left: 12px;
  margin-right: 12px;
}

.column-resize-handle {
  min-width: 10px;
  border-radius: 999px;
}

.column-resize-handle:hover {
  background-color: alpha(currentColor, 0.12);
}

.column-resize-handle-active {
  background-color: alpha(@accent_color, 0.18);
}

.column-resize-line {
  background-color: alpha(currentColor, 0.28);
  min-width: 1px;
  border-radius: 999px;
}

.filter-status-dot {
  border-radius: 999px;
  min-width: 10px;
  min-height: 10px;
}

.filter-status-dot.status-good {
  background-color: @success_color;
  color: @success_color;
}

.filter-status-dot.status-warning {
  background-color: @warning_color;
  color: @warning_color;
}

.filter-status-dot.status-bad {
  background-color: @error_color;
  color: @error_color;
}

.filter-status-dot.status-neutral {
  background-color: alpha(currentColor, 0.45);
}

.filter-chip {
  border-radius: 999px;
  padding: 7px 12px;
  background-color: alpha(currentColor, 0.08);
  min-height: 22px;
}

flowboxchild.filter-chip-child,
flowboxchild.filter-chip-child:hover,
flowboxchild.filter-chip-child:selected,
flowboxchild.filter-chip-child:selected:hover {
  background: transparent;
  border-radius: 999px;
  box-shadow: none;
}

.filter-chip:hover {
  background-color: alpha(currentColor, 0.11);
}

.filter-chip-active {
  background-color: alpha(@accent_color, 0.18);
  color: @accent_color;
  font-weight: 700;
}

.filter-chip-active:hover {
  background-color: alpha(@accent_color, 0.24);
}
"#;

#[derive(Debug, Clone)]
pub(super) enum ClusterSummaryState {
    Loading,
    Loaded(ClusterSummary),
    Error(String),
}

pub struct App {
    projects: ProjectStore,
    contexts: Vec<ContextInfo>,
    namespaces: Vec<String>,
    resources: Vec<ResourceKind>,
    objects: Vec<ObjectSummary>,
    /// Bumped on every `rebuild_object_list` call so a still-running
    /// chunked render of a stale list can detect it's obsolete and stop
    /// instead of appending rows after a newer rebuild already started.
    object_list_generation: std::rc::Rc<std::cell::Cell<u64>>,
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
    split_view: adw::NavigationSplitView,
    project_list: gtk::ListBox,
    project_title_label: gtk::Label,
    add_project_button: gtk::Button,
    cluster_back_button: gtk::Button,
    cluster_menu_button: gtk::MenuButton,
    cluster_refresh_button: gtk::Button,
    context_selector_label: gtk::Label,
    cluster_list: gtk::ListBox,
    add_cluster_button: gtk::Button,
    import_cluster_button: gtk::Button,
    /// State/Provider/Version/CPU/Memory/Pods snapshot per context name,
    /// fetched lazily the first time the Clusters page shows a given
    /// context and cached until the app restarts.
    cluster_summaries: std::collections::HashMap<String, ClusterSummaryState>,
    namespace_menu_button: gtk::MenuButton,
    namespace_selector_label: gtk::Label,
    namespace_list: gtk::ListBox,
    search_entry: gtk::SearchEntry,
    status_filter_list: gtk::FlowBox,
    column_filter_list: gtk::FlowBox,
    create_yaml_button: gtk::Button,
    refresh_button: gtk::Button,
    detail_back_button: gtk::Button,
    content_title_label: gtk::Label,
    content_header_stack: gtk::Stack,
    content_stack: gtk::Stack,
    status_label: gtk::Label,
    spinner: gtk::Spinner,
    resource_list: gtk::ListBox,
    object_list: gtk::ListBox,
    detail_stack: gtk::Stack,
    detail_name_label: gtk::Label,
    detail_namespace_label: gtk::Label,
    detail_status_label: gtk::Label,
    detail_kind_label: gtk::Label,
    detail_api_label: gtk::Label,
    detail_age_label: gtk::Label,
    detail_cpu_label: gtk::Label,
    detail_memory_label: gtk::Label,
    detail_container_metrics_list: gtk::ListBox,
    detail_scale_spin: gtk::SpinButton,
    detail_scale_button: gtk::Button,
    detail_cordon_button: gtk::Button,
    detail_drain_button: gtk::Button,
    detail_explain_yaml_button: gtk::Button,
    detail_apply_button: gtk::Button,
    detail_download_yaml_button: gtk::Button,
    detail_delete_button: gtk::Button,
    detail_terminal_button: gtk::Button,
    detail_yaml_buffer: sourceview5::Buffer,
    detail_events_list: gtk::ListBox,
    detail_conditions_list: gtk::ListBox,
    detail_related_pods_list: gtk::ListBox,
    detail_log_container_dropdown: gtk::DropDown,
    detail_log_follow_check: gtk::CheckButton,
    detail_log_timestamps_check: gtk::CheckButton,
    detail_log_start_button: gtk::Button,
    detail_log_stop_button: gtk::Button,
    detail_log_status_label: gtk::Label,
    detail_log_buffer: gtk::TextBuffer,
    detail_log_view: gtk::TextView,
    detail_port_local_spin: gtk::SpinButton,
    detail_port_remote_spin: gtk::SpinButton,
    detail_port_start_button: gtk::Button,
    detail_port_stop_button: gtk::Button,
    detail_port_status_label: gtk::Label,
    detail_port_forward_group: gtk::Box,
    detail_overview_section: gtk::Box,
    detail_expand_logs_button: gtk::Button,
    detail_target: Option<DetailTarget>,
    detail_log_target: Option<PodLogTarget>,
    detail_exec_target: Option<PodLogTarget>,
    detail_port_forward_target: Option<PodLogTarget>,
    detail_related_pods: Vec<ObjectSummary>,
    detail_node_unschedulable: Option<bool>,
    object_watch_token: u64,
    object_watch_abort_handle: Option<AbortHandle>,
    detail_request_token: u64,
    log_streaming: bool,
    log_stream_token: u64,
    log_abort_handle: Option<AbortHandle>,
    exec_token: u64,
    terminal_sessions: HashMap<u64, TerminalSession>,
    port_forwarding: bool,
    port_forward_token: u64,
    port_forward_abort_handle: Option<AbortHandle>,
    custom_namespace_dialog: adw::Dialog,
    custom_namespace_entry: gtk::Entry,
    custom_namespace_button: gtk::Button,
    project_dialog: adw::Dialog,
    project_dialog_description: gtk::Label,
    project_name_entry: gtk::Entry,
    project_create_button: gtk::Button,
    /// The project's name before this edit, when the project dialog is in
    /// rename mode. `None` means the dialog is creating a new project.
    editing_project_name: Option<String>,
    create_yaml_dialog: adw::Dialog,
    create_yaml_buffer: sourceview5::Buffer,
    create_yaml_apply_button: gtk::Button,
    cluster_dialog: adw::Dialog,
    cluster_dialog_stack: gtk::Stack,
    cluster_token_title_label: gtk::Label,
    cluster_token_back_button: gtk::Button,
    setup_name_entry: gtk::Entry,
    setup_server_entry: gtk::Entry,
    setup_token_entry: gtk::PasswordEntry,
    setup_ca_entry: gtk::Entry,
    setup_insecure_check: gtk::CheckButton,
    setup_button: gtk::Button,
    editing_cluster: bool,
    editing_context_name: Option<String>,
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
