use super::*;

#[derive(Clone, Copy)]
pub(crate) enum StatusTone {
    Good,
    Warning,
    Bad,
    Info,
    Neutral,
}

impl StatusTone {
    pub(crate) fn css_class(self) -> &'static str {
        match self {
            Self::Good => "status-good",
            Self::Warning => "status-warning",
            Self::Bad => "status-bad",
            Self::Info => "status-info",
            Self::Neutral => "status-neutral",
        }
    }
}

pub(crate) fn status_chip(text: &str, tone: StatusTone) -> gtk::Label {
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

pub(crate) fn status_tone(status: &str) -> StatusTone {
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

pub(crate) fn rebuild_status_filter_list(list: &gtk::FlowBox, selected: &BTreeSet<StatusFilter>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    for filter in StatusFilter::ALL {
        list.insert(&status_filter_chip(filter, selected.contains(&filter)), -1);
    }
}

pub(crate) fn rebuild_column_filter_list(
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
