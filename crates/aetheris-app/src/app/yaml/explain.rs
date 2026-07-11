use super::super::*;

pub(crate) fn build_yaml_explanation_content(
    yaml: &str,
    target: Option<&DetailTarget>,
) -> gtk::Widget {
    let parsed = match serde_yaml::from_str::<serde_yaml::Value>(yaml) {
        Ok(parsed) => parsed,
        Err(error) => {
            return yaml_error_page(&tr("Unable to parse this YAML"), &format!("{error}")).upcast();
        }
    };
    let Some(mapping) = parsed.as_mapping() else {
        return yaml_error_page(
            &tr("This is not a Kubernetes object"),
            &tr("The document is valid YAML, but Kubernetes manifests must be maps with fields such as apiVersion, kind and metadata."),
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

    let summary = adw::PreferencesGroup::builder()
        .title(tr("Summary"))
        .build();
    summary.add(&yaml_row(
        &tr("Resource"),
        kind,
        &kind_explanation(kind),
        "kubernetes resource type",
        "package-x-generic-symbolic",
    ));
    summary.add(&yaml_row(
        &tr("API Version"),
        api_version,
        &tr("Identifies the Kubernetes API group and version that owns this object."),
        "apiVersion",
        "network-server-symbolic",
    ));
    summary.add(&yaml_row(
        &tr("Name"),
        name,
        &tr("Unique object name inside its scope."),
        "metadata.name",
        "document-properties-symbolic",
    ));
    summary.add(&yaml_row(
        &tr("Namespace"),
        namespace,
        &namespace_explanation(namespace),
        "metadata.namespace",
        "folder-symbolic",
    ));
    content.append(&summary);

    let lifecycle = adw::PreferencesGroup::builder()
        .title(tr("How Kubernetes Uses It"))
        .build();
    let desired_state = if yaml_has_key(mapping, "spec") {
        tr("Declared")
    } else {
        tr("Not declared")
    };
    lifecycle.add(&yaml_row(
        &tr("Desired State"),
        &desired_state,
        &desired_state_explanation(kind, mapping),
        "spec",
        "document-edit-symbolic",
    ));
    let live_state = if yaml_has_key(mapping, "status") {
        tr("Present")
    } else {
        tr("Managed by cluster")
    };
    lifecycle.add(&yaml_row(
        &tr("Live State"),
        &live_state,
        &tr("Status is written by Kubernetes controllers and usually should not be edited manually."),
        "status",
        "view-refresh-symbolic",
    ));
    content.append(&lifecycle);

    if let Some(metadata) = metadata {
        let metadata_group = adw::PreferencesGroup::builder()
            .title(tr("Metadata"))
            .description(tr(
                "Labels and annotations are commonly used for selection, automation and ownership.",
            ))
            .build();
        let mut has_metadata_rows = false;
        if let Some(labels) = yaml_mapping_field(metadata, "labels") {
            metadata_group.add(&yaml_row(
                &tr("Labels"),
                &format!(
                    "{} {}",
                    labels.len(),
                    trn("label", "labels", labels.len() as u32)
                ),
                &tr("Small key/value pairs used by selectors and grouping."),
                "metadata.labels",
                "emblem-system-symbolic",
            ));
            has_metadata_rows = true;
        }
        if let Some(annotations) = yaml_mapping_field(metadata, "annotations") {
            metadata_group.add(&yaml_row(
                &tr("Annotations"),
                &format!(
                    "{} {}",
                    annotations.len(),
                    trn("annotation", "annotations", annotations.len() as u32)
                ),
                &tr("Free-form metadata used by tools, controllers and integrations."),
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
            .title(tr("Spec"))
            .description(spec_explanation(kind))
            .build();
        for (title, value, description, path, icon) in spec_rows(kind, spec) {
            spec_group.add(&yaml_row(&title, &value, &description, &path, icon));
        }
        content.append(&spec_group);
    }

    let data_group = adw::PreferencesGroup::builder()
        .title(tr("Data"))
        .description(tr(
            "Payload fields used by configuration-oriented resources.",
        ))
        .build();
    let mut has_data = false;
    if let Some(data) = yaml_mapping_field(mapping, "data") {
        data_group.add(&yaml_row(
            "data",
            &format!(
                "{} {}",
                data.len(),
                trn("entry", "entries", data.len() as u32)
            ),
            &tr("Key/value payload. In Secrets these values are base64 encoded."),
            "data",
            "document-open-symbolic",
        ));
        has_data = true;
    }
    if let Some(string_data) = yaml_mapping_field(mapping, "stringData") {
        data_group.add(&yaml_row(
            "stringData",
            &format!(
                "{} {}",
                string_data.len(),
                trn("entry", "entries", string_data.len() as u32)
            ),
            &tr("Plain string Secret input converted by Kubernetes into data."),
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
            .title(tr("Other Fields"))
            .description(tr(
                "These fields are interpreted according to the resource schema.",
            ))
            .build();
        for field in other_fields {
            other_group.add(&yaml_row(
                field,
                &tr("Present"),
                &tr("Additional top-level field in this manifest."),
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

fn yaml_error_page(title: &str, description: &str) -> adw::StatusPage {
    adw::StatusPage::builder()
        .icon_name("dialog-warning-symbolic")
        .title(title)
        .description(description)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .build()
}

fn yaml_explanation_header(kind: &str, name: &str, namespace: &str) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    header.set_valign(gtk::Align::Start);

    let icon = gtk::Image::from_icon_name("text-x-generic-symbolic");
    icon.set_pixel_size(42);
    icon.add_css_class("dim-label");
    header.append(&icon);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let title = gtk::Label::builder()
        .label(tr_format(
            "{kind} Manifest",
            &[("{kind}", kind.to_string())],
        ))
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .css_classes(["title-2"])
        .build();
    let subtitle = gtk::Label::builder()
        .label(if namespace == "-" {
            tr_format(
                "{name} at cluster scope",
                &[("{name}", display_yaml_value(name).to_string())],
            )
        } else {
            tr_format(
                "{name} in namespace {namespace}",
                &[
                    ("{name}", display_yaml_value(name).to_string()),
                    ("{namespace}", namespace.to_string()),
                ],
            )
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

fn yaml_row(
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

fn display_yaml_value(value: &str) -> &str {
    if value.is_empty() || value == "-" {
        "Not set"
    } else {
        value
    }
}

fn namespace_explanation(namespace: &str) -> String {
    if namespace == "-" {
        tr("This resource is cluster-scoped and does not belong to a namespace.")
    } else {
        tr("Namespace that scopes this resource and separates it from other environments.")
    }
}

fn desired_state_explanation(kind: &str, mapping: &serde_yaml::Mapping) -> String {
    if !yaml_has_key(mapping, "spec") {
        return match kind {
            "ConfigMap" => {
                tr("ConfigMaps usually express desired configuration through data instead of spec.")
            }
            "Secret" => tr(
                "Secrets usually express desired sensitive data through data or stringData instead of spec.",
            ),
            _ => tr("This manifest does not include a spec section."),
        };
    }

    spec_explanation(kind)
}

fn spec_rows(
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
                &tr("Replicas"),
                &tr("Number of Pod copies the Deployment should keep running."),
                "spec.replicas",
                "view-grid-symbolic",
            );
            if yaml_mapping_field(spec, "selector").is_some() {
                rows.push((
                    tr("Selector"),
                    tr("Present"),
                    tr("Matches the Pods managed by this Deployment."),
                    String::from("spec.selector"),
                    "edit-find-symbolic",
                ));
            }
            if yaml_mapping_field(spec, "template").is_some() {
                rows.push((
                    tr("Pod Template"),
                    tr("Present"),
                    tr("Template used to create new Pods."),
                    String::from("spec.template"),
                    "document-new-symbolic",
                ));
            }
        }
        "Pod" => {
            if let Some(containers) = yaml_sequence_field(spec, "containers") {
                rows.push((
                    tr("Containers"),
                    format!(
                        "{} {}",
                        containers.len(),
                        trn("container", "containers", containers.len() as u32)
                    ),
                    tr("Application containers started together in this Pod."),
                    String::from("spec.containers"),
                    "utilities-terminal-symbolic",
                ));
            }
            push_yaml_field(
                &mut rows,
                spec,
                "restartPolicy",
                &tr("Restart Policy"),
                &tr("Controls how Kubernetes restarts containers after exit."),
                "spec.restartPolicy",
                "view-refresh-symbolic",
            );
            push_yaml_field(
                &mut rows,
                spec,
                "nodeName",
                &tr("Node"),
                &tr("Node currently targeted by this Pod."),
                "spec.nodeName",
                "computer-symbolic",
            );
        }
        "Service" => {
            push_yaml_field(
                &mut rows,
                spec,
                "type",
                &tr("Type"),
                &tr("Controls how the Service is exposed."),
                "spec.type",
                "network-wired-symbolic",
            );
            if yaml_mapping_field(spec, "selector").is_some() {
                rows.push((
                    tr("Selector"),
                    tr("Present"),
                    tr("Selects Pods that receive traffic from this Service."),
                    String::from("spec.selector"),
                    "edit-find-symbolic",
                ));
            }
            if let Some(ports) = yaml_sequence_field(spec, "ports") {
                rows.push((
                    tr("Ports"),
                    format!(
                        "{} {}",
                        ports.len(),
                        trn("port", "ports", ports.len() as u32)
                    ),
                    tr("Network ports exposed by this Service."),
                    String::from("spec.ports"),
                    "network-transmit-receive-symbolic",
                ));
            }
        }
        "Ingress" => {
            if let Some(rules) = yaml_sequence_field(spec, "rules") {
                rows.push((
                    tr("Rules"),
                    format!(
                        "{} {}",
                        rules.len(),
                        trn("rule", "rules", rules.len() as u32)
                    ),
                    tr("HTTP routing rules handled by this Ingress."),
                    String::from("spec.rules"),
                    "network-server-symbolic",
                ));
            }
            if let Some(tls) = yaml_sequence_field(spec, "tls") {
                rows.push((
                    tr("TLS"),
                    format!(
                        "{} {}",
                        tls.len(),
                        trn("entry", "entries", tls.len() as u32)
                    ),
                    tr("TLS certificate routing configuration."),
                    String::from("spec.tls"),
                    "changes-allow-symbolic",
                ));
            }
        }
        "Job" | "CronJob" => {
            if yaml_mapping_field(spec, "jobTemplate").is_some() {
                rows.push((
                    tr("Job Template"),
                    tr("Present"),
                    tr("Template used to create scheduled Jobs."),
                    String::from("spec.jobTemplate"),
                    "document-new-symbolic",
                ));
            }
            push_yaml_field(
                &mut rows,
                spec,
                "schedule",
                &tr("Schedule"),
                &tr("Cron schedule used to create Jobs."),
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
            tr("Field declared in the desired state for this resource."),
            format!("spec.{key}"),
            "dialog-information-symbolic",
        ));
    }

    if rows.is_empty() {
        rows.push((
            tr("Spec"),
            tr("Empty"),
            tr("No desired-state fields are declared in this spec."),
            String::from("spec"),
            "dialog-information-symbolic",
        ));
    }

    rows
}

fn push_yaml_field(
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

fn yaml_value_summary(value: Option<&serde_yaml::Value>) -> String {
    match value {
        Some(serde_yaml::Value::Null) | None => tr("Not set"),
        Some(serde_yaml::Value::Bool(value)) => value.to_string(),
        Some(serde_yaml::Value::Number(value)) => value.to_string(),
        Some(serde_yaml::Value::String(value)) => value.clone(),
        Some(serde_yaml::Value::Sequence(values)) => {
            format!(
                "{} {}",
                values.len(),
                trn("item", "items", values.len() as u32)
            )
        }
        Some(serde_yaml::Value::Mapping(values)) => {
            format!(
                "{} {}",
                values.len(),
                trn("field", "fields", values.len() as u32)
            )
        }
        Some(serde_yaml::Value::Tagged(_)) => tr("Tagged value"),
    }
}

fn yaml_sequence_field<'a>(
    mapping: &'a serde_yaml::Mapping,
    key: &str,
) -> Option<&'a Vec<serde_yaml::Value>> {
    mapping
        .get(serde_yaml::Value::String(key.to_owned()))
        .and_then(serde_yaml::Value::as_sequence)
}

fn yaml_has_key(mapping: &serde_yaml::Mapping, key: &str) -> bool {
    mapping
        .get(serde_yaml::Value::String(key.to_owned()))
        .is_some()
}

fn yaml_mapping_field<'a>(
    mapping: &'a serde_yaml::Mapping,
    key: &str,
) -> Option<&'a serde_yaml::Mapping> {
    mapping
        .get(serde_yaml::Value::String(key.to_owned()))
        .and_then(serde_yaml::Value::as_mapping)
}

fn yaml_string_field<'a>(mapping: &'a serde_yaml::Mapping, key: &str) -> Option<&'a str> {
    mapping
        .get(serde_yaml::Value::String(key.to_owned()))
        .and_then(serde_yaml::Value::as_str)
}

fn kind_explanation(kind: &str) -> String {
    match kind {
        "Pod" => tr("Runs one or more containers together on a node."),
        "Deployment" => tr("Manages ReplicaSets and keeps the desired number of Pods running."),
        "Service" => tr("Gives a stable virtual address and discovery name to a set of Pods."),
        "Ingress" => tr("Routes external HTTP or HTTPS traffic to Services."),
        "ConfigMap" => tr("Stores non-secret configuration consumed by Pods."),
        "Secret" => tr("Stores sensitive configuration. Values under data are base64 encoded."),
        "Job" => tr("Runs Pods until a task completes successfully."),
        "CronJob" => tr("Creates Jobs on a schedule."),
        "Namespace" => tr("Partitions namespaced resources inside a cluster."),
        "Node" => tr("Represents a worker machine registered in the cluster."),
        _ => tr("Declares the Kubernetes resource type for this manifest."),
    }
}

fn spec_explanation(kind: &str) -> String {
    match kind {
        "Pod" => tr(
            "Desired Pod configuration, including containers, volumes, restart policy and scheduling hints.",
        ),
        "Deployment" => {
            tr("Desired Deployment state, usually replicas, selector and the Pod template.")
        }
        "Service" => tr("Desired Service routing, including selector, type and exposed ports."),
        "Ingress" => tr("Desired routing rules, TLS configuration and backend Services."),
        "Job" => tr("Desired one-shot workload, including completion policy and Pod template."),
        "CronJob" => tr("Desired schedule and Job template."),
        "Node" => tr("Node spec is mostly managed by Kubernetes and should be edited carefully."),
        _ => tr(
            "Desired state for this resource. Controllers reconcile the live object toward this section.",
        ),
    }
}
