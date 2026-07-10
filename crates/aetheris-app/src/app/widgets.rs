use super::*;

pub(super) fn section_title(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class("heading");
    label
}

pub(super) fn selector_button_child(label: &gtk::Label) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_margin_start(8);
    content.set_margin_end(8);
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_max_width_chars(24);
    content.append(label);
    let arrow = gtk::Image::from_icon_name("pan-down-symbolic");
    arrow.add_css_class("dim-label");
    content.append(&arrow);
    content
}

pub(super) fn selector_popover(list: &gtk::ListBox) -> gtk::Popover {
    let popover = gtk::Popover::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);

    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text(tr("Filter namespaces"))
        .build();
    content.append(&search_entry);

    let query = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    list.set_filter_func({
        let query = query.clone();
        move |row| {
            let query = query.borrow();
            if query.is_empty() {
                return true;
            }
            // Action rows ("Add namespace") stay visible no matter the
            // query — they are commands, not candidates being filtered.
            if row.widget_name() == "selector-action-row" {
                return true;
            }
            row.child()
                .and_then(|child| child.downcast::<adw::ActionRow>().ok())
                .map(|action| action.title().to_lowercase().contains(query.as_str()))
                .unwrap_or(true)
        }
    });
    search_entry.connect_search_changed({
        let list = list.clone();
        move |entry| {
            *query.borrow_mut() = entry.text().to_lowercase();
            list.invalidate_filter();
        }
    });
    // A stale query from the last time the popover was open would silently
    // hide namespaces the next time it pops up.
    popover.connect_closed({
        let search_entry = search_entry.clone();
        move |_| search_entry.set_text("")
    });
    // The list can hold dozens of entries (one per namespace); without a
    // height cap the popover grows past the monitor edge, so size to the
    // content only up to a limit and scroll beyond it.
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .propagate_natural_width(true)
        .max_content_height(420)
        .child(list)
        .build();
    content.append(&scroller);
    popover.set_child(Some(&content));
    popover
}

pub(super) fn resource_count_label(count: usize) -> String {
    format!(
        "{} {}",
        count,
        trn("resource type", "resource types", count as u32)
    )
}

pub(super) fn namespace_selector_row(
    namespace: &str,
    selected: bool,
    is_custom: bool,
    sender: Option<ComponentSender<App>>,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(true);

    let action = adw::ActionRow::builder()
        .title(namespace)
        .activatable(true)
        .build();
    action.add_prefix(&gtk::Image::from_icon_name("folder-symbolic"));

    if is_custom && let Some(sender) = sender {
        let edit_button = gtk::Button::from_icon_name("document-edit-symbolic");
        edit_button.add_css_class("flat");
        edit_button.set_valign(gtk::Align::Center);
        edit_button.set_tooltip_text(Some(&tr("Rename")));
        edit_button.connect_clicked({
            let sender = sender.clone();
            let namespace = namespace.to_owned();
            move |_| sender.input(AppMsg::OpenRenameNamespaceDialog(namespace.clone()))
        });
        action.add_suffix(&edit_button);

        let delete_button = gtk::Button::from_icon_name("user-trash-symbolic");
        delete_button.add_css_class("flat");
        delete_button.add_css_class("destructive-action");
        delete_button.set_valign(gtk::Align::Center);
        delete_button.set_tooltip_text(Some(&tr("Remove")));
        delete_button.connect_clicked({
            let namespace = namespace.to_owned();
            move |_| sender.input(AppMsg::RemoveCustomNamespace(namespace.clone()))
        });
        action.add_suffix(&delete_button);
    }

    if selected {
        row.add_css_class("resource-row-selected");
    }

    row.set_child(Some(&action));
    row
}

pub(super) fn add_namespace_selector_row() -> gtk::ListBoxRow {
    selector_action_row(&tr("Add namespace"), "list-add-symbolic")
}

fn selector_action_row(title: &str, icon_name: &str) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_widget_name("selector-action-row");
    row.set_activatable(true);
    let action = adw::ActionRow::builder()
        .title(title)
        .activatable(true)
        .build();
    action.add_prefix(&gtk::Image::from_icon_name(icon_name));
    row.set_child(Some(&action));
    row
}

#[derive(Clone, Copy)]
pub(super) enum StatusTone {
    Good,
    Warning,
    Bad,
    Info,
    Neutral,
}

impl StatusTone {
    pub(super) fn css_class(self) -> &'static str {
        match self {
            Self::Good => "status-good",
            Self::Warning => "status-warning",
            Self::Bad => "status-bad",
            Self::Info => "status-info",
            Self::Neutral => "status-neutral",
        }
    }
}

pub(super) fn status_chip(text: &str, tone: StatusTone) -> gtk::Label {
    let label = gtk::Label::builder()
        .label(text)
        .valign(gtk::Align::Center)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .max_width_chars(18)
        .build();
    label.add_css_class("status-chip");
    label.add_css_class(tone.css_class());
    label
}

