use relm4::{adw, gtk};
use similar::TextDiff;

use super::super::{
    i18n::{tr, tr_format},
    projects::DetailTarget,
};
use adw::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffLineKind {
    Context,
    Addition,
    Deletion,
    Hunk,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiffLine {
    text: String,
    kind: DiffLineKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YamlDiff {
    lines: Vec<DiffLine>,
}

pub(crate) fn yaml_diff(original: &str, edited: &str) -> Option<YamlDiff> {
    if original == edited {
        return None;
    }

    let unified = TextDiff::from_lines(original, edited)
        .unified_diff()
        .context_radius(3)
        .to_string();
    let lines = unified
        .split_inclusive('\n')
        .map(|text| DiffLine {
            kind: classify_line(text),
            text: text.to_owned(),
        })
        .collect();

    Some(YamlDiff { lines })
}

fn classify_line(line: &str) -> DiffLineKind {
    if line.starts_with("@@") {
        DiffLineKind::Hunk
    } else if line.starts_with('+') {
        DiffLineKind::Addition
    } else if line.starts_with('-') {
        DiffLineKind::Deletion
    } else if line.starts_with('\\') {
        DiffLineKind::Hint
    } else {
        DiffLineKind::Context
    }
}

pub(crate) fn build_apply_yaml_dialog(target: &DetailTarget, diff: &YamlDiff) -> adw::AlertDialog {
    let dialog = adw::AlertDialog::builder()
        .heading(tr("Apply YAML changes?"))
        .body(tr_format(
            "The following changes will be applied to {kind} {name}.",
            &[
                ("{kind}", target.resource.kind.clone()),
                ("{name}", target.name.clone()),
            ],
        ))
        .build();
    dialog.add_responses(&[("cancel", &tr("Cancel")), ("apply", &tr("Apply"))]);
    dialog.set_close_response("cancel");
    dialog.set_default_response(Some("apply"));
    dialog.set_response_appearance("apply", adw::ResponseAppearance::Suggested);

    let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    let dark = adw::StyleManager::default().is_dark();
    let mut iter = buffer.start_iter();
    for line in &diff.lines {
        if let Some(markup) = line_markup(line, dark) {
            buffer.insert_markup(&mut iter, &markup);
        } else {
            buffer.insert(&mut iter, &line.text);
        }
    }

    let view = gtk::TextView::builder()
        .buffer(&buffer)
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::None)
        .left_margin(12)
        .right_margin(12)
        .top_margin(12)
        .bottom_margin(12)
        .build();
    view.add_css_class("card");

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(820)
        .max_content_width(900)
        .min_content_height(360)
        .max_content_height(520)
        .propagate_natural_width(true)
        .propagate_natural_height(true)
        .vexpand(true)
        .child(&view)
        .build();
    dialog.set_extra_child(Some(&scrolled));
    dialog
}

fn line_markup(line: &DiffLine, dark: bool) -> Option<String> {
    let (foreground, background, emphasis) = match (line.kind, dark) {
        (DiffLineKind::Context, _) => return None,
        (DiffLineKind::Addition, true) => ("#57e389", "#193d2a", "weight=\"bold\""),
        (DiffLineKind::Addition, false) => ("#1b8553", "#d9f2e3", "weight=\"bold\""),
        (DiffLineKind::Deletion, true) => ("#ff7b63", "#45201d", "weight=\"bold\""),
        (DiffLineKind::Deletion, false) => ("#c01c28", "#f8d7da", "weight=\"bold\""),
        (DiffLineKind::Hunk, true) => ("#f8e45c", "#443d18", "weight=\"bold\""),
        (DiffLineKind::Hunk, false) => ("#7a4e00", "#f9eac2", "weight=\"bold\""),
        (DiffLineKind::Hint, true) => ("#9a9996", "transparent", "style=\"italic\""),
        (DiffLineKind::Hint, false) => ("#77767b", "transparent", "style=\"italic\""),
    };
    let text = gtk::glib::markup_escape_text(&line.text);
    if background == "transparent" {
        Some(format!(
            "<span foreground=\"{foreground}\" {emphasis}>{text}</span>"
        ))
    } else {
        Some(format!(
            "<span foreground=\"{foreground}\" background=\"{background}\" {emphasis}>{text}</span>"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(diff: &YamlDiff, kind: DiffLineKind) -> Vec<&str> {
        diff.lines
            .iter()
            .filter(|line| line.kind == kind)
            .map(|line| line.text.as_str())
            .collect()
    }

    #[test]
    fn identical_yaml_has_no_diff() {
        assert!(yaml_diff("kind: Pod\n", "kind: Pod\n").is_none());
    }

    #[test]
    fn classifies_additions_deletions_and_hunks() {
        let diff = yaml_diff("spec:\n  replicas: 1\n", "spec:\n  replicas: 2\n")
            .expect("changed YAML must produce a diff");
        assert!(lines(&diff, DiffLineKind::Hunk)[0].starts_with("@@"));
        assert_eq!(lines(&diff, DiffLineKind::Deletion), ["-  replicas: 1\n"]);
        assert_eq!(lines(&diff, DiffLineKind::Addition), ["+  replicas: 2\n"]);
    }

    #[test]
    fn keeps_three_context_lines_and_multiple_hunks() {
        let original = (1..=14)
            .map(|line| format!("key{line}: old\n"))
            .collect::<String>();
        let edited = original
            .replace("key2: old", "key2: new")
            .replace("key13: old", "key13: new");
        let diff = yaml_diff(&original, &edited).expect("changed YAML must produce a diff");
        assert_eq!(lines(&diff, DiffLineKind::Hunk).len(), 2);
        assert!(!diff.lines.iter().any(|line| line.text.contains("key7:")));
    }

    #[test]
    fn preserves_unicode_and_reports_missing_newline() {
        let diff = yaml_diff("name: café\n", "name: foguete 🚀")
            .expect("changed YAML must produce a diff");
        assert!(lines(&diff, DiffLineKind::Addition)[0].contains("foguete 🚀"));
        assert!(!lines(&diff, DiffLineKind::Hint).is_empty());
    }

    #[test]
    fn markup_has_explicit_theme_colors_and_escapes_yaml_text() {
        let line = DiffLine {
            text: "+ value: <changed>\n".to_owned(),
            kind: DiffLineKind::Addition,
        };
        let dark = line_markup(&line, true).expect("addition must have markup");
        let light = line_markup(&line, false).expect("addition must have markup");
        assert!(dark.contains("foreground=\"#57e389\""));
        assert!(dark.contains("background=\"#193d2a\""));
        assert!(light.contains("foreground=\"#1b8553\""));
        assert!(light.contains("background=\"#d9f2e3\""));
        assert!(dark.contains("&lt;changed&gt;"));
    }
}
