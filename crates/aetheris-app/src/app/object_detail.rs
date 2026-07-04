use super::widgets::*;
use super::yaml::*;
use super::*;

pub(super) struct ObjectDetailWidgets<'a> {
    pub(super) stack: &'a gtk::Stack,
    pub(super) name: &'a gtk::Label,
    pub(super) namespace: &'a gtk::Label,
    pub(super) status: &'a gtk::Label,
    pub(super) kind: &'a gtk::Label,
    pub(super) api_version: &'a gtk::Label,
    pub(super) age: &'a gtk::Label,
    pub(super) cpu: &'a gtk::Label,
    pub(super) memory: &'a gtk::Label,
    pub(super) container_metrics_list: &'a gtk::ListBox,
    pub(super) scale_spin: &'a gtk::SpinButton,
    pub(super) scale_button: &'a gtk::Button,
    pub(super) cordon_button: &'a gtk::Button,
    pub(super) drain_button: &'a gtk::Button,
    pub(super) explain_yaml_button: &'a gtk::Button,
    pub(super) apply_button: &'a gtk::Button,
    pub(super) download_yaml_button: &'a gtk::Button,
    pub(super) yaml_buffer: &'a sourceview5::Buffer,
    pub(super) yaml_error_label: &'a gtk::Label,
    pub(super) events_list: &'a gtk::ListBox,
    pub(super) conditions_list: &'a gtk::ListBox,
    pub(super) related_pods_view: &'a gtk::ColumnView,
    pub(super) related_pods_stack: &'a gtk::Stack,
    pub(super) related_pods_message: &'a adw::StatusPage,
    pub(super) log_container_dropdown: &'a gtk::DropDown,
    pub(super) log_follow_check: &'a gtk::CheckButton,
    pub(super) log_timestamps_check: &'a gtk::CheckButton,
    pub(super) log_start_button: &'a gtk::Button,
    pub(super) log_stop_button: &'a gtk::Button,
    pub(super) log_clear_button: &'a gtk::Button,
    pub(super) log_download_button: &'a gtk::Button,
    pub(super) log_status_label: &'a gtk::Label,
    pub(super) log_view: &'a gtk::TextView,
    pub(super) port_local_spin: &'a gtk::SpinButton,
    pub(super) port_remote_spin: &'a gtk::SpinButton,
    pub(super) port_start_button: &'a gtk::Button,
    pub(super) port_stop_button: &'a gtk::Button,
    pub(super) port_status_label: &'a gtk::Label,
    pub(super) port_forward_group: &'a gtk::Box,
    pub(super) overview_section: &'a gtk::Box,
    pub(super) expand_logs_button: &'a gtk::Button,
}