pub(super) fn status_tone(status: &str) -> StatusTone {
    match status.to_ascii_lowercase().as_str() {
        "ready" | "running" | "complete" | "active" | "clusterip" | "nodeport" | "loadbalancer" => {
            StatusTone::Good
        }
        "updating" | "progressing" | "pending" | "scheduled" | "suspended" | "scaled" => {
            StatusTone::Warning
        }
        "failed" | "error" | "unavailable" | "notready" | "unknown" => StatusTone::Bad,
        "managed" | "externalname" => StatusTone::Info,
        _ => StatusTone::Neutral,
    }
}

pub(super) fn rebuild_status_filter_list(list: &gtk::FlowBox, selected: &BTreeSet<StatusFilter>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    for filter in StatusFilter::ALL {
        list.insert(&status_filter_chip(filter, selected.contains(&filter)), -1);
    }
}

pub(super) fn rebuild_column_filter_list(
    list: &gtk::FlowBox,
    offerable_columns: &[ObjectColumn],
    visible_columns: &[ObjectColumn],
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    for column in offerable_columns.iter().copied() {
        list.insert(
            &column_filter_chip(column, visible_columns.contains(&column)),
            -1,
        );
    }
}

fn column_filter_chip(column: ObjectColumn, visible: bool) -> gtk::FlowBoxChild {
    filter_chip("view-list-symbolic", &column.label(), None, visible)
}

fn status_filter_chip(filter: StatusFilter, selected: bool) -> gtk::FlowBoxChild {
    filter_chip(
        "",
        &filter.label(),
        Some(status_filter_tone(filter)),
        selected,
    )
}

fn filter_chip(
    icon_name: &str,
    label: &str,
    tone: Option<StatusTone>,
    active: bool,
) -> gtk::FlowBoxChild {
    let child = gtk::FlowBoxChild::new();
    child.add_css_class("filter-chip-child");
    child.set_valign(gtk::Align::Center);

    let chip = gtk::Box::new(gtk::Orientation::Horizontal, 7);
    chip.add_css_class("filter-chip");
    chip.set_valign(gtk::Align::Center);
    chip.set_size_request(92, 34);
    if active {
        chip.add_css_class("filter-chip-active");
    }

    if let Some(tone) = tone {
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.add_css_class("filter-status-dot");
        dot.add_css_class(tone.css_class());
        dot.set_valign(gtk::Align::Center);
        dot.set_size_request(10, 10);
        chip.append(&dot);
    } else {
        let icon = gtk::Image::from_icon_name(icon_name);
        icon.set_pixel_size(14);
        if !active {
            icon.add_css_class("dim-label");
        }
        chip.append(&icon);
    }

    let text = gtk::Label::builder()
        .label(label)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();
    chip.append(&text);

    if active {
        let check = gtk::Image::from_icon_name("object-select-symbolic");
        check.set_pixel_size(14);
        chip.append(&check);
    }

    child.set_child(Some(&chip));
    child
}

fn status_filter_tone(filter: StatusFilter) -> StatusTone {
    match filter {
        StatusFilter::Ready | StatusFilter::Available | StatusFilter::Running => StatusTone::Good,
        StatusFilter::Pending | StatusFilter::Unavailable => StatusTone::Warning,
        StatusFilter::Failed => StatusTone::Bad,
    }
}

pub(super) fn rebuild_project_list(list: &gtk::ListBox, projects: &ProjectStore) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    for project in &projects.projects {
        let selected = projects.selected_project.as_deref() == Some(project.name.as_str());
        list.append(&project_row(project, selected));
    }
}

pub(super) fn project_row(project: &Project, selected: bool) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(true);
    if selected {
        row.add_css_class("resource-row-selected");
    }

    let container = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    container.set_hexpand(true);
    container.set_margin_top(14);
    container.set_margin_bottom(14);
    container.set_margin_start(14);
    container.set_margin_end(12);

    let icon_frame = gtk::CenterBox::new();
    icon_frame.set_size_request(44, 44);
    icon_frame.set_valign(gtk::Align::Center);
    icon_frame.set_halign(gtk::Align::Center);
    icon_frame.add_css_class("project-icon");
    let icon = gtk::Image::from_icon_name("folder-symbolic");
    icon.set_pixel_size(24);
    icon_frame.set_center_widget(Some(&icon));
    container.append(&icon_frame);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);

    let title = gtk::Label::new(Some(project.name.as_str()));
    title.set_xalign(0.0);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.add_css_class("heading");
    text.append(&title);

    let subtitle = gtk::Label::new(Some(&project_context_count(project.contexts.len())));
    subtitle.set_xalign(0.0);
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
    subtitle.add_css_class("dim-label");
    text.append(&subtitle);
    container.append(&text);

    let arrow = gtk::Image::from_icon_name("go-next-symbolic");
    arrow.add_css_class("dim-label");
    arrow.set_valign(gtk::Align::Center);
    container.append(&arrow);

    row.set_child(Some(&container));
    row
}

pub(super) fn project_context_count(count: usize) -> String {
    format!(
        "{} {}",
        count,
        if count == 1 { "cluster" } else { "clusters" }
    )
}

