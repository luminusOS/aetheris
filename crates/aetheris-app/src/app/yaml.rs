use super::utils::text_buffer_text;
use super::*;

pub(super) fn ensure_text_tag(
    buffer: &gtk::TextBuffer,
    name: &str,
    properties: &[(&str, &dyn ToValue)],
) {
    if buffer.tag_table().lookup(name).is_none() {
        let _ = buffer.create_tag(Some(name), properties);
    }
}

pub(super) fn build_yaml_view(buffer: &sourceview5::Buffer) -> sourceview5::View {
    let view = sourceview5::View::with_buffer(buffer);
    view.set_show_line_numbers(true);
    view.set_highlight_current_line(true);
    view.set_tab_width(2);
    view.set_monospace(true);
    view.set_wrap_mode(gtk::WrapMode::None);
    view
}

/// A Ctrl+F search bar for a YAML editor: highlights every match in the
/// buffer and steps through them. Returns a `Revealer` to place above (or
/// below) the editor's `ScrolledWindow`; wires Ctrl+F on `view` to reveal
/// it and Escape to hide it again.
pub(super) fn build_yaml_search_bar(
    view: &sourceview5::View,
    buffer: &sourceview5::Buffer,
) -> gtk::Revealer {
    let settings = sourceview5::SearchSettings::new();
    settings.set_wrap_around(true);
    let search_context = sourceview5::SearchContext::new(buffer, Some(&settings));
    search_context.set_highlight(true);

    let entry = gtk::SearchEntry::builder().hexpand(true).build();
    let prev_button = gtk::Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text("Find previous match (Shift+Enter)")
        .build();
    let next_button = gtk::Button::builder()
        .icon_name("go-down-symbolic")
        .tooltip_text("Find next match (Enter)")
        .build();
    let count_label = gtk::Label::new(None);
    count_label.add_css_class("dim-label");
    count_label.add_css_class("caption");
    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text("Close search (Escape)")
        .build();

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bar.set_margin_all(6);
    bar.append(&entry);
    bar.append(&count_label);
    bar.append(&prev_button);
    bar.append(&next_button);
    bar.append(&close_button);

    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .reveal_child(false)
        .build();
    revealer.set_child(Some(&bar));

    search_context.connect_occurrences_count_notify({
        let count_label = count_label.clone();
        let entry = entry.clone();
        move |search_context| {
            let count = search_context.occurrences_count();
            count_label.set_label(&match (entry.text().is_empty(), count) {
                (true, _) => String::new(),
                (false, 0) => String::from("No matches"),
                (false, count) if count < 0 => String::new(),
                (false, count) => format!("{count} matches"),
            });
        }
    });

    entry.connect_search_changed({
        let settings = settings.clone();
        move |entry| settings.set_search_text(Some(&entry.text()))
    });

    let find_next = {
        let buffer = buffer.clone();
        let search_context = search_context.clone();
        let view = view.clone();
        move || {
            let text_buffer: &gtk::TextBuffer = buffer.upcast_ref();
            let cursor = text_buffer.iter_at_mark(&text_buffer.get_insert());
            if let Some((start, end, _wrapped)) = search_context.forward(&cursor) {
                text_buffer.select_range(&start, &end);
                view.scroll_to_iter(&mut start.clone(), 0.1, false, 0.0, 0.0);
            }
        }
    };
    let find_previous = {
        let buffer = buffer.clone();
        let search_context = search_context.clone();
        let view = view.clone();
        move || {
            let text_buffer: &gtk::TextBuffer = buffer.upcast_ref();
            let cursor = text_buffer.iter_at_mark(&text_buffer.get_insert());
            if let Some((start, end, _wrapped)) = search_context.backward(&cursor) {
                text_buffer.select_range(&start, &end);
                view.scroll_to_iter(&mut start.clone(), 0.1, false, 0.0, 0.0);
            }
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

/// Wires up YAML syntax highlighting (via GtkSourceView's own lexer, not
/// hand-rolled) and live structural error checking: the first bad line
/// gets a red background and `error_label` shows the parser's message.
pub(super) fn setup_yaml_buffer(buffer: &sourceview5::Buffer, error_label: &gtk::Label) {
    if let Some(language) = sourceview5::LanguageManager::default().language("yaml") {
        buffer.set_language(Some(&language));
    }
    let scheme_id = if adw::StyleManager::default().is_dark() {
        "Adwaita-dark"
    } else {
        "Adwaita"
    };
    if let Some(scheme) = sourceview5::StyleSchemeManager::default().scheme(scheme_id) {
        buffer.set_style_scheme(Some(&scheme));
    }

    ensure_text_tag(
        buffer.upcast_ref(),
        "yaml-error-line",
        &[("background", &"rgba(224, 27, 36, 0.18)")],
    );

    buffer.connect_changed({
        let error_label = error_label.clone();
        move |buffer| update_yaml_error_state(buffer, &error_label)
    });
    update_yaml_error_state(buffer, error_label);
}

fn update_yaml_error_state(buffer: &sourceview5::Buffer, error_label: &gtk::Label) {
    let text_buffer: &gtk::TextBuffer = buffer.upcast_ref();
    if let Some(tag) = text_buffer.tag_table().lookup("yaml-error-line") {
        text_buffer.remove_tag(&tag, &text_buffer.start_iter(), &text_buffer.end_iter());
    }

    // Kept permanently visible (just blank when there's nothing to report)
    // rather than toggling `set_visible`: hiding it removed its hexpand
    // from the buttons row's layout, so the buttons jumped position
    // depending on whether an error happened to be showing.
    let Some((line, message)) = yaml_parse_error(&text_buffer_text(text_buffer)) else {
        error_label.set_label("");
        return;
    };

    if let Some(tag) = text_buffer.tag_table().lookup("yaml-error-line") {
        if let Some(start) = text_buffer.iter_at_line((line - 1) as i32) {
            let mut end = start;
            end.forward_to_line_end();
            text_buffer.apply_tag(&tag, &start, &end);
        }
    }
    error_label.set_label(&format!("Line {line}: {message}"));
}

pub(super) fn yaml_parse_error(text: &str) -> Option<(usize, String)> {
    if text.trim().is_empty() {
        return None;
    }
    let error = serde_yaml::from_str::<serde_yaml::Value>(text).err()?;
    let line = error.location().map_or(1, |location| location.line());
    Some((line, error.to_string()))
}

pub(super) fn build_yaml_explanation_content(
    yaml: &str,
    target: Option<&DetailTarget>,
) -> gtk::Widget {
    let parsed = match serde_yaml::from_str::<serde_yaml::Value>(yaml) {
        Ok(parsed) => parsed,
        Err(error) => {
            return yaml_error_page("Unable to parse this YAML", &format!("{error}")).upcast();
        }
    };
    let Some(mapping) = parsed.as_mapping() else {
        return yaml_error_page(
            "This is not a Kubernetes object",
            "The document is valid YAML, but Kubernetes manifests must be maps with fields such as apiVersion, kind and metadata.",
        )
        .upcast();
    };

    let api_version = yaml_string_field(mapping, "apiVersion").unwrap_or("-");
    let kind = yaml_string_field(mapping, "kind")
        .or_else(|| target.map(|target| target.resource.kind.as_str()))
        .unwrap_or("-");
    let metadata = yaml_mapping_field(mapping, "metadata");
    let name = metadata
        .and_then(|metadata| yaml_string_field(metadata, "name"))
        .unwrap_or("-");
    let namespace = metadata
        .and_then(|metadata| yaml_string_field(metadata, "namespace"))
        .or_else(|| target.and_then(|target| target.namespace.as_deref()))
        .unwrap_or("-");
    let top_keys = mapping
        .keys()
        .filter_map(serde_yaml::Value::as_str)
        .collect::<Vec<_>>();

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 18);
    outer.set_margin_top(18);
    outer.set_margin_bottom(18);
    outer.set_margin_start(18);
    outer.set_margin_end(18);

    let clamp = adw::Clamp::builder()
        .maximum_size(640)
        .tightening_threshold(520)
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);

    content.append(&yaml_explanation_header(kind, name, namespace));

    let summary = adw::PreferencesGroup::builder().title("Summary").build();
    summary.add(&yaml_row(
        "Resource",
        kind,
        kind_explanation(kind),
        "kubernetes resource type",
        "package-x-generic-symbolic",
    ));
    summary.add(&yaml_row(
        "API Version",
        api_version,
        "Identifies the Kubernetes API group and version that owns this object.",
        "apiVersion",
        "network-server-symbolic",
    ));
    summary.add(&yaml_row(
        "Name",
        name,
        "Unique object name inside its scope.",
        "metadata.name",
        "document-properties-symbolic",
    ));
    summary.add(&yaml_row(
        "Namespace",
        namespace,
        namespace_explanation(namespace),
        "metadata.namespace",
        "folder-symbolic",
    ));
    content.append(&summary);

    let lifecycle = adw::PreferencesGroup::builder()
        .title("How Kubernetes Uses It")
        .build();
    lifecycle.add(&yaml_row(
        "Desired State",
        if yaml_has_key(mapping, "spec") {
            "Declared"
        } else {
            "Not declared"
        },
        desired_state_explanation(kind, mapping),
        "spec",
        "document-edit-symbolic",
    ));
    lifecycle.add(&yaml_row(
        "Live State",
        if yaml_has_key(mapping, "status") {
            "Present"
        } else {
            "Managed by cluster"
        },
        "Status is written by Kubernetes controllers and usually should not be edited manually.",
        "status",
        "view-refresh-symbolic",
    ));
    content.append(&lifecycle);

    if let Some(metadata) = metadata {
        let metadata_group = adw::PreferencesGroup::builder()
            .title("Metadata")
            .description(
                "Labels and annotations are commonly used for selection, automation and ownership.",
            )
            .build();
        let mut has_metadata_rows = false;
        if let Some(labels) = yaml_mapping_field(metadata, "labels") {
            metadata_group.add(&yaml_row(
                "Labels",
                &format!("{} labels", labels.len()),
                "Small key/value pairs used by selectors and grouping.",
                "metadata.labels",
                "emblem-system-symbolic",
            ));
            has_metadata_rows = true;
        }
        if let Some(annotations) = yaml_mapping_field(metadata, "annotations") {
            metadata_group.add(&yaml_row(
                "Annotations",
                &format!("{} annotations", annotations.len()),
                "Free-form metadata used by tools, controllers and integrations.",
                "metadata.annotations",
                "document-properties-symbolic",
            ));
            has_metadata_rows = true;
        }
        if has_metadata_rows {
            content.append(&metadata_group);
        }
    }

    if let Some(spec) = yaml_mapping_field(mapping, "spec") {
        let spec_group = adw::PreferencesGroup::builder()
            .title("Spec")
            .description(spec_explanation(kind))
            .build();
        for (title, value, description, path, icon) in spec_rows(kind, spec) {
            spec_group.add(&yaml_row(&title, &value, &description, &path, icon));
        }
        content.append(&spec_group);
    }

    let data_group = adw::PreferencesGroup::builder()
        .title("Data")
        .description("Payload fields used by configuration-oriented resources.")
        .build();
    let mut has_data = false;
    if let Some(data) = yaml_mapping_field(mapping, "data") {
        data_group.add(&yaml_row(
            "data",
            &format!("{} entries", data.len()),
            "Key/value payload. In Secrets these values are base64 encoded.",
            "data",
            "document-open-symbolic",
        ));
        has_data = true;
    }
    if let Some(string_data) = yaml_mapping_field(mapping, "stringData") {
        data_group.add(&yaml_row(
            "stringData",
            &format!("{} entries", string_data.len()),
            "Plain string Secret input converted by Kubernetes into data.",
            "stringData",
            "document-edit-symbolic",
        ));
        has_data = true;
    }
    if has_data {
        content.append(&data_group);
    }

    let other_fields = top_keys
        .into_iter()
        .filter(|key| {
            !matches!(
                *key,
                "apiVersion" | "kind" | "metadata" | "spec" | "status" | "data" | "stringData"
            )
        })
        .collect::<Vec<_>>();
    if !other_fields.is_empty() {
        let other_group = adw::PreferencesGroup::builder()
            .title("Other Fields")
            .description("These fields are interpreted according to the resource schema.")
            .build();
        for field in other_fields {
            other_group.add(&yaml_row(
                field,
                "Present",
                "Additional top-level field in this manifest.",
                field,
                "dialog-information-symbolic",
            ));
        }
        content.append(&other_group);
    }

    clamp.set_child(Some(&content));
    outer.append(&clamp);
    outer.upcast()
}

pub(super) fn yaml_error_page(title: &str, description: &str) -> adw::StatusPage {
    adw::StatusPage::builder()
        .icon_name("dialog-warning-symbolic")
        .title(title)
        .description(description)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .build()
}

pub(super) fn yaml_explanation_header(kind: &str, name: &str, namespace: &str) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    header.set_valign(gtk::Align::Start);

    let icon = gtk::Image::from_icon_name("text-x-generic-symbolic");
    icon.set_pixel_size(42);
    icon.add_css_class("dim-label");
    header.append(&icon);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let title = gtk::Label::builder()
        .label(format!("{kind} Manifest"))
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["title-2"])
        .build();
    let subtitle = gtk::Label::builder()
        .label(if namespace == "-" {
            format!("{} at cluster scope", display_yaml_value(name))
        } else {
            format!("{} in namespace {namespace}", display_yaml_value(name))
        })
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label"])
        .build();
    text.append(&title);
    text.append(&subtitle);
    header.append(&text);
    header
}

