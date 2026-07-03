use super::yaml::*;
use super::*;

pub(super) fn build_custom_namespace_dialog(
    entry: &adw::EntryRow,
    apply_button: &gtk::Button,
) -> adw::Dialog {
    let dialog = adw::Dialog::builder()
        .title("Namespace")
        .content_width(460)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.set_margin_all(18);

    let title = gtk::Label::new(Some("Use a Custom Namespace"));
    title.set_xalign(0.0);
    title.add_css_class("title-4");
    content.append(&title);

    let subtitle = gtk::Label::new(Some(
        "Enter a namespace that was not returned by the cluster. It will be saved for this project.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.add_css_class("dim-label");
    content.append(&subtitle);

    content.append(&entry_list(&[entry.upcast_ref()]));

    apply_button.add_css_class("pill");
    apply_button.set_halign(gtk::Align::Center);
    apply_button.set_margin_top(6);
    content.append(apply_button);

    toolbar.set_content(Some(&content));
    dialog.set_child(Some(&toolbar));
    dialog
}

pub(super) fn build_rename_namespace_dialog(
    entry: &adw::EntryRow,
    apply_button: &gtk::Button,
) -> adw::Dialog {
    let dialog = adw::Dialog::builder()
        .title("Rename Namespace")
        .content_width(460)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.set_margin_all(18);

    let title = gtk::Label::new(Some("Rename Namespace"));
    title.set_xalign(0.0);
    title.add_css_class("title-4");
    content.append(&title);

    let subtitle = gtk::Label::new(Some(
        "This only renames the saved shortcut. It does not rename anything in the cluster.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.add_css_class("dim-label");
    content.append(&subtitle);

    content.append(&entry_list(&[entry.upcast_ref()]));

    apply_button.add_css_class("pill");
    apply_button.set_halign(gtk::Align::Center);
    apply_button.set_margin_top(6);
    content.append(apply_button);

    toolbar.set_content(Some(&content));
    dialog.set_child(Some(&toolbar));
    dialog
}

pub(super) fn build_project_dialog(
    entry: &adw::EntryRow,
    create_button: &gtk::Button,
    description: &gtk::Label,
) -> adw::Dialog {
    let dialog = adw::Dialog::builder()
        .title("New Project")
        .content_width(420)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_all(18);

    description.set_xalign(0.0);
    description.set_wrap(true);
    description.add_css_class("title-3");
    content.append(description);

    content.append(&entry_list(&[entry.upcast_ref()]));

    create_button.add_css_class("pill");
    create_button.set_halign(gtk::Align::Center);
    create_button.set_margin_top(6);
    content.append(create_button);

    toolbar.set_content(Some(&content));
    dialog.set_child(Some(&toolbar));
    dialog
}

pub(super) fn build_create_yaml_dialog(
    buffer: &sourceview5::Buffer,
    create_button: &gtk::Button,
    error_label: &gtk::Label,
) -> adw::Dialog {
    let dialog = adw::Dialog::builder()
        .title("Create from YAML")
        .content_width(760)
        .content_height(560)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    let view = build_yaml_view(buffer);
    view.set_editable(true);
    view.set_cursor_visible(true);
    let search_bar = build_yaml_search_bar(&view, buffer);

    let scrolled = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .min_content_height(360)
        .build();
    scrolled.set_child(Some(&view));

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_all(12);
    error_label.set_xalign(0.0);
    error_label.set_wrap(true);
    error_label.add_css_class("caption");
    error_label.add_css_class("error");
    content.append(&search_bar);
    content.append(error_label);
    content.append(&scrolled);

    create_button.add_css_class("pill");
    create_button.set_halign(gtk::Align::Center);
    create_button.set_margin_top(6);
    content.append(create_button);

    toolbar.set_content(Some(&content));
    dialog.set_child(Some(&toolbar));
    dialog
}

pub(super) struct ClusterDialogWidgets<'a> {
    pub(super) stack: &'a gtk::Stack,
    pub(super) name_entry: &'a adw::EntryRow,
    pub(super) server_entry: &'a adw::EntryRow,
    pub(super) token_entry: &'a adw::PasswordEntryRow,
    pub(super) ca_entry: &'a adw::EntryRow,
    pub(super) insecure_check: &'a adw::SwitchRow,
    pub(super) add_button: &'a gtk::Button,
    pub(super) title_label: &'a gtk::Label,
    pub(super) back_button: &'a gtk::Button,
}

pub(super) fn build_cluster_dialog(
    widgets: ClusterDialogWidgets<'_>,
    sender: ComponentSender<App>,
) -> adw::Dialog {
    let dialog = adw::Dialog::builder()
        .title("Add Cluster")
        .content_width(520)
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());

    widgets
        .stack
        .add_named(&cluster_options_page(sender.clone()), Some("options"));
    widgets
        .stack
        .add_named(&token_cluster_page(&widgets, sender), Some("token"));
    widgets.stack.set_visible_child_name("options");

    toolbar.set_content(Some(widgets.stack));
    dialog.set_child(Some(&toolbar));
    dialog
}

pub(super) fn cluster_options_page(sender: ComponentSender<App>) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 14);
    container.set_margin_all(18);

    let title = gtk::Label::new(Some("Choose how to connect"));
    title.set_xalign(0.0);
    title.add_css_class("title-3");
    container.append(&title);

    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    list.append(&option_row(
        "Connect with token",
        "Use an API server URL and bearer token.",
        "dialog-password-symbolic",
    ));
    list.append(&option_row(
        "Import kubeconfig",
        "Merge contexts from an existing kubeconfig file.",
        "document-open-symbolic",
    ));
    list.connect_row_activated(move |_, row| match row.index() {
        0 => sender.input(AppMsg::ShowTokenForm),
        1 => sender.input(AppMsg::ShowImportFile),
        _ => {}
    });
    container.append(&list);

    container
}

pub(super) fn token_cluster_page(
    widgets: &ClusterDialogWidgets<'_>,
    sender: ComponentSender<App>,
) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 14);
    container.set_margin_all(18);

    let back = widgets.back_button;
    back.add_css_class("flat");
    back.set_tooltip_text(Some("Back to connection options"));
    back.connect_clicked(move |_| sender.input(AppMsg::ShowAddClusterDialog));

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    heading.append(back);
    let title = widgets.title_label;
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("title-3");
    heading.append(title);
    container.append(&heading);

    container.append(&entry_list(&[
        widgets.name_entry.upcast_ref(),
        widgets.server_entry.upcast_ref(),
        widgets.token_entry.upcast_ref(),
        widgets.ca_entry.upcast_ref(),
        widgets.insecure_check.upcast_ref(),
    ]));

    widgets.add_button.add_css_class("pill");
    widgets.add_button.set_halign(gtk::Align::Center);
    widgets.add_button.set_margin_top(6);
    container.append(widgets.add_button);
    container
}

pub(super) fn option_row(title: &str, subtitle: &str, icon_name: &str) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let action = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .activatable(true)
        .build();
    let icon = gtk::Image::from_icon_name(icon_name);
    action.add_prefix(&icon);
    row.set_child(Some(&action));
    row
}

/// Groups `AdwEntryRow`/`AdwPasswordEntryRow`/`AdwSwitchRow` fields into the
/// standard GNOME HIG "boxed list" form container — each row carries its own
/// label embedded in the top-left corner, so no separate label widget is
/// needed above it.
pub(super) fn entry_list(rows: &[&gtk::Widget]) -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);
    for row in rows {
        list.append(*row);
    }
    list
}