pub(super) fn rebuild_cluster_list(
    list: &gtk::ListBox,
    contexts: &[&ContextInfo],
    summaries: &std::collections::HashMap<String, ClusterSummaryState>,
    selected: Option<&str>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    for context in contexts {
        let is_selected = selected == Some(context.name.as_str());
        let summary = summaries.get(&context.name);
        list.append(&cluster_row(context, summary, is_selected));
    }
}

pub(super) fn cluster_row(
    context: &ContextInfo,
    summary: Option<&ClusterSummaryState>,
    selected: bool,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(true);
    if selected {
        row.add_css_class("resource-row-selected");
    }

    let container = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    container.set_margin_top(14);
    container.set_margin_bottom(14);
    container.set_margin_start(14);
    container.set_margin_end(12);

    let icon_frame = gtk::CenterBox::new();
    icon_frame.set_size_request(44, 44);
    icon_frame.set_valign(gtk::Align::Center);
    icon_frame.set_halign(gtk::Align::Center);
    icon_frame.add_css_class("project-icon");
    let icon = gtk::Image::from_icon_name("network-server-symbolic");
    icon.set_pixel_size(24);
    icon_frame.set_center_widget(Some(&icon));
    container.append(&icon_frame);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);

    let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    title_row.set_hexpand(true);
    let title = gtk::Label::new(Some(context.name.as_str()));
    title.set_xalign(0.0);
    title.set_hexpand(false);
    title.set_max_width_chars(34);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.add_css_class("heading");
    title_row.append(&title);
    title_row.append(&cluster_state_chip(summary));
    text.append(&title_row);

    let subtitle = gtk::Label::new(Some(&cluster_subtitle_text(summary)));
    subtitle.set_xalign(0.0);
    subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
    subtitle.add_css_class("dim-label");
    text.append(&subtitle);
    container.append(&text);

    let arrow = gtk::Image::from_icon_name("go-next-symbolic");
    arrow.add_css_class("dim-label");
    arrow.set_valign(gtk::Align::Center);
    container.append(&arrow);

    row.set_child(Some(&container));
    row
}

fn cluster_state_chip(summary: Option<&ClusterSummaryState>) -> gtk::Label {
    let (text, tone, tooltip) = match summary {
        None | Some(ClusterSummaryState::Loading) => (tr("Checking..."), StatusTone::Neutral, None),
        Some(ClusterSummaryState::Loaded(_)) => (tr("Active"), StatusTone::Good, None),
        Some(ClusterSummaryState::Error(error)) => {
            (tr("Unreachable"), StatusTone::Bad, Some(error.as_str()))
        }
    };
    let chip = status_chip(&text, tone);
    if let Some(tooltip) = tooltip {
        chip.set_tooltip_text(Some(tooltip));
    }
    chip
}

fn cluster_subtitle_text(summary: Option<&ClusterSummaryState>) -> String {
    match summary {
        None | Some(ClusterSummaryState::Loading) => tr("Checking cluster..."),
        Some(ClusterSummaryState::Error(_)) => tr("Could not reach this cluster."),
        Some(ClusterSummaryState::Loaded(data)) => {
            let parts: Vec<&str> = [data.provider.as_deref(), data.version.as_deref()]
                .into_iter()
                .flatten()
                .collect();
            if parts.is_empty() {
                tr("Kubernetes cluster")
            } else {
                parts.join(" · ")
            }
        }
    }
}

pub(super) fn connect_resource_row(
    row: &adw::ActionRow,
    sender: Option<ComponentSender<App>>,
    resource_index: usize,
    section: ResourceSection,
) {
    let Some(sender) = sender else {
        return;
    };

    row.connect_activated(move |_| {
        sender.input(AppMsg::ResourceChanged(resource_index, section));
    });
}

pub(super) fn connect_favorite_object_row(
    row: &adw::ActionRow,
    sender: Option<ComponentSender<App>>,
    favorite: ObjectFavorite,
) {
    let Some(sender) = sender else {
        return;
    };

    row.connect_activated(move |_| {
        sender.input(AppMsg::FavoriteObjectActivated(favorite.clone()));
    });
}

pub(super) fn resource_row(resource: &ResourceKind, selected: bool) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(resource.kind.as_str())
        .subtitle(format!(
            "{} · {}",
            if resource.group.is_empty() {
                "core"
            } else {
                resource.group.as_str()
            },
            resource.api_version
        ))
        .activatable(true)
        .build();
    let icon = gtk::Image::from_icon_name(available_icon_name(
        resource_icon_name(resource),
        "application-x-addon-symbolic",
    ));
    row.add_prefix(&icon);

    if selected {
        row.add_css_class("resource-row-selected");
    }

    row
}

pub(super) fn favorite_object_row(favorite: &ObjectFavorite) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(favorite.name.as_str())
        .subtitle(favorite.kind())
        .title_lines(1)
        .subtitle_lines(1)
        .activatable(true)
        .tooltip_text(favorite.name.as_str())
        .build();
    let icon = gtk::Image::from_icon_name(available_icon_name(
        resource_icon_name(&favorite.resource()),
        "application-x-addon-symbolic",
    ));
    row.add_prefix(&icon);
    row
}