pub(super) fn yaml_row(
    title: &str,
    value: &str,
    description: &str,
    path: &str,
    icon_name: &str,
) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(description)
        .build();
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.add_css_class("dim-label");
    row.add_prefix(&icon);

    let suffix = gtk::Box::new(gtk::Orientation::Vertical, 2);
    suffix.set_halign(gtk::Align::End);
    suffix.set_valign(gtk::Align::Center);
    let value_label = gtk::Label::builder()
        .label(display_yaml_value(value))
        .xalign(1.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .max_width_chars(22)
        .css_classes(["heading"])
        .build();
    let path_label = gtk::Label::builder()
        .label(path)
        .xalign(1.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .max_width_chars(22)
        .css_classes(["caption", "dim-label"])
        .build();
    suffix.append(&value_label);
    suffix.append(&path_label);
    row.add_suffix(&suffix);
    row
}

pub(super) fn display_yaml_value(value: &str) -> &str {
    if value.is_empty() || value == "-" {
        "Not set"
    } else {
        value
    }
}

pub(super) fn namespace_explanation(namespace: &str) -> &'static str {
    if namespace == "-" {
        "This resource is cluster-scoped and does not belong to a namespace."
    } else {
        "Namespace that scopes this resource and separates it from other environments."
    }
}

pub(super) fn desired_state_explanation(kind: &str, mapping: &serde_yaml::Mapping) -> &'static str {
    if !yaml_has_key(mapping, "spec") {
        return match kind {
            "ConfigMap" => "ConfigMaps usually express desired configuration through data instead of spec.",
            "Secret" => "Secrets usually express desired sensitive data through data or stringData instead of spec.",
            _ => "This manifest does not include a spec section.",
        };
    }

    spec_explanation(kind)
}

