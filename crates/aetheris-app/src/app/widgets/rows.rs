use super::classify::{available_icon_name, resource_icon_name};
use super::*;

pub(crate) fn section_title(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class("heading");
    label
}

pub(crate) fn selector_button_child(label: &gtk::Label) -> gtk::Box {
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

pub(crate) fn selector_popover(list: &gtk::ListBox) -> gtk::Popover {
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

pub(crate) fn resource_count_label(count: usize) -> String {
    format!(
        "{} {}",
        count,
        trn("resource type", "resource types", count as u32)
    )
}

pub(crate) fn namespace_selector_row(
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

pub(crate) fn add_namespace_selector_row() -> gtk::ListBoxRow {
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

pub(crate) fn rebuild_project_list(list: &gtk::ListBox, projects: &ProjectStore) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    for project in &projects.projects {
        let selected = projects.selected_project.as_deref() == Some(project.name.as_str());
        list.append(&project_row(project, selected));
    }
}

pub(crate) fn project_row(project: &Project, selected: bool) -> gtk::ListBoxRow {
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

pub(crate) fn project_context_count(count: usize) -> String {
    format!(
        "{} {}",
        count,
        if count == 1 { "cluster" } else { "clusters" }
    )
}

pub(crate) fn connect_resource_row(
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

pub(crate) fn connect_favorite_object_row(
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

pub(crate) fn resource_row(resource: &ResourceKind, selected: bool) -> adw::ActionRow {
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

pub(crate) fn favorite_object_row(favorite: &ObjectFavorite) -> adw::ActionRow {
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