pub(super) fn resource_icon_name(resource: &ResourceKind) -> &'static str {
    match resource.group.as_str() {
        "" => match resource.kind.as_str() {
            "Pod" => "lucide-box-symbolic",
            "ConfigMap" => "lucide-file-sliders-symbolic",
            "Secret" => "lucide-file-key-2-symbolic",
            "Namespace" => "lucide-orbit-symbolic",
            "Service" => "lucide-waypoints-symbolic",
            "Node" => "lucide-server-symbolic",
            "PersistentVolume" => "lucide-hard-drive-download-symbolic",
            "PersistentVolumeClaim" => "lucide-hard-drive-upload-symbolic",
            "Event" => "dialog-information-symbolic",
            "ServiceAccount" => "lucide-user-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "apps" => match resource.kind.as_str() {
            "ReplicaSet" => "lucide-layers-2-symbolic",
            "Deployment" => "lucide-layers-3-symbolic",
            "StatefulSet" => "lucide-database-symbolic",
            "DaemonSet" => "lucide-server-cog-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "batch" => match resource.kind.as_str() {
            "Job" => "lucide-cloud-cog-symbolic",
            "CronJob" => "lucide-timer-reset-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "networking.k8s.io" => match resource.kind.as_str() {
            "Ingress" => "lucide-radio-tower-symbolic",
            "IngressClass" => "lucide-cast-symbolic",
            "NetworkPolicy" => "lucide-globe-lock-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "events.k8s.io" => match resource.kind.as_str() {
            "Event" => "dialog-information-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "apiextensions.k8s.io" => match resource.kind.as_str() {
            "CustomResourceDefinition" => "lucide-toy-brick-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "storage.k8s.io" => match resource.kind.as_str() {
            "CSIDriver" => "lucide-warehouse-symbolic",
            "CSINode" => "lucide-cylinder-symbolic",
            "StorageClass" => "lucide-import-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "helm.toolkit.fluxcd.io" => match resource.kind.as_str() {
            "HelmRelease" => "lucide-package-open-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "source.toolkit.fluxcd.io" => match resource.kind.as_str() {
            "HelmChart" => "lucide-map-symbolic",
            "HelmRepository" => "lucide-library-symbolic",
            "GitRepository" => "lucide-folder-git-symbolic",
            "Bucket" => "lucide-paint-bucket-symbolic",
            "OCIRepository" => "lucide-container-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        "monitoring.coreos.com" => match resource.kind.as_str() {
            "PodMonitor" => "lucide-package-search-symbolic",
            _ => "lucide-blocks-symbolic",
        },
        _ => "lucide-blocks-symbolic",
    }
}

pub(super) fn is_workload_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "Pod" | "Deployment" | "ReplicaSet" | "StatefulSet" | "DaemonSet" | "Job" | "CronJob"
    ) || matches!(resource.group.as_str(), "apps" | "batch")
}

pub(super) fn is_network_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "Service" | "Ingress" | "EndpointSlice" | "NetworkPolicy" | "Endpoints"
    ) || matches!(
        resource.group.as_str(),
        "networking.k8s.io" | "discovery.k8s.io"
    )
}

pub(super) fn is_storage_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "PersistentVolume"
            | "PersistentVolumeClaim"
            | "StorageClass"
            | "CSIDriver"
            | "CSINode"
            | "CSIStorageCapacity"
            | "VolumeAttachment"
    ) || resource.group == "storage.k8s.io"
}

pub(super) fn is_configuration_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "ConfigMap"
            | "Secret"
            | "ResourceQuota"
            | "LimitRange"
            | "HorizontalPodAutoscaler"
            | "PodDisruptionBudget"
            | "PriorityClass"
    ) || matches!(
        resource.group.as_str(),
        "autoscaling" | "policy" | "scheduling.k8s.io"
    )
}

pub(super) fn is_access_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "ServiceAccount"
            | "Role"
            | "RoleBinding"
            | "ClusterRole"
            | "ClusterRoleBinding"
            | "CertificateSigningRequest"
    ) || matches!(
        resource.group.as_str(),
        "rbac.authorization.k8s.io" | "certificates.k8s.io"
    )
}

pub(super) fn is_cluster_resource(resource: &ResourceKind) -> bool {
    matches!(
        resource.kind.as_str(),
        "Namespace"
            | "Node"
            | "ComponentStatus"
            | "RuntimeClass"
            | "CustomResourceDefinition"
            | "APIService"
            | "MutatingWebhookConfiguration"
            | "ValidatingWebhookConfiguration"
    )
}

const RELATED_POD_COLUMNS: [ObjectColumn; 4] = [
    ObjectColumn::Image,
    ObjectColumn::Namespace,
    ObjectColumn::Api,
    ObjectColumn::Age,
];