pub(super) fn spec_rows(
    kind: &str,
    spec: &serde_yaml::Mapping,
) -> Vec<(String, String, String, String, &'static str)> {
    let mut rows = Vec::new();
    match kind {
        "Deployment" => {
            push_yaml_field(
                &mut rows,
                spec,
                "replicas",
                "Replicas",
                "Number of Pod copies the Deployment should keep running.",
                "spec.replicas",
                "view-grid-symbolic",
            );
            if yaml_mapping_field(spec, "selector").is_some() {
                rows.push((
                    String::from("Selector"),
                    String::from("Present"),
                    String::from("Matches the Pods managed by this Deployment."),
                    String::from("spec.selector"),
                    "edit-find-symbolic",
                ));
            }
            if yaml_mapping_field(spec, "template").is_some() {
                rows.push((
                    String::from("Pod Template"),
                    String::from("Present"),
                    String::from("Template used to create new Pods."),
                    String::from("spec.template"),
                    "document-new-symbolic",
                ));
            }
        }
        "Pod" => {
            if let Some(containers) = yaml_sequence_field(spec, "containers") {
                rows.push((
                    String::from("Containers"),
                    format!("{} containers", containers.len()),
                    String::from("Application containers started together in this Pod."),
                    String::from("spec.containers"),
                    "utilities-terminal-symbolic",
                ));
            }
            push_yaml_field(
                &mut rows,
                spec,
                "restartPolicy",
                "Restart Policy",
                "Controls how Kubernetes restarts containers after exit.",
                "spec.restartPolicy",
                "view-refresh-symbolic",
            );
            push_yaml_field(
                &mut rows,
                spec,
                "nodeName",
                "Node",
                "Node currently targeted by this Pod.",
                "spec.nodeName",
                "computer-symbolic",
            );
        }
        "Service" => {
            push_yaml_field(
                &mut rows,
                spec,
                "type",
                "Type",
                "Controls how the Service is exposed.",
                "spec.type",
                "network-wired-symbolic",
            );
            if yaml_mapping_field(spec, "selector").is_some() {
                rows.push((
                    String::from("Selector"),
                    String::from("Present"),
                    String::from("Selects Pods that receive traffic from this Service."),
                    String::from("spec.selector"),
                    "edit-find-symbolic",
                ));
            }
            if let Some(ports) = yaml_sequence_field(spec, "ports") {
                rows.push((
                    String::from("Ports"),
                    format!("{} ports", ports.len()),
                    String::from("Network ports exposed by this Service."),
                    String::from("spec.ports"),
                    "network-transmit-receive-symbolic",
                ));
            }
        }
        "Ingress" => {
            if let Some(rules) = yaml_sequence_field(spec, "rules") {
                rows.push((
                    String::from("Rules"),
                    format!("{} rules", rules.len()),
                    String::from("HTTP routing rules handled by this Ingress."),
                    String::from("spec.rules"),
                    "network-server-symbolic",
                ));
            }
            if let Some(tls) = yaml_sequence_field(spec, "tls") {
                rows.push((
                    String::from("TLS"),
                    format!("{} entries", tls.len()),
                    String::from("TLS certificate routing configuration."),
                    String::from("spec.tls"),
                    "changes-allow-symbolic",
                ));
            }
        }
        "Job" | "CronJob" => {
            if yaml_mapping_field(spec, "jobTemplate").is_some() {
                rows.push((
                    String::from("Job Template"),
                    String::from("Present"),
                    String::from("Template used to create scheduled Jobs."),
                    String::from("spec.jobTemplate"),
                    "document-new-symbolic",
                ));
            }
            push_yaml_field(
                &mut rows,
                spec,
                "schedule",
                "Schedule",
                "Cron schedule used to create Jobs.",
                "spec.schedule",
                "appointment-new-symbolic",
            );
        }
        _ => {}
    }

    for key in spec.keys().filter_map(serde_yaml::Value::as_str) {
        if rows
            .iter()
            .any(|(_, _, _, path, _)| path == &format!("spec.{key}"))
        {
            continue;
        }
        rows.push((
            key.to_owned(),
            yaml_value_summary(spec.get(serde_yaml::Value::String(key.to_owned()))),
            String::from("Field declared in the desired state for this resource."),
            format!("spec.{key}"),
            "dialog-information-symbolic",
        ));
    }

    if rows.is_empty() {
        rows.push((
            String::from("Spec"),
            String::from("Empty"),
            String::from("No desired-state fields are declared in this spec."),
            String::from("spec"),
            "dialog-information-symbolic",
        ));
    }

    rows
}

