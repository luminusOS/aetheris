use super::*;

pub(super) fn section_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class("caption-heading");
    label
}

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
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    content.append(list);
    popover.set_child(Some(&content));
    popover
}

/// A left-aligned icon + label row styled as a flat popover menu item.
pub(super) fn menu_action_button(icon_name: &str, label: &str) -> gtk::Button {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);
    content.append(&gtk::Image::from_icon_name(icon_name));
    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.0);
    text.set_hexpand(true);
    content.append(&text);

    let button = gtk::Button::new();
    button.set_child(Some(&content));
    button.add_css_class("flat");
    button
}

pub(super) fn action_menu_popover(buttons: &[&gtk::Button]) -> gtk::Popover {
    let popover = gtk::Popover::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.set_margin_all(6);
    for button in buttons {
        content.append(*button);
    }
    popover.set_child(Some(&content));
    popover
}

pub(super) fn resource_count_label(count: usize) -> String {
    format!(
        "{} {}",
        count,
        if count == 1 {
            "resource type"
        } else {
            "resource types"
        }
    )
}

pub(super) fn namespace_selector_row(
    namespace: &str,
    selected: bool,
    _index: u32,
) -> gtk::ListBoxRow {
    let row = selector_action_row(namespace, "folder-symbolic");
    if selected {
        row.add_css_class("resource-row-selected");
    }
    row
}

pub(super) fn add_namespace_selector_row() -> gtk::ListBoxRow {
    selector_action_row("Add namespace", "list-add-symbolic")
}

fn selector_action_row(title: &str, icon_name: &str) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
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
    filter_chip("view-list-symbolic", column.label(), None, visible)
}

fn status_filter_chip(filter: StatusFilter, selected: bool) -> gtk::FlowBoxChild {
    filter_chip(
        "",
        filter.label(),
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
        StatusFilter::Ready | StatusFilter::Running => StatusTone::Good,
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
    title_row.append(&cluster_state_chip(summary));
    let title = gtk::Label::new(Some(context.name.as_str()));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.add_css_class("heading");
    title_row.append(&title);
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
        None | Some(ClusterSummaryState::Loading) => ("Checking…", StatusTone::Neutral, None),
        Some(ClusterSummaryState::Loaded(_)) => ("Active", StatusTone::Good, None),
        Some(ClusterSummaryState::Error(error)) => {
            ("Unreachable", StatusTone::Bad, Some(error.as_str()))
        }
    };
    let chip = status_chip(text, tone);
    if let Some(tooltip) = tooltip {
        chip.set_tooltip_text(Some(tooltip));
    }
    chip
}