/// The virtualized related-Pods table for the detail page's "Pods" tab:
/// same cell factories as the main object table, default widths, no
/// persistence. Returns the sorted model too — activation positions are
/// indices into it, not into the unsorted store.
pub(super) fn related_pods_column_view()
-> (gtk::ColumnView, gtk::gio::ListStore, gtk::SortListModel) {
    let store = gtk::gio::ListStore::new::<gtk::glib::BoxedAnyObject>();
    let view = gtk::ColumnView::builder()
        .single_click_activate(true)
        .reorderable(false)
        .build();
    view.add_css_class("aetheris-table");
    view.set_vexpand(true);

    let name_column =
        gtk::ColumnViewColumn::new(Some(&tr("Name")), Some(object_name_column_factory()));
    name_column.set_resizable(true);
    name_column.set_fixed_width(OBJECT_NAME_WIDTH);
    view.append_column(&name_column);
    for column in RELATED_POD_COLUMNS {
        let view_column = gtk::ColumnViewColumn::new(
            Some(&column.label()),
            Some(object_data_column_factory(column)),
        );
        view_column.set_resizable(true);
        view_column.set_fixed_width(column.default_width());
        view_column.set_sorter(object_column_sorter(column).as_ref());
        view.append_column(&view_column);
    }
    append_filler_column(&view);

    let sorted = gtk::SortListModel::new(Some(store.clone()), view.sorter());
    view.set_model(Some(&gtk::NoSelection::new(Some(sorted.clone()))));
    connect_sorted_header_highlight(&view);
    (view, store, sorted)
}

/// Trailing zero-content column that soaks up leftover width, so the
/// header background always reaches the table's right edge instead of
/// stopping after the last real column.
pub(super) fn append_filler_column(view: &gtk::ColumnView) {
    let filler = gtk::ColumnViewColumn::new(None, None::<gtk::ListItemFactory>);
    filler.set_expand(true);
    view.append_column(&filler);
}

/// Mirrors the active sort column onto its header button via a "sorted"
/// CSS class. GTK itself only draws the small direction arrow and exposes
/// no styleable state for the sorted column, so this walks the header's
/// buttons (one per column, same order) whenever the view's sorter fires.
pub(super) fn connect_sorted_header_highlight(view: &gtk::ColumnView) {
    let Some(sorter) = view.sorter() else {
        return;
    };
    let view = view.downgrade();
    sorter.connect_changed(move |sorter, _| {
        let Some(view) = view.upgrade() else {
            return;
        };
        let Some(sorter) = sorter.downcast_ref::<gtk::ColumnViewSorter>() else {
            return;
        };
        let primary = sorter.primary_sort_column();
        let Some(header) = column_view_header(&view) else {
            return;
        };
        let columns = view.columns();
        let mut index = 0;
        let mut child = header.first_child();
        while let Some(button) = child {
            child = button.next_sibling();
            let column = columns.item(index).and_downcast::<gtk::ColumnViewColumn>();
            if column.is_some() && column == primary {
                button.add_css_class("sorted");
            } else {
                button.remove_css_class("sorted");
            }
            index += 1;
        }
    });
}

fn column_view_header(view: &gtk::ColumnView) -> Option<gtk::Widget> {
    let mut child = view.first_child();
    while let Some(widget) = child {
        if widget.css_name() == "header" {
            return Some(widget);
        }
        child = widget.next_sibling();
    }
    None
}