pub(super) fn push_yaml_field(
    rows: &mut Vec<(String, String, String, String, &'static str)>,
    mapping: &serde_yaml::Mapping,
    key: &str,
    title: &str,
    description: &str,
    path: &str,
    icon_name: &'static str,
) {
    if let Some(value) = mapping.get(serde_yaml::Value::String(key.to_owned())) {
        rows.push((
            title.to_owned(),
            yaml_value_summary(Some(value)),
            description.to_owned(),
            path.to_owned(),
            icon_name,
        ));
    }
}

pub(super) fn yaml_value_summary(value: Option<&serde_yaml::Value>) -> String {
    match value {
        Some(serde_yaml::Value::Null) | None => String::from("Not set"),
        Some(serde_yaml::Value::Bool(value)) => value.to_string(),
        Some(serde_yaml::Value::Number(value)) => value.to_string(),
        Some(serde_yaml::Value::String(value)) => value.clone(),
        Some(serde_yaml::Value::Sequence(values)) => format!("{} items", values.len()),
        Some(serde_yaml::Value::Mapping(values)) => format!("{} fields", values.len()),
        Some(serde_yaml::Value::Tagged(_)) => String::from("Tagged value"),
    }
}

pub(super) fn yaml_sequence_field<'a>(
    mapping: &'a serde_yaml::Mapping,
    key: &str,
) -> Option<&'a Vec<serde_yaml::Value>> {
    mapping
        .get(serde_yaml::Value::String(key.to_owned()))
        .and_then(serde_yaml::Value::as_sequence)
}