pub(super) fn build_object_detail_page(widgets: ObjectDetailWidgets<'_>) -> gtk::Box {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_hexpand(true);
    outer.set_vexpand(true);

    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.set_hexpand(true);
    page.set_vexpand(true);
    page.set_margin_top(8);
    page.set_margin_bottom(8);
    page.set_margin_start(12);
    page.set_margin_end(12);

    let overview = widgets.overview_section;
    overview.append(&detail_overview_grid(&widgets));

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::Start);
    widgets.scale_spin.set_width_chars(5);
    actions.append(widgets.scale_spin);
    actions.append(widgets.scale_button);
    actions.append(widgets.cordon_button);
    actions.append(widgets.drain_button);
    overview.append(&actions);

    widgets
        .port_forward_group
        .append(&section_title("Port Forward"));
    let ports_controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    ports_controls.append(&field_box("Local", widgets.port_local_spin));
    ports_controls.append(&field_box("Remote", widgets.port_remote_spin));
    // field_box stacks a caption above each spin button, making this row
    // taller than the buttons' own natural height; without an explicit
    // valign, Box stretches them to fill that height instead of sitting at
    // their natural size level with the spin buttons.
    widgets.port_start_button.set_valign(gtk::Align::End);
    widgets.port_stop_button.set_valign(gtk::Align::End);
    ports_controls.append(widgets.port_start_button);
    ports_controls.append(widgets.port_stop_button);
    widgets.port_forward_group.append(&ports_controls);
    widgets.port_status_label.add_css_class("caption");
    widgets.port_status_label.add_css_class("dim-label");
    widgets.port_forward_group.append(widgets.port_status_label);
    overview.append(widgets.port_forward_group);

    page.append(overview);

    let switcher = gtk::StackSwitcher::new();
    switcher.set_stack(Some(widgets.stack));
    switcher.set_halign(gtk::Align::Center);
    page.append(&switcher);

    let yaml_view = build_yaml_view(widgets.yaml_buffer);
    yaml_view.set_editable(true);
    yaml_view.set_cursor_visible(true);
    let yaml_search_bar = build_yaml_search_bar(&yaml_view, widgets.yaml_buffer);

    let yaml_scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    yaml_scrolled.set_child(Some(&yaml_view));

    let yaml_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    yaml_page.set_margin_all(12);
    let yaml_controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    widgets.yaml_error_label.set_xalign(0.0);
    widgets.yaml_error_label.set_wrap(true);
    widgets.yaml_error_label.add_css_class("caption");
    widgets.yaml_error_label.add_css_class("error");
    widgets.yaml_error_label.set_hexpand(true);
    yaml_controls.append(widgets.yaml_error_label);
    yaml_controls.append(widgets.explain_yaml_button);
    yaml_controls.append(widgets.download_yaml_button);
    yaml_controls.append(widgets.apply_button);
    yaml_page.append(&yaml_controls);
    yaml_page.append(&yaml_search_bar);
    yaml_page.append(&yaml_scrolled);
    widgets.stack.add_titled(&yaml_page, Some("yaml"), "YAML");

    let events_page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    events_page.set_margin_all(12);
    let events_scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    events_scrolled.set_child(Some(widgets.events_list));
    events_page.append(&events_scrolled);
    widgets
        .stack
        .add_titled(&events_page, Some("events"), "Recent Events");

    let conditions_page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    conditions_page.set_margin_all(12);
    let conditions_scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    conditions_scrolled.set_child(Some(widgets.conditions_list));
    conditions_page.append(&conditions_scrolled);
    widgets
        .stack
        .add_titled(&conditions_page, Some("conditions"), "Conditions");

    let containers_page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    containers_page.set_margin_all(12);
    let containers_scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    containers_scrolled.set_child(Some(widgets.container_metrics_list));
    containers_page.append(&containers_scrolled);
    widgets
        .stack
        .add_titled(&containers_page, Some("containers"), "Containers");

    let pods_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    pods_page.set_margin_all(12);
    let pods_scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .css_classes(["aetheris-table-frame"])
        .build();
    // Direct child of the ScrolledWindow so the ColumnView virtualizes
    // (see the main object table in layout.rs).
    pods_scrolled.set_child(Some(widgets.related_pods_view));
    widgets
        .related_pods_stack
        .add_named(&pods_scrolled, Some("table"));
    widgets
        .related_pods_stack
        .add_named(widgets.related_pods_message, Some("message"));
    widgets.related_pods_stack.set_visible_child_name("message");
    widgets.related_pods_stack.set_vexpand(true);
    pods_page.append(widgets.related_pods_stack);
    widgets.stack.add_titled(&pods_page, Some("pods"), "Pods");

    let logs_page = gtk::Box::new(gtk::Orientation::Vertical, 10);
    logs_page.set_margin_all(12);

    let log_controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    widgets.log_container_dropdown.set_hexpand(true);
    log_controls.append(widgets.log_container_dropdown);
    log_controls.append(widgets.log_follow_check);
    log_controls.append(widgets.log_timestamps_check);
    log_controls.append(widgets.log_start_button);
    log_controls.append(widgets.log_stop_button);
    log_controls.append(widgets.log_clear_button);
    log_controls.append(widgets.log_download_button);
    log_controls.append(widgets.expand_logs_button);
    logs_page.append(&log_controls);
    logs_page.append(widgets.log_status_label);

    widgets.log_view.set_editable(false);
    widgets.log_view.set_cursor_visible(false);
    widgets.log_view.set_monospace(true);
    widgets.log_view.set_wrap_mode(gtk::WrapMode::None);
    let log_search_bar = build_log_search_bar(widgets.log_view, &widgets.log_view.buffer());
    logs_page.append(&log_search_bar);

    let logs_scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    logs_scrolled.set_child(Some(widgets.log_view));
    logs_page.append(&logs_scrolled);
    widgets.stack.add_titled(&logs_page, Some("logs"), "Logs");

    widgets.stack.set_visible_child_name("yaml");
    widgets.stack.set_vexpand(true);
    page.append(widgets.stack);

    // A tight clamp reads well for prose, but this page is mostly YAML,
    // logs, and tables — content that benefits from the extra width on a
    // maximized or fullscreen window rather than sitting narrow in the
    // middle of it.
    let clamp = adw::Clamp::builder()
        .maximum_size(1400)
        .tightening_threshold(1100)
        .build();
    clamp.add_css_class("content-clamp");
    clamp.set_vexpand(true);
    clamp.set_child(Some(&page));

    let center = gtk::CenterBox::new();
    center.set_hexpand(true);
    center.set_vexpand(true);
    center.set_center_widget(Some(&clamp));
    outer.append(&center);
    outer
}