pub(super) fn object_column_sorter(column: ObjectColumn) -> Option<gtk::CustomSorter> {
    match column {
        ObjectColumn::Image => Some(summary_sorter(|a, b| {
            super::utils::pod_main_image(&a.images)
                .cmp(&super::utils::pod_main_image(&b.images))
                .then_with(|| a.name.cmp(&b.name))
        })),
        ObjectColumn::Namespace => Some(summary_sorter(|a, b| {
            a.namespace
                .cmp(&b.namespace)
                .then_with(|| a.name.cmp(&b.name))
        })),
        ObjectColumn::Target => Some(summary_sorter(|a, b| {
            a.service_target
                .cmp(&b.service_target)
                .then_with(|| a.name.cmp(&b.name))
        })),
        ObjectColumn::Selector => Some(summary_sorter(|a, b| {
            a.service_selector
                .cmp(&b.service_selector)
                .then_with(|| a.name.cmp(&b.name))
        })),
        ObjectColumn::IngressClass => Some(summary_sorter(|a, b| {
            a.ingress_class
                .cmp(&b.ingress_class)
                .then_with(|| a.name.cmp(&b.name))
        })),
        ObjectColumn::Cpu | ObjectColumn::Memory => Some(summary_sorter(move |a, b| {
            // Usage percentage is the primary key; raw quantity only breaks
            // ties among objects with no percentage (no requests set), and
            // `None` (no metrics sample) groups at one end.
            let (a_ratio, a_raw) = metric_sort_key(a, column);
            let (b_ratio, b_raw) = metric_sort_key(b, column);
            a_ratio
                .cmp(&b_ratio)
                .then_with(|| {
                    a_raw
                        .partial_cmp(&b_raw)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.name.cmp(&b.name))
        })),
        _ => None,
    }
}

fn metric_sort_key(object: &ObjectSummary, column: ObjectColumn) -> (Option<u32>, Option<f64>) {
    let Some(usage) = object.metrics.as_ref() else {
        return (None, None);
    };
    let (ratio, raw) = match column {
        ObjectColumn::Cpu => (usage.cpu_ratio.as_ref(), &usage.cpu),
        ObjectColumn::Memory => (usage.memory_ratio.as_ref(), &usage.memory),
        _ => return (None, None),
    };
    (
        ratio.map(|ratio| ratio.basis_points),
        super::utils::parse_quantity(raw),
    )
}

fn summary_sorter(
    compare: impl Fn(&ObjectSummary, &ObjectSummary) -> std::cmp::Ordering + 'static,
) -> gtk::CustomSorter {
    gtk::CustomSorter::new(move |a, b| {
        let (Some(a), Some(b)) = (
            a.downcast_ref::<gtk::glib::BoxedAnyObject>(),
            b.downcast_ref::<gtk::glib::BoxedAnyObject>(),
        ) else {
            return gtk::Ordering::Equal;
        };
        compare(&a.borrow::<ObjectSummary>(), &b.borrow::<ObjectSummary>()).into()
    })
}

pub(super) fn connect_object_column_persistence(
    view_column: &gtk::ColumnViewColumn,
    table_column: ObjectTableColumn,
    sender: ComponentSender<App>,
) {
    view_column.connect_fixed_width_notify(move |view_column| {
        let width = view_column.fixed_width();
        let clamped = clamp_table_column_width(table_column, width);
        if clamped != width {
            view_column.set_fixed_width(clamped);
            return;
        }
        sender.input(AppMsg::ObjectColumnResized(table_column, clamped));
    });
}

fn clamp_table_column_width(column: ObjectTableColumn, width: i32) -> i32 {
    match column {
        ObjectTableColumn::Name => width.max(OBJECT_NAME_MIN_WIDTH),
        ObjectTableColumn::Data(_) => width.max(OBJECT_COLUMN_MIN_WIDTH),
    }
}

pub(super) fn boxed_object(object: &ObjectSummary) -> gtk::glib::BoxedAnyObject {
    gtk::glib::BoxedAnyObject::new(object.clone())
}

fn list_item_object(
    item: &gtk::glib::Object,
) -> Option<(gtk::ListItem, gtk::glib::BoxedAnyObject)> {
    let item = item.downcast_ref::<gtk::ListItem>()?.clone();
    let boxed = item.item().and_downcast::<gtk::glib::BoxedAnyObject>()?;
    Some((item, boxed))
}

pub(super) fn object_name_column_factory() -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let cell = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        cell.set_valign(gtk::Align::Center);
        cell.set_margin_top(6);
        cell.set_margin_bottom(6);
        item.set_child(Some(&cell));
    });
    factory.connect_bind(|_, item| {
        let Some((item, boxed)) = list_item_object(item) else {
            return;
        };
        let Some(cell) = item.child().and_downcast::<gtk::Box>() else {
            return;
        };
        while let Some(child) = cell.first_child() {
            cell.remove(&child);
        }
        let object = boxed.borrow::<ObjectSummary>();
        if has_meaningful_status(&object.status) {
            cell.append(&status_prefix_chip(&object.status));
        }
        let name = gtk::Label::new(Some(&object.name));
        name.set_xalign(0.0);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name.add_css_class("heading");
        name.set_tooltip_text(Some(&object.name));
        cell.append(&name);
    });
    factory
}

pub(super) fn object_data_column_factory(column: ObjectColumn) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    match column {
        ObjectColumn::Cpu | ObjectColumn::Memory => {
            factory.connect_setup(|_, item| {
                let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
                    return;
                };
                let cell = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                cell.set_valign(gtk::Align::Center);
                item.set_child(Some(&cell));
            });
            factory.connect_bind(move |_, item| {
                let Some((item, boxed)) = list_item_object(item) else {
                    return;
                };
                let Some(cell) = item.child().and_downcast::<gtk::Box>() else {
                    return;
                };
                while let Some(child) = cell.first_child() {
                    cell.remove(&child);
                }
                cell.set_tooltip_text(None);
                let object = boxed.borrow::<ObjectSummary>();
                cell.append(&metric_bar_with_width(
                    object.metrics.as_ref(),
                    column,
                    OBJECT_METRIC_WIDTH,
                ));
            });
        }
        _ => {
            factory.connect_setup(|_, item| {
                let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
                    return;
                };
                let label = gtk::Label::new(None);
                label.set_xalign(0.0);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                item.set_child(Some(&label));
            });
            factory.connect_bind(move |_, item| {
                let Some((item, boxed)) = list_item_object(item) else {
                    return;
                };
                let Some(label) = item.child().and_downcast::<gtk::Label>() else {
                    return;
                };
                let object = boxed.borrow::<ObjectSummary>();
                let (text, tooltip) = match column {
                    ObjectColumn::Namespace => (object.namespace.clone(), None),
                    ObjectColumn::Target => {
                        let target = object_target(&object);
                        (
                            target.to_owned(),
                            (!target.is_empty()).then(|| target.to_owned()),
                        )
                    }
                    ObjectColumn::Selector => (
                        object.service_selector.clone(),
                        (!object.service_selector.is_empty())
                            .then(|| object.service_selector.clone()),
                    ),
                    ObjectColumn::IngressClass => (
                        object.ingress_class.clone(),
                        (!object.ingress_class.is_empty()).then(|| object.ingress_class.clone()),
                    ),
                    ObjectColumn::Image => {
                        let Some(main_image) = super::utils::pod_main_image(&object.images) else {
                            return;
                        };
                        let extra = object.images.len().saturating_sub(1);
                        let text = if extra > 0 {
                            format!(
                                "{} {}",
                                main_image,
                                tr_format("+ {count} more", &[("{count}", extra.to_string())])
                            )
                        } else {
                            main_image
                        };
                        let tooltip = object
                            .images
                            .iter()
                            .map(|image| super::utils::shortened_image(image))
                            .collect::<Vec<_>>()
                            .join("\n");
                        (text, (!tooltip.is_empty()).then_some(tooltip))
                    }
                    ObjectColumn::Status => match object.status_ratio {
                        Some((ready, desired)) => {
                            (format!("{ready}/{desired}"), Some(object.status.clone()))
                        }
                        None => (String::new(), None),
                    },
                    ObjectColumn::Api => (object.api_version.clone(), None),
                    ObjectColumn::Age => (object.age.clone(), None),
                    ObjectColumn::Cpu | ObjectColumn::Memory => unreachable!(),
                };
                label.set_text(&text);
                label.set_tooltip_text(tooltip.as_deref());
            });
        }
    }
    factory
}