pub(super) fn yaml_has_key(mapping: &serde_yaml::Mapping, key: &str) -> bool {
    mapping
        .get(serde_yaml::Value::String(key.to_owned()))
        .is_some()
}

pub(super) fn yaml_mapping_field<'a>(
    mapping: &'a serde_yaml::Mapping,
    key: &str,
) -> Option<&'a serde_yaml::Mapping> {
    mapping
        .get(serde_yaml::Value::String(key.to_owned()))
        .and_then(serde_yaml::Value::as_mapping)
}

pub(super) fn yaml_string_field<'a>(
    mapping: &'a serde_yaml::Mapping,
    key: &str,
) -> Option<&'a str> {
    mapping
        .get(serde_yaml::Value::String(key.to_owned()))
        .and_then(serde_yaml::Value::as_str)
}

pub(super) fn kind_explanation(kind: &str) -> &'static str {
    match kind {
        "Pod" => "Runs one or more containers together on a node.",
        "Deployment" => "Manages ReplicaSets and keeps the desired number of Pods running.",
        "Service" => "Gives a stable virtual address and discovery name to a set of Pods.",
        "Ingress" => "Routes external HTTP or HTTPS traffic to Services.",
        "ConfigMap" => "Stores non-secret configuration consumed by Pods.",
        "Secret" => "Stores sensitive configuration. Values under data are base64 encoded.",
        "Job" => "Runs Pods until a task completes successfully.",
        "CronJob" => "Creates Jobs on a schedule.",
        "Namespace" => "Partitions namespaced resources inside a cluster.",
        "Node" => "Represents a worker machine registered in the cluster.",
        _ => "Declares the Kubernetes resource type for this manifest.",
    }
}

