use super::super::*;

pub(crate) fn rebuild_related_pods(
    store: &gtk::gio::ListStore,
    stack: &gtk::Stack,
    message: &adw::StatusPage,
    detail: &ObjectDetail,
) {
    if detail.kind != "Deployment" {
        store.remove_all();
        message.set_title(&tr("Pods are shown for Deployments"));
        message.set_description(Some(&tr("Open a Deployment to inspect its related Pods.")));
        stack.set_visible_child_name("message");
        return;
    }

    if detail.related_pods.is_empty() {
        store.remove_all();
        message.set_title(&tr("No related Pods"));
        message.set_description(Some(&tr("No Pods matched this Deployment selector.")));
        stack.set_visible_child_name("message");
        return;
    }

    let items: Vec<gtk::glib::BoxedAnyObject> = detail
        .related_pods
        .iter()
        .map(super::super::widgets::boxed_object)
        .collect();
    store.splice(0, store.n_items(), &items);
    stack.set_visible_child_name("table");
}

pub(crate) fn rebuild_related_pod_states(
    section: &gtk::Box,
    states: &gtk::FlowBox,
    detail: &ObjectDetail,
) {
    while let Some(child) = states.first_child() {
        states.remove(&child);
    }

    section.set_visible(detail.kind == "Deployment" && !detail.related_pod_states.is_empty());
    for state in &detail.related_pod_states {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 2);
        card.add_css_class("pod-state-card");
        card.add_css_class(pod_state_css_class(&state.state));
        card.set_hexpand(true);
        card.set_size_request(170, -1);

        let count = gtk::Label::builder()
            .label(state.count.to_string())
            .xalign(0.0)
            .css_classes(["title-1"])
            .build();
        let label = gtk::Label::builder()
            .label(&state.state)
            .xalign(0.0)
            .css_classes(["caption"])
            .build();
        card.append(&count);
        card.append(&label);
        states.insert(&card, -1);
    }
}

fn pod_state_css_class(state: &str) -> &'static str {
    match state {
        "Running" => "pod-state-running",
        "Pending" => "pod-state-pending",
        "Succeeded" => "pod-state-succeeded",
        "Failed" => "pod-state-failed",
        _ => "pod-state-unknown",
    }
}