fn object_target(object: &ObjectSummary) -> &str {
    if object.service_target.is_empty() {
        &object.ingress_target
    } else {
        &object.service_target
    }
}

/// Whether a status string is actual information rather than the "no
/// status data" placeholder (e.g. ControllerRevision, which has no status
/// subresource at all).
pub(super) fn has_meaningful_status(status: &str) -> bool {
    !status.is_empty() && status != "-"
}

pub(super) fn status_prefix_chip(status: &str) -> gtk::Label {
    let unknown = tr("Unknown");
    let primary = status
        .split_whitespace()
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(&unknown);
    let chip = status_chip(primary, status_tone(primary));
    chip.set_tooltip_text(Some(status));
    // Show the full state name (e.g. "CrashLoopBackOff"); the virtualized
    // table clips the cell at the column edge, so no width cap is needed.
    chip.set_ellipsize(gtk::pango::EllipsizeMode::None);
    chip.set_max_width_chars(-1);
    chip
}

/// `usage` is `None` when metrics.k8s.io is unavailable or has no sample for
/// the object. Keep that cell blank; otherwise mirror Seabird's compact
/// LevelBar presentation and leave the raw Kubernetes quantity in the tooltip.
fn metric_bar_with_width(
    usage: Option<&ResourceUsage>,
    column: ObjectColumn,
    width: i32,
) -> gtk::Widget {
    let Some(usage) = usage else {
        return grid_label("", Some(width), false).upcast();
    };
    let (raw_value, ratio) = match column {
        ObjectColumn::Cpu => (usage.cpu.as_str(), usage.cpu_ratio.as_ref()),
        ObjectColumn::Memory => (usage.memory.as_str(), usage.memory_ratio.as_ref()),
        _ => return grid_label("", Some(width), false).upcast(),
    };
    if raw_value.is_empty() || raw_value == "-" {
        return grid_label("", Some(width), false).upcast();
    }

    let cell = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    cell.set_size_request(width, -1);
    cell.set_hexpand(false);
    cell.set_halign(gtk::Align::Start);
    cell.set_valign(gtk::Align::Center);

    let bar = gtk::LevelBar::new();
    bar.set_size_request(50.min(width.max(0)), -1);
    bar.set_halign(gtk::Align::Start);
    bar.set_valign(gtk::Align::Center);
    bar.set_min_value(0.0);
    bar.set_max_value(1.0);
    bar.remove_offset_value(Some("low"));
    bar.remove_offset_value(Some("high"));
    bar.add_offset_value("lb-normal", 0.85);
    bar.add_offset_value("lb-warning", 0.95);
    bar.add_offset_value("lb-error", 1.0);
    // Without a reference total (Pods with no resource requests set) there
    // is no percentage — keep the zeroed bar and leave the raw quantity in
    // the tooltip.
    if let Some(ratio) = ratio {
        let percent = ratio.basis_points as f64 / 100.0;
        bar.set_value((ratio.basis_points as f64 / 10_000.0).min(1.0));
        cell.set_tooltip_text(Some(&format!("{percent:.0}% ({raw_value})")));
        bar.set_tooltip_text(Some(&format!("{percent:.0}% ({raw_value})")));
    } else {
        bar.set_value(0.0);
        cell.set_tooltip_text(Some(raw_value));
        bar.set_tooltip_text(Some(raw_value));
    }
    cell.append(&bar);
    cell.upcast()
}

pub(super) fn available_icon_name<'a>(preferred: &'a str, fallback: &'a str) -> &'a str {
    let Some(display) = gtk::gdk::Display::default() else {
        return fallback;
    };
    let theme = gtk::IconTheme::for_display(&display);
    if theme.has_icon(preferred) {
        preferred
    } else {
        fallback
    }
}