pub(super) fn detail_overview_grid(widgets: &ObjectDetailWidgets<'_>) -> gtk::Grid {
    let grid = gtk::Grid::builder()
        .column_spacing(24)
        .row_spacing(10)
        .column_homogeneous(true)
        .hexpand(true)
        .build();

    // Paired so related facts sit side by side (Name/Status up top, then
    // Namespace/Age, etc.) instead of one long single-column list.
    let fields = [
        ("Name", widgets.name),
        ("Status", widgets.status),
        ("Namespace", widgets.namespace),
        ("Age", widgets.age),
        ("Kind", widgets.kind),
        ("CPU", widgets.cpu),
        ("API Version", widgets.api_version),
        ("Memory", widgets.memory),
    ];

    for (index, (label, value)) in fields.into_iter().enumerate() {
        let column = (index % 2) as i32;
        let row = (index / 2) as i32;
        grid.attach(&field_box(label, value), column, row, 1, 1);
    }

    grid
}

pub(super) fn set_stack_page(stack: &gtk::Stack, name: &str, visible: bool, title: &str) {
    let Some(child) = stack.child_by_name(name) else {
        return;
    };
    child.set_visible(visible);
    let page = stack.page(&child);
    page.set_visible(visible);
    page.set_title(title);
}

pub(super) fn field_box(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let title = gtk::Label::builder()
        .label(label)
        .xalign(0.0)
        .css_classes(["caption", "dim-label"])
        .build();
    container.append(&title);
    container.append(widget);
    container
}

pub(super) fn rebuild_detail_events(list: &gtk::ListBox, detail: &ObjectDetail) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if let Some(error) = &detail.events_error {
        list.append(&detail_message_row(
            "Unable to load events",
            error,
            "dialog-warning-symbolic",
        ));
        return;
    }

    if detail.events.is_empty() {
        list.append(&detail_message_row(
            "No events",
            "No events were found for this object.",
            "dialog-information-symbolic",
        ));
        return;
    }

    for event in &detail.events {
        list.append(&event_row(event));
    }
}

pub(super) fn rebuild_detail_conditions(list: &gtk::ListBox, detail: &ObjectDetail) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if detail.conditions.is_empty() {
        list.append(&detail_message_row(
            "No conditions",
            "This object does not expose status conditions.",
            "dialog-information-symbolic",
        ));
        return;
    }

    for condition in &detail.conditions {
        list.append(&condition_row(condition));
    }
}

pub(super) fn rebuild_related_pods(
    store: &gtk::gio::ListStore,
    stack: &gtk::Stack,
    message: &adw::StatusPage,
    detail: &ObjectDetail,
) {
    if detail.kind != "Deployment" {
        store.remove_all();
        message.set_title("Pods are shown for Deployments");
        message.set_description(Some("Open a Deployment to inspect its related Pods."));
        stack.set_visible_child_name("message");
        return;
    }

    if detail.related_pods.is_empty() {
        store.remove_all();
        message.set_title("No related Pods");
        message.set_description(Some("No Pods matched this Deployment selector."));
        stack.set_visible_child_name("message");
        return;
    }

    let items: Vec<gtk::glib::BoxedAnyObject> =
        detail.related_pods.iter().map(boxed_object).collect();
    store.splice(0, store.n_items(), &items);
    stack.set_visible_child_name("table");
}

pub(super) fn rebuild_container_metrics(list: &gtk::ListBox, detail: &ObjectDetail) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if detail.kind != "Pod" {
        list.set_visible(false);
        return;
    }

    list.set_visible(true);
    if detail.containers.is_empty() {
        list.append(&detail_message_row(
            "No containers",
            "This Pod does not expose containers in its spec.",
            "dialog-information-symbolic",
        ));
        return;
    }

    for container in &detail.containers {
        let usage = detail
            .container_metrics
            .iter()
            .find(|usage| usage.name == *container);
        list.append(&container_metric_row(container, usage));
    }
}

pub(super) fn container_metric_row(name: &str, usage: Option<&ContainerUsage>) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_selectable(false);
    let action = adw::ActionRow::builder().title(name).build();
    action.add_prefix(&status_prefix_chip("Running"));
    if let Some(usage) = usage {
        let metrics = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        metrics.set_valign(gtk::Align::Center);
        metrics.set_margin_start(12);
        metrics.append(&metric_badge(
            "applications-engineering-symbolic",
            "CPU usage",
            &format_cpu_quantity(&usage.cpu),
            &usage.cpu,
        ));
        metrics.append(&metric_badge(
            "drive-harddisk-symbolic",
            "Memory usage",
            &format_memory_quantity(&usage.memory),
            &usage.memory,
        ));
        action.add_suffix(&metrics);
    } else {
        action.set_subtitle("Metrics unavailable");
    }
    row.set_child(Some(&action));
    row
}

