use super::filters::{StatusTone, status_chip};
use super::*;

pub(crate) fn rebuild_cluster_list(
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

pub(crate) fn cluster_row(
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
