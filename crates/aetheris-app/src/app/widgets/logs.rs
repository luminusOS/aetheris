use super::*;

/// A Ctrl+F search bar for a plain `gtk::TextView` (the log viewer, which
/// isn't a `sourceview5::Buffer` and so can't use `SearchContext`) — uses
/// GTK's own `TextIter::forward_search`/`backward_search` instead. Returns
/// a `Revealer` to place above the view; wires Ctrl+F on `view` to reveal
/// it and Escape to hide it again.
pub(crate) fn build_log_search_bar(
    view: &gtk::TextView,
    buffer: &gtk::TextBuffer,
) -> gtk::Revealer {
    let entry = gtk::SearchEntry::builder().hexpand(true).build();
    let prev_button = gtk::Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text(tr("Find previous match (Shift+Enter)"))
        .build();
    let next_button = gtk::Button::builder()
        .icon_name("go-down-symbolic")
        .tooltip_text(tr("Find next match (Enter)"))
        .build();
    let status_label = gtk::Label::new(None);
    status_label.add_css_class("dim-label");
    status_label.add_css_class("caption");
    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text(tr("Close search (Escape)"))
        .build();

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bar.set_margin_all(6);
    bar.append(&entry);
    bar.append(&status_label);
    bar.append(&prev_button);
    bar.append(&next_button);
    bar.append(&close_button);

    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .reveal_child(false)
        .build();
    revealer.set_child(Some(&bar));

    let flags = gtk::TextSearchFlags::CASE_INSENSITIVE;

    let jump_to = {
        let view = view.clone();
        let status_label = status_label.clone();
        move |buffer: &gtk::TextBuffer, found: Option<(gtk::TextIter, gtk::TextIter)>| match found {
            Some((start, end)) => {
                buffer.select_range(&start, &end);
                view.scroll_to_iter(&mut start.clone(), 0.1, false, 0.0, 0.0);
                status_label.set_label("");
            }
            None => status_label.set_label(&tr("No matches")),
        }
    };

    entry.connect_search_changed({
        let buffer = buffer.clone();
        let jump_to = jump_to.clone();
        let status_label = status_label.clone();
        move |entry| {
            let query = entry.text();
            if query.is_empty() {
                status_label.set_label("");
                return;
            }
            let found = buffer.start_iter().forward_search(&query, flags, None);
            jump_to(&buffer, found);
        }
    });

    let find_next = {
        let buffer = buffer.clone();
        let entry = entry.clone();
        let jump_to = jump_to.clone();
        move || {
            let query = entry.text();
            if query.is_empty() {
                return;
            }
            let from = buffer
                .selection_bounds()
                .map(|(_, end)| end)
                .unwrap_or_else(|| buffer.iter_at_mark(&buffer.get_insert()));
            let found = from
                .forward_search(&query, flags, None)
                .or_else(|| buffer.start_iter().forward_search(&query, flags, None));
            jump_to(&buffer, found);
        }
    };
    let find_previous = {
        let buffer = buffer.clone();
        let entry = entry.clone();
        let jump_to = jump_to.clone();
        move || {
            let query = entry.text();
            if query.is_empty() {
                return;
            }
            let from = buffer
                .selection_bounds()
                .map(|(start, _)| start)
                .unwrap_or_else(|| buffer.iter_at_mark(&buffer.get_insert()));
            let found = from
                .backward_search(&query, flags, None)
                .or_else(|| buffer.end_iter().backward_search(&query, flags, None));
            jump_to(&buffer, found);
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
