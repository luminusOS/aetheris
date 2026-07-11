use super::super::*;

pub(crate) fn event_row(event: &ObjectEvent) -> gtk::ListBoxRow {
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

fn event_icon_name(event: &ObjectEvent) -> &'static str {
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