fn format_cpu_quantity(value: &str) -> String {
    let Some((amount, suffix)) = split_quantity(value) else {
        return value.to_owned();
    };
    let millicores = match suffix {
        "n" => amount / 1_000_000.0,
        "u" => amount / 1_000.0,
        "m" => amount,
        "" => amount * 1_000.0,
        _ => return value.to_owned(),
    };
    if millicores >= 1.0 {
        format!("{}m", millicores.round() as u64)
    } else {
        format!("{millicores:.1}m")
    }
}

fn format_memory_quantity(value: &str) -> String {
    let Some((amount, suffix)) = split_quantity(value) else {
        return value.to_owned();
    };
    let bytes = match suffix {
        "Ki" => amount * 1024.0,
        "Mi" => amount * 1024.0 * 1024.0,
        "Gi" => amount * 1024.0 * 1024.0 * 1024.0,
        "Ti" => amount * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "K" => amount * 1_000.0,
        "M" => amount * 1_000_000.0,
        "G" => amount * 1_000_000_000.0,
        "" => amount,
        _ => return value.to_owned(),
    };
    let gib = bytes / 1024.0 / 1024.0 / 1024.0;
    let mib = bytes / 1024.0 / 1024.0;
    if gib >= 1.0 {
        format!("{gib:.1} GiB")
    } else if mib >= 1.0 {
        format!("{} MiB", mib.round() as u64)
    } else {
        format!("{} KiB", (bytes / 1024.0).round() as u64)
    }
}

fn split_quantity(value: &str) -> Option<(f64, &str)> {
    let split_at = value
        .char_indices()
        .find(|(_, character)| !character.is_ascii_digit() && *character != '.')
        .map(|(index, _)| index)
        .unwrap_or(value.len());
    let amount = value.get(..split_at)?.parse::<f64>().ok()?;
    Some((amount, value.get(split_at..)?))
}

pub(super) fn condition_row(condition: &ObjectCondition) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_selectable(false);
    let subtitle = if condition.message == "-" {
        condition.reason.clone()
    } else if condition.reason == "-" {
        condition.message.clone()
    } else {
        format!("{} · {}", condition.reason, condition.message)
    };
    let action = adw::ActionRow::builder()
        .title(condition.type_.as_str())
        .subtitle(subtitle.as_str())
        .build();
    action.add_prefix(&gtk::Image::from_icon_name(condition_icon_name(condition)));
    action.add_suffix(&event_meta_label(&condition.status));
    action.add_suffix(&event_meta_label(&condition.last_transition));
    row.set_child(Some(&action));
    row
}

pub(super) fn condition_icon_name(condition: &ObjectCondition) -> &'static str {
    if condition.status.eq_ignore_ascii_case("false") {
        "dialog-warning-symbolic"
    } else {
        "dialog-information-symbolic"
    }
}

pub(super) fn detail_message_row(title: &str, subtitle: &str, icon_name: &str) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_selectable(false);
    let action = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();
    action.add_prefix(&gtk::Image::from_icon_name(icon_name));
    row.set_child(Some(&action));
    row
}

pub(super) fn event_row(event: &ObjectEvent) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_selectable(false);
    let action = adw::ActionRow::builder()
        .title(event.reason.as_str())
        .subtitle(event.message.as_str())
        .build();
    action.add_prefix(&gtk::Image::from_icon_name(event_icon_name(event)));
    action.add_suffix(&event_meta_label(&event.type_));
    action.add_suffix(&event_meta_label(&format!("{}x", event.count)));
    action.add_suffix(&event_meta_label(&event.last_seen));
    row.set_child(Some(&action));
    row
}

pub(super) fn event_icon_name(event: &ObjectEvent) -> &'static str {
    if event.type_.eq_ignore_ascii_case("warning") {
        "dialog-warning-symbolic"
    } else {
        "dialog-information-symbolic"
    }
}

pub(super) fn event_meta_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.add_css_class("caption");
    label.add_css_class("dim-label");
    label
}

pub(super) fn detail_value_label() -> gtk::Label {
    gtk::Label::builder()
        .xalign(0.0)
        .selectable(true)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .max_width_chars(28)
        .build()
}
