use super::super::dialogs::*;
use super::super::widgets::{selector_button_child, selector_popover};
use super::super::*;

pub(super) struct NamespacesWidgets {
    pub(super) namespace_menu_button: gtk::MenuButton,
    pub(super) namespace_selector_label: gtk::Label,
    pub(super) namespace_list: gtk::ListBox,
    pub(super) custom_namespace_entry: adw::EntryRow,
    pub(super) custom_namespace_button: gtk::Button,
    pub(super) custom_namespace_dialog: adw::Dialog,
    pub(super) rename_namespace_entry: adw::EntryRow,
    pub(super) rename_namespace_button: gtk::Button,
    pub(super) rename_namespace_dialog: adw::Dialog,
}

/// Builds the namespace selector (popover + list), the "add custom
/// namespace" dialog, and the "rename namespace" dialog, and wires their
/// signals to `AppMsg`.
pub(super) fn build(sender: &ComponentSender<App>) -> NamespacesWidgets {
    let namespace_selector_label = gtk::Label::new(Some("default"));
    let namespace_menu_button = gtk::MenuButton::new();
    namespace_menu_button.set_size_request(170, -1);
    namespace_menu_button.set_child(Some(&selector_button_child(&namespace_selector_label)));
    let namespace_list = gtk::ListBox::new();
    namespace_menu_button.set_popover(Some(&selector_popover(&namespace_list)));

    let custom_namespace_entry = adw::EntryRow::builder()
        .title(tr("Namespace"))
        .hexpand(true)
        .build();
    let custom_namespace_button = gtk::Button::builder()
        .label(tr("Use"))
        .tooltip_text(tr("Use and save this namespace"))
        .build();
    custom_namespace_button.add_css_class("suggested-action");
    let custom_namespace_dialog =
        build_custom_namespace_dialog(&custom_namespace_entry, &custom_namespace_button);

    let rename_namespace_entry = adw::EntryRow::builder()
        .title(tr("Namespace"))
        .hexpand(true)
        .build();
    let rename_namespace_button = gtk::Button::builder().label(tr("Rename")).build();
    rename_namespace_button.add_css_class("suggested-action");
    let rename_namespace_dialog =
        build_rename_namespace_dialog(&rename_namespace_entry, &rename_namespace_button);

    namespace_list.connect_row_activated({
        let sender = sender.clone();
        move |_, row| sender.input(AppMsg::NamespaceChanged(row.index() as u32))
    });
    custom_namespace_entry.connect_entry_activated({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::CustomNamespaceEntered)
    });
    custom_namespace_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::CustomNamespaceEntered)
    });
    rename_namespace_entry.connect_entry_activated({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::RenameNamespaceConfirmed)
    });
    rename_namespace_button.connect_clicked({
        let sender = sender.clone();
        move |_| sender.input(AppMsg::RenameNamespaceConfirmed)
    });

    NamespacesWidgets {
        namespace_menu_button,
        namespace_selector_label,
        namespace_list,
        custom_namespace_entry,
        custom_namespace_button,
        custom_namespace_dialog,
        rename_namespace_entry,
        rename_namespace_button,
        rename_namespace_dialog,
    }
}