pub(super) fn grid_label(text: &str, width: Option<i32>, hexpand: bool) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_hexpand(hexpand);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    if let Some(width) = width {
        set_grid_label_pixel_width(&label, width);
    }
    label
}

fn set_grid_label_pixel_width(label: &gtk::Label, width: i32) {
    // Conservative average px/char so max-width-chars' Pango-side estimate
    // stays comfortably under the pinned size_request floor for real
    // content — a tighter ratio let actual text (e.g. "apps/v1", "200d")
    // occasionally outgrow the floor even though it was within the char
    // cap, since the column's real width pinning only holds when content
    // never exceeds it.
    let chars = (width / 10).max(4);
    label.set_size_request(width, -1);
    label.set_width_chars(chars);
    label.set_max_width_chars(chars);
}

/// A Ctrl+F search bar for a plain `gtk::TextView` (the log viewer, which
/// isn't a `sourceview5::Buffer` and so can't use `SearchContext`) — uses
/// GTK's own `TextIter::forward_search`/`backward_search` instead. Returns
/// a `Revealer` to place above the view; wires Ctrl+F on `view` to reveal
/// it and Escape to hide it again.
pub(super) fn build_log_search_bar(
    view: &gtk::TextView,
    buffer: &gtk::TextBuffer,
) -> gtk::Revealer {
    let entry = gtk::SearchEntry::builder().hexpand(true).build();
    let prev_button = gtk::Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text(tr("Find previous match (Shift+Enter)"))
        .build();
    let next_button = gtk::Button::builder()
        .icon_name("go-down-symbolic")
        .tooltip_text(tr("Find next match (Enter)"))
        .build();
    let status_label = gtk::Label::new(None);
    status_label.add_css_class("dim-label");
    status_label.add_css_class("caption");
    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text(tr("Close search (Escape)"))
        .build();

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bar.set_margin_all(6);
    bar.append(&entry);
    bar.append(&status_label);
    bar.append(&prev_button);
    bar.append(&next_button);
    bar.append(&close_button);

    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .reveal_child(false)
        .build();
    revealer.set_child(Some(&bar));

    let flags = gtk::TextSearchFlags::CASE_INSENSITIVE;

    let jump_to = {
        let view = view.clone();
        let status_label = status_label.clone();
        move |buffer: &gtk::TextBuffer, found: Option<(gtk::TextIter, gtk::TextIter)>| match found {
            Some((start, end)) => {
                buffer.select_range(&start, &end);
                view.scroll_to_iter(&mut start.clone(), 0.1, false, 0.0, 0.0);
                status_label.set_label("");
            }
            None => status_label.set_label(&tr("No matches")),
        }
    };

    entry.connect_search_changed({
        let buffer = buffer.clone();
        let jump_to = jump_to.clone();
        let status_label = status_label.clone();
        move |entry| {
            let query = entry.text();
            if query.is_empty() {
                status_label.set_label("");
                return;
            }
            let found = buffer.start_iter().forward_search(&query, flags, None);
            jump_to(&buffer, found);
        }
    });

    let find_next = {
        let buffer = buffer.clone();
        let entry = entry.clone();
        let jump_to = jump_to.clone();
        move || {
            let query = entry.text();
            if query.is_empty() {
                return;
            }
            let from = buffer
                .selection_bounds()
                .map(|(_, end)| end)
                .unwrap_or_else(|| buffer.iter_at_mark(&buffer.get_insert()));
            let found = from
                .forward_search(&query, flags, None)
                .or_else(|| buffer.start_iter().forward_search(&query, flags, None));
            jump_to(&buffer, found);
        }
    };
    let find_previous = {
        let buffer = buffer.clone();
        let entry = entry.clone();
        let jump_to = jump_to.clone();
        move || {
            let query = entry.text();
            if query.is_empty() {
                return;
            }
            let from = buffer
                .selection_bounds()
                .map(|(start, _)| start)
                .unwrap_or_else(|| buffer.iter_at_mark(&buffer.get_insert()));
            let found = from
                .backward_search(&query, flags, None)
                .or_else(|| buffer.end_iter().backward_search(&query, flags, None));
            jump_to(&buffer, found);
        }
    };

    entry.connect_activate({
        let find_next = find_next.clone();
        move |_| find_next()
    });
    next_button.connect_clicked(move |_| find_next());
    prev_button.connect_clicked(move |_| find_previous());
    close_button.connect_clicked({
        let revealer = revealer.clone();
        let view = view.clone();
        move |_| {
            revealer.set_reveal_child(false);
            view.grab_focus();
        }
    });

    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed({
        let revealer = revealer.clone();
        let entry = entry.clone();
        move |_, key, _, modifiers| {
            if key == gtk::gdk::Key::f && modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                revealer.set_reveal_child(true);
                entry.grab_focus();
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        }
    });
    view.add_controller(key_controller);

    let entry_key_controller = gtk::EventControllerKey::new();
    entry_key_controller.connect_key_pressed({
        let revealer = revealer.clone();
        let view = view.clone();
        move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                revealer.set_reveal_child(false);
                view.grab_focus();
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        }
    });
    entry.add_controller(entry_key_controller);

    revealer
}