/// "{provider} · {version}" when loaded, dropping either half that's
/// unavailable; a short status line otherwise.
fn cluster_subtitle_text(summary: Option<&ClusterSummaryState>) -> String {
    match summary {
        None | Some(ClusterSummaryState::Loading) => String::from("Checking cluster…"),
        Some(ClusterSummaryState::Error(_)) => String::from("Could not reach this cluster."),
        Some(ClusterSummaryState::Loaded(data)) => {
            let parts: Vec<&str> = [data.provider.as_deref(), data.version.as_deref()]
                .into_iter()
                .flatten()
                .collect();
            if parts.is_empty() {
                String::from("Kubernetes cluster")
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
) {
    let Some(sender) = sender else {
        return;
    };

    row.connect_activated(move |_| {
        sender.input(AppMsg::ResourceChanged(resource_index));
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

    if selected {
        row.add_css_class("resource-row-selected");
    }

    row
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

/// Used for the Deployment detail page's "Pods" tab, which always lists
/// Pods as a compact preview. Pods never have a "Status" ready/desired
/// ratio, and CPU/Memory depend on metrics-server being installed (often
/// not, and not worth a column that's blank for everyone when it isn't) —
/// both are dropped here regardless of which resource kind the main object
/// list has selected.
pub(super) fn object_header() -> gtk::Box {
    object_header_with_columns(&RELATED_POD_COLUMNS)
}

const RELATED_POD_COLUMNS: [ObjectColumn; 3] = [
    ObjectColumn::Namespace,
    ObjectColumn::Api,
    ObjectColumn::Age,
];

/// A `Box`, not a `Grid`: with a `Grid`, leftover row width (none of these
/// fixed-width cells claims `hexpand`) gets redistributed across columns,
/// and exactly how much there is to redistribute can differ subtly between
/// the header row and a regular data row — drifting further with each
/// column to the right. A `Box` just places each child at its own pinned
/// width with no such redistribution, so header and rows can't diverge.
pub(super) fn object_header_with_columns(columns: &[ObjectColumn]) -> gtk::Box {
    let widths = default_column_widths(columns);
    object_header_with_column_widths(OBJECT_NAME_WIDTH, columns, &widths, None, None)
}

pub(super) fn object_header_with_column_widths(
    name_width: i32,
    columns: &[ObjectColumn],
    widths: &[i32],
    sender: Option<ComponentSender<App>>,
    list: Option<gtk::ListBox>,
) -> gtk::Box {
    let header = object_row_box();
    header.add_css_class("caption-heading");

    if let Some(sender) = sender.as_ref() {
        header.append(&resizable_header_cell(
            ObjectTableColumn::Name,
            "Name",
            name_width,
            0,
            sender.clone(),
            list.clone(),
        ));
    } else {
        header.append(&grid_label("Name", Some(name_width), false));
    }
    for (index, (column, width)) in columns
        .iter()
        .copied()
        .zip(widths.iter().copied())
        .enumerate()
    {
        if let Some(sender) = sender.as_ref() {
            header.append(&resizable_header_cell(
                ObjectTableColumn::Data(column),
                column.label(),
                width,
                index + 1,
                sender.clone(),
                list.clone(),
            ));
        } else {
            header.append(&grid_label(column.label(), Some(width), false));
        }
    }

    header
}

pub(super) fn object_header_row_with_column_widths(
    name_width: i32,
    columns: &[ObjectColumn],
    widths: &[i32],
    sender: Option<ComponentSender<App>>,
    list: Option<gtk::ListBox>,
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(false);
    row.set_selectable(false);

    let header = object_header_with_column_widths(name_width, columns, widths, sender, list);
    header.set_margin_all(8);
    row.set_child(Some(&header));
    row
}

pub(super) fn object_row(object: &ObjectSummary) -> gtk::ListBoxRow {
    let widths = default_column_widths(&RELATED_POD_COLUMNS);
    object_row_with_column_widths(object, OBJECT_NAME_WIDTH, &RELATED_POD_COLUMNS, &widths)
}

pub(super) fn object_row_with_column_widths(
    object: &ObjectSummary,
    name_width: i32,
    columns: &[ObjectColumn],
    widths: &[i32],
) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_activatable(true);
    let container = object_row_box();
    container.set_margin_all(8);

    container.append(&object_name_cell(object, name_width));
    for (column, width) in columns.iter().copied().zip(widths.iter().copied()) {
        let label = match column {
            ObjectColumn::Namespace => grid_label(&object.namespace, Some(width), false),
            ObjectColumn::Status => match object.status_ratio {
                Some((ready, desired)) => {
                    let label = grid_label(&format!("{ready}/{desired}"), Some(width), false);
                    label.set_tooltip_text(Some(&object.status));
                    label
                }
                None => grid_label("", Some(width), false),
            },
            ObjectColumn::Cpu => metric_label_with_width(
                object.metrics.as_ref().map(|usage| usage.cpu.as_str()),
                width,
            ),
            ObjectColumn::Memory => metric_label_with_width(
                object.metrics.as_ref().map(|usage| usage.memory.as_str()),
                width,
            ),
            ObjectColumn::Api => grid_label(&object.api_version, Some(width), false),
            ObjectColumn::Age => grid_label(&object.age, Some(width), false),
        };
        container.append(&label);
    }

    row.set_child(Some(&container));
    row
}

fn default_column_widths(columns: &[ObjectColumn]) -> Vec<i32> {
    columns
        .iter()
        .copied()
        .map(ObjectColumn::default_width)
        .collect()
}

fn resizable_header_cell(
    column: ObjectTableColumn,
    title: &str,
    width: i32,
    cell_index: usize,
    sender: ComponentSender<App>,
    list: Option<gtk::ListBox>,
) -> gtk::Box {
    let cell = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    cell.set_size_request(width, -1);
    cell.set_hexpand(false);

    let label = grid_label(title, None, false);
    label.set_hexpand(true);
    cell.append(&label);

    let handle = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    handle.add_css_class("column-resize-handle");
    handle.set_size_request(10, -1);
    handle.set_cursor_from_name(Some("col-resize"));
    handle.set_tooltip_text(Some("Drag to resize column"));
    let line = gtk::Box::new(gtk::Orientation::Vertical, 0);
    line.add_css_class("column-resize-line");
    line.set_margin_start(4);
    line.set_margin_end(4);
    line.set_vexpand(true);
    handle.append(&line);
    cell.append(&handle);

    let start_width = std::rc::Rc::new(std::cell::Cell::new(width));
    let next_width = std::rc::Rc::new(std::cell::Cell::new(width));
    let pending_apply = std::rc::Rc::new(std::cell::Cell::new(false));

    // The gesture lives on `cell`, not `handle`: `handle` slides to the
    // right as `cell` grows (it's `label`'s hexpand that pushes it), so its
    // own on-screen origin moves as a *direct result* of the resize it's
    // driving. GTK translates pointer events into a controller's widget
    // using that widget's *current* allocation, so a drag attached to a
    // widget that keeps shifting because of the very drag it's producing
    // is a coordinate feedback loop — every relayout nudges the reference
    // frame out from under the still-held pointer, which is what showed up
    // as the handle visibly shaking while held. `cell`'s own origin is
    // stable during its own resize (only its far edge extends), so we
    // attach there and just restrict activation to presses that actually
    // land on the handle's rendered bounds.
    let gesture = gtk::GestureDrag::new();
    gesture.connect_drag_begin({
        let handle = handle.clone();
        let cell_for_hit_test = cell.clone();
        let start_width = start_width.clone();
        let next_width = next_width.clone();
        move |gesture, x, y| {
            let within_handle = handle
                .compute_bounds(&cell_for_hit_test)
                .is_some_and(|bounds| {
                    bounds.contains_point(&gtk::graphene::Point::new(x as f32, y as f32))
                });
            if !within_handle {
                gesture.set_state(gtk::EventSequenceState::Denied);
                return;
            }
            handle.add_css_class("column-resize-handle-active");
            start_width.set(next_width.get());
        }
    });
    gesture.connect_drag_update({
        let cell = cell.clone();
        let list = list.clone();
        let start_width = start_width.clone();
        let next_width = next_width.clone();
        let pending_apply = pending_apply.clone();
        move |_, offset_x, _| {
            let width =
                clamp_table_column_width(column, start_width.get() + offset_x.round() as i32);
            next_width.set(width);
            // Coalesce to at most one relayout per frame: a high-poll-rate
            // mouse fires drag-update far more often than the display
            // refreshes, and applying every single event synchronously
            // meant redundant relayouts within the same visual frame.
            if pending_apply.get() {
                return;
            }
            pending_apply.set(true);
            let cell = cell.clone();
            let list = list.clone();
            let next_width = next_width.clone();
            let pending_apply = pending_apply.clone();
            gtk::glib::idle_add_local_once(move || {
                pending_apply.set(false);
                let width = next_width.get();
                cell.set_size_request(width, -1);
                if let Some(list) = list.as_ref() {
                    apply_object_table_column_width(list, cell_index, width);
                }
            });
        }
    });
    gesture.connect_drag_end({
        let handle = handle.clone();
        let sender = sender.clone();
        let next_width = next_width.clone();
        move |_, _, _| {
            handle.remove_css_class("column-resize-handle-active");
            sender.input(AppMsg::ObjectColumnResized(column, next_width.get()));
        }
    });
    cell.add_controller(gesture);

    cell
}

fn clamp_table_column_width(column: ObjectTableColumn, width: i32) -> i32 {
    match column {
        ObjectTableColumn::Name => width.clamp(OBJECT_NAME_MIN_WIDTH, OBJECT_NAME_MAX_WIDTH),
        ObjectTableColumn::Data(_) => width.clamp(OBJECT_COLUMN_MIN_WIDTH, OBJECT_COLUMN_MAX_WIDTH),
    }
}

fn apply_object_table_column_width(list: &gtk::ListBox, cell_index: usize, width: i32) {
    // A resource list can hold thousands of rows; touching every one of
    // them on every live-drag frame is real, visible per-frame cost. Rows
    // scrolled out of view don't need to track the drag in real time —
    // they'll pick up the final width from the full rebuild that already
    // runs once the drag ends — so only the rows actually on screen get
    // updated live, which keeps each frame's work proportional to the
    // viewport instead of the whole list.
    let visible_range = visible_row_range(list);

    let mut row_widget = list.first_child();
    while let Some(widget) = row_widget {
        row_widget = widget.next_sibling();
        let Ok(row) = widget.downcast::<gtk::ListBoxRow>() else {
            continue;
        };
        if let Some((top, bottom)) = visible_range {
            let Some(bounds) = row.compute_bounds(list) else {
                continue;
            };
            let row_top = f64::from(bounds.y());
            let row_bottom = row_top + f64::from(bounds.height());
            if row_bottom < top || row_top > bottom {
                continue;
            }
        }
        let Some(container) = row
            .child()
            .and_then(|child| child.downcast::<gtk::Box>().ok())
        else {
            continue;
        };
        let Some(cell) = box_child_at(&container, cell_index) else {
            continue;
        };
        cell.set_size_request(width, -1);
        if cell_index == 0 {
            resize_live_name_label(&cell, width);
        }
    }
}

/// The visible viewport, in `list`'s own (unscrolled-content) coordinate
/// space, from the enclosing `GtkScrolledWindow`'s vertical adjustment.
/// `None` if `list` isn't inside one (or isn't realized yet), in which case
/// callers should just update every row.
fn visible_row_range(list: &gtk::ListBox) -> Option<(f64, f64)> {
    let scrolled = list
        .ancestor(gtk::ScrolledWindow::static_type())?
        .downcast::<gtk::ScrolledWindow>()
        .ok()?;
    let adjustment = scrolled.vadjustment();
    let top = adjustment.value();
    Some((top, top + adjustment.page_size()))
}

/// The Name cell (index 0) wraps an optional status chip plus the actual
/// name label; the label — not the wrapping cell — is what carries
/// ellipsize/max-width-chars, so it needs its own updated budget too, or it
/// keeps ellipsizing against its stale construction-time width while the
/// cell around it resizes live.
fn resize_live_name_label(cell: &gtk::Widget, width: i32) {
    let Some(name_cell) = cell.downcast_ref::<gtk::Box>() else {
        return;
    };
    let Some(label) = name_cell
        .last_child()
        .and_then(|child| child.downcast::<gtk::Label>().ok())
    else {
        return;
    };
    let has_chip = name_cell.first_child() != name_cell.last_child();
    let chip_budget = if has_chip { NAME_CELL_CHIP_BUDGET } else { 0 };
    set_grid_label_pixel_width(&label, (width - chip_budget).max(40));
}

fn box_child_at(container: &gtk::Box, index: usize) -> Option<gtk::Widget> {
    let mut position = 0;
    let mut child = container.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        if position == index {
            return Some(widget);
        }
        position += 1;
    }
    None
}

pub(super) fn object_row_box() -> gtk::Box {
    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row_box.set_hexpand(true);
    row_box.set_halign(gtk::Align::Start);
    row_box
}

/// The status chip's own CSS padding + text + the cell's own spacing,
/// budgeted out of the Name column's width so the label's pixel budget
/// (computed both at construction and during a live column-resize drag)
/// stays in sync in both places.
const NAME_CELL_CHIP_BUDGET: i32 = 86;

pub(super) fn object_name_cell(object: &ObjectSummary, width: i32) -> gtk::Box {
    let cell = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    cell.set_hexpand(false);
    cell.set_valign(gtk::Align::Center);
    cell.set_size_request(width, -1);

    let has_status = has_meaningful_status(&object.status);
    if has_status {
        cell.append(&status_prefix_chip(&object.status));
    }

    // `max-width-chars` alone is only a soft sizing hint to Pango — with
    // nothing else in this row claiming `hexpand`, a label with no pixel
    // `size_request` renders at its full natural (untruncated) width
    // whenever the row has any slack at all, which is why some long names
    // rendered in full while others ellipsized: it depended on whether that
    // particular row's total width happened to exceed the window, not on
    // this column's intended budget. Routing through `grid_label`'s pixel
    // `size_request` (like every other column already does) is what
    // actually forces ellipsis to engage reliably, regardless of slack.
    // Covers the chip's own CSS padding, its text, and the cell's spacing,
    // so the two together still fit inside `width`.
    let chip_budget = if has_status { NAME_CELL_CHIP_BUDGET } else { 0 };
    let name = grid_label(&object.name, Some((width - chip_budget).max(40)), false);
    name.add_css_class("heading");
    name.set_tooltip_text(Some(&object.name));
    cell.append(&name);
    cell
}

/// Whether a status string is actual information rather than the "no
/// status data" placeholder (e.g. ControllerRevision, which has no status
/// subresource at all).
pub(super) fn has_meaningful_status(status: &str) -> bool {
    !status.is_empty() && status != "-"
}

pub(super) fn status_prefix_chip(status: &str) -> gtk::Label {
    let primary = status
        .split_whitespace()
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("Unknown");
    let chip = status_chip(primary, status_tone(primary));
    chip.set_tooltip_text(Some(status));
    // ".status-chip" CSS adds 16px of horizontal padding on top of this;
    // budgeted into OBJECT_NAME_WIDTH's split with object_name_cell's name
    // label so the two together can't outgrow the column.
    chip.set_max_width_chars(6);
    chip
}

/// `value` is `None` when the object has no metrics at all (metrics.k8s.io
/// unavailable, or this resource kind isn't covered by it) — leave the
/// cell blank rather than clutter the row with a dash.
fn metric_label_with_width(value: Option<&str>, width: i32) -> gtk::Label {
    let label = grid_label(value.unwrap_or(""), Some(width), false);
    label.add_css_class("caption");
    label
}

pub(super) fn metric_badge(icon_name: &str, label: &str, value: &str, raw_value: &str) -> gtk::Box {
    let badge = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    badge.set_valign(gtk::Align::Center);
    badge.set_tooltip_text(Some(&format!("{label}: {raw_value}")));
    let icon = gtk::Image::from_icon_name(available_icon_name(
        icon_name,
        "applications-system-symbolic",
    ));
    icon.add_css_class("dim-label");
    icon.set_tooltip_text(Some(label));
    badge.append(&icon);
    let label = gtk::Label::new(Some(value));
    label.add_css_class("caption");
    label.set_tooltip_text(Some(raw_value));
    badge.append(&label);
    badge
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

/// Shared by `grid_label` (at construction) and the live column-resize
/// drag (updating an existing label in place) so the two never drift out
/// of sync with each other.
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
        .tooltip_text("Find previous match (Shift+Enter)")
        .build();
    let next_button = gtk::Button::builder()
        .icon_name("go-down-symbolic")
        .tooltip_text("Find next match (Enter)")
        .build();
    let status_label = gtk::Label::new(None);
    status_label.add_css_class("dim-label");
    status_label.add_css_class("caption");
    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text("Close search (Escape)")
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
            None => status_label.set_label("No matches"),
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
