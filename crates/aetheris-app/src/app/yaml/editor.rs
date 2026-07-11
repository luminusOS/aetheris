use super::super::utils::text_buffer_text;
use super::super::*;

pub(crate) fn ensure_text_tag(
    buffer: &gtk::TextBuffer,
    name: &str,
    properties: &[(&str, &dyn ToValue)],
) {
    if buffer.tag_table().lookup(name).is_none() {
        let _ = buffer.create_tag(Some(name), properties);
    }
}

pub(crate) fn build_yaml_view(buffer: &sourceview5::Buffer) -> sourceview5::View {
    let view = sourceview5::View::with_buffer(buffer);
    view.set_show_line_numbers(true);
    view.set_highlight_current_line(true);
    view.set_tab_width(2);
    view.set_monospace(true);
    view.set_wrap_mode(gtk::WrapMode::None);
    view
}

/// A Ctrl+F search bar for a YAML editor: highlights every match in the
/// buffer and steps through them. Returns a `Revealer` to place above (or
/// below) the editor's `ScrolledWindow`; wires Ctrl+F on `view` to reveal
/// it and Escape to hide it again.
pub(crate) fn build_yaml_search_bar(
    view: &sourceview5::View,
    buffer: &sourceview5::Buffer,
) -> gtk::Revealer {
    let settings = sourceview5::SearchSettings::new();
    settings.set_wrap_around(true);
    let search_context = sourceview5::SearchContext::new(buffer, Some(&settings));
    search_context.set_highlight(true);

    let entry = gtk::SearchEntry::builder().hexpand(true).build();
    let prev_button = gtk::Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text(tr("Find previous match (Shift+Enter)"))
        .build();
    let next_button = gtk::Button::builder()
        .icon_name("go-down-symbolic")
        .tooltip_text(tr("Find next match (Enter)"))
        .build();
    let count_label = gtk::Label::new(None);
    count_label.add_css_class("dim-label");
    count_label.add_css_class("caption");
    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text(tr("Close search (Escape)"))
        .build();

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bar.set_margin_all(6);
    bar.append(&entry);
    bar.append(&count_label);
    bar.append(&prev_button);
    bar.append(&next_button);
    bar.append(&close_button);

    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .reveal_child(false)
        .build();
    revealer.set_child(Some(&bar));

    search_context.connect_occurrences_count_notify({
        let count_label = count_label.clone();
        let entry = entry.clone();
        move |search_context| {
            let count = search_context.occurrences_count();
            count_label.set_label(&match (entry.text().is_empty(), count) {
                (true, _) => String::new(),
                (false, 0) => tr("No matches"),
                (false, count) if count < 0 => String::new(),
                (false, count) => {
                    format!("{} {}", count, trn("match", "matches", count as u32))
                }
            });
        }
    });

    entry.connect_search_changed({
        let settings = settings.clone();
        move |entry| settings.set_search_text(Some(&entry.text()))
    });

    let find_next = {
        let buffer = buffer.clone();
        let search_context = search_context.clone();
        let view = view.clone();
        move || {
            let text_buffer: &gtk::TextBuffer = buffer.upcast_ref();
            let cursor = text_buffer.iter_at_mark(&text_buffer.get_insert());
            if let Some((start, end, _wrapped)) = search_context.forward(&cursor) {
                text_buffer.select_range(&start, &end);
                view.scroll_to_iter(&mut start.clone(), 0.1, false, 0.0, 0.0);
            }
        }
    };
    let find_previous = {
        let buffer = buffer.clone();
        let search_context = search_context.clone();
        let view = view.clone();
        move || {
            let text_buffer: &gtk::TextBuffer = buffer.upcast_ref();
            let cursor = text_buffer.iter_at_mark(&text_buffer.get_insert());
            if let Some((start, end, _wrapped)) = search_context.backward(&cursor) {
                text_buffer.select_range(&start, &end);
                view.scroll_to_iter(&mut start.clone(), 0.1, false, 0.0, 0.0);
            }
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

/// Wires up YAML syntax highlighting (via GtkSourceView's own lexer, not
/// hand-rolled) and live structural error checking: the first bad line
/// gets a red background and `error_label` shows the parser's message.
pub(crate) fn setup_yaml_buffer(buffer: &sourceview5::Buffer, error_label: &gtk::Label) {
    if let Some(language) = sourceview5::LanguageManager::default().language("yaml") {
        buffer.set_language(Some(&language));
    }
    let scheme_id = if adw::StyleManager::default().is_dark() {
        "Adwaita-dark"
    } else {
        "Adwaita"
    };
    if let Some(scheme) = sourceview5::StyleSchemeManager::default().scheme(scheme_id) {
        buffer.set_style_scheme(Some(&scheme));
    }

    ensure_text_tag(
        buffer.upcast_ref(),
        "yaml-error-line",
        &[("background", &"rgba(224, 27, 36, 0.18)")],
    );

    buffer.connect_changed({
        let error_label = error_label.clone();
        move |buffer| update_yaml_error_state(buffer, &error_label)
    });
    update_yaml_error_state(buffer, error_label);
}

fn update_yaml_error_state(buffer: &sourceview5::Buffer, error_label: &gtk::Label) {
    let text_buffer: &gtk::TextBuffer = buffer.upcast_ref();
    if let Some(tag) = text_buffer.tag_table().lookup("yaml-error-line") {
        text_buffer.remove_tag(&tag, &text_buffer.start_iter(), &text_buffer.end_iter());
    }

    // Kept permanently visible (just blank when there's nothing to report)
    // rather than toggling `set_visible`: hiding it removed its hexpand
    // from the buttons row's layout, so the buttons jumped position
    // depending on whether an error happened to be showing.
    let Some((line, message)) = yaml_parse_error(&text_buffer_text(text_buffer)) else {
        error_label.set_label("");
        return;
    };

    if let Some(tag) = text_buffer.tag_table().lookup("yaml-error-line")
        && let Some(start) = text_buffer.iter_at_line((line - 1) as i32)
    {
        let mut end = start;
        end.forward_to_line_end();
        text_buffer.apply_tag(&tag, &start, &end);
    }
    error_label.set_label(&tr_format(
        "Line {line}: {message}",
        &[
            ("{line}", line.to_string()),
            ("{message}", message.to_string()),
        ],
    ));
}

pub(super) fn yaml_parse_error(text: &str) -> Option<(usize, String)> {
    if text.trim().is_empty() {
        return None;
    }
    let error = serde_yaml::from_str::<serde_yaml::Value>(text).err()?;
    let line = error.location().map_or(1, |location| location.line());
    Some((line, error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::yaml_parse_error;

    #[test]
    fn yaml_parse_error_accepts_valid_yaml() {
        assert_eq!(yaml_parse_error("apiVersion: v1\nkind: Pod\n"), None);
    }

    #[test]
    fn yaml_parse_error_ignores_blank_text() {
        assert_eq!(yaml_parse_error(""), None);
        assert_eq!(yaml_parse_error("   \n  "), None);
    }

    #[test]
    fn yaml_parse_error_reports_the_offending_line() {
        let (line, message) = yaml_parse_error("apiVersion: v1\nkind: Pod\n  bad: [1, 2\n")
            .expect("malformed YAML should fail to parse");
        assert_eq!(line, 3);
        assert!(!message.is_empty());
    }
}