pub(super) fn spec_explanation(kind: &str) -> &'static str {
    match kind {
        "Pod" => "Desired Pod configuration, including containers, volumes, restart policy and scheduling hints.",
        "Deployment" => "Desired Deployment state, usually replicas, selector and the Pod template.",
        "Service" => "Desired Service routing, including selector, type and exposed ports.",
        "Ingress" => "Desired routing rules, TLS configuration and backend Services.",
        "Job" => "Desired one-shot workload, including completion policy and Pod template.",
        "CronJob" => "Desired schedule and Job template.",
        "Node" => "Node spec is mostly managed by Kubernetes and should be edited carefully.",
        _ => "Desired state for this resource. Controllers reconcile the live object toward this section.",
    }
}

#[cfg(test)]
mod tests {
    use super::yaml_parse_error;

    #[test]
    fn yaml_parse_error_accepts_valid_yaml() {
        assert_eq!(yaml_parse_error("apiVersion: v1\nkind: Pod\n"), None);
    }

    #[test]
    fn yaml_parse_error_ignores_blank_text() {
        assert_eq!(yaml_parse_error(""), None);
        assert_eq!(yaml_parse_error("   \n  "), None);
    }

    #[test]
    fn yaml_parse_error_reports_the_offending_line() {
        let (line, message) = yaml_parse_error("apiVersion: v1\nkind: Pod\n  bad: [1, 2\n")
            .expect("malformed YAML should fail to parse");
        assert_eq!(line, 3);
        assert!(!message.is_empty());
    }
}
