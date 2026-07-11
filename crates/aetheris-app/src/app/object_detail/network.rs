use super::super::*;

pub(crate) fn rebuild_service_ports(list: &gtk::ListBox, detail: &ObjectDetail) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if detail.service_ports.is_empty() {
        list.append(&super::detail_message_row(
            &tr("No ports"),
            &tr("This Service does not expose any ports."),
            "dialog-information-symbolic",
        ));
        return;
    }

    for port in &detail.service_ports {
        list.append(&service_port_row(port));
    }
}

pub(crate) fn rebuild_service_selectors(list: &gtk::ListBox, detail: &ObjectDetail) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if detail.service_selectors.is_empty() {
        list.append(&super::detail_message_row(
            &tr("No selectors"),
            &tr("This Service does not select Pods."),
            "dialog-information-symbolic",
        ));
        return;
    }

    for selector in &detail.service_selectors {
        let row = adw::ActionRow::builder()
            .title(&selector.key)
            .subtitle(&selector.value)
            .build();
        row.add_prefix(&gtk::Image::from_icon_name("lucide-waypoints-symbolic"));
        list.append(&row);
    }
}

pub(crate) fn rebuild_ingress_rules(list: &gtk::ListBox, detail: &ObjectDetail) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    if detail.ingress_rules.is_empty() {
        list.append(&super::detail_message_row(
            &tr("No rules"),
            &tr("This Ingress does not route traffic to a Service."),
            "dialog-information-symbolic",
        ));
        return;
    }

    for rule in &detail.ingress_rules {
        list.append(&ingress_rule_row(rule));
    }
}

fn ingress_rule_row(rule: &IngressRule) -> adw::ActionRow {
    let target = tr_format(
        "{service}:{port}",
        &[
            ("{service}", rule.service.clone()),
            ("{port}", rule.port.clone()),
        ],
    );
    let subtitle = tr_format(
        "{path_type} → {target}",
        &[
            ("{path_type}", rule.path_type.clone()),
            ("{target}", target),
        ],
    );
    let row = adw::ActionRow::builder()
        .title(format!("{}{}", rule.host, rule.path))
        .subtitle(subtitle)
        .build();
    row.add_prefix(&gtk::Image::from_icon_name("lucide-radio-tower-symbolic"));
    row
}

fn service_port_row(port: &ServicePort) -> adw::ActionRow {
    let title = tr_format(
        "Port {port} → Target {target}",
        &[
            ("{port}", port.port.clone()),
            ("{target}", port.target_port.clone()),
        ],
    );
    let mut details = vec![port.protocol.clone()];
    if !port.name.is_empty() {
        details.push(port.name.clone());
    }
    if let Some(node_port) = &port.node_port {
        details.push(tr_format(
            "NodePort {port}",
            &[("{port}", node_port.clone())],
        ));
    }

    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(details.join(" · "))
        .build();
    row.add_prefix(&gtk::Image::from_icon_name(
        "network-transmit-receive-symbolic",
    ));
    row
}
