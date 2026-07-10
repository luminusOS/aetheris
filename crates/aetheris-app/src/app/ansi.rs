use super::yaml::ensure_text_tag;
use super::*;

const ESC: char = '\u{1b}';

pub(super) fn setup_log_highlighting(buffer: &gtk::TextBuffer) {
    ensure_text_tag(buffer, "ansi-bold", &[("weight", &700)]);
    ensure_text_tag(buffer, "ansi-fg-black", &[("foreground", &"#9a9996")]);
    ensure_text_tag(buffer, "ansi-fg-red", &[("foreground", &"#e01b24")]);
    ensure_text_tag(buffer, "ansi-fg-green", &[("foreground", &"#2ec27e")]);
    ensure_text_tag(buffer, "ansi-fg-yellow", &[("foreground", &"#e5a50a")]);
    ensure_text_tag(buffer, "ansi-fg-blue", &[("foreground", &"#3584e4")]);
    ensure_text_tag(buffer, "ansi-fg-magenta", &[("foreground", &"#c061cb")]);
    ensure_text_tag(buffer, "ansi-fg-cyan", &[("foreground", &"#0891b2")]);
    ensure_text_tag(buffer, "ansi-fg-white", &[("foreground", &"#f6f5f4")]);
}

pub(super) fn insert_ansi_line(buffer: &gtk::TextBuffer, line: &str) {
    let mut iter = buffer.end_iter();
    for (text, tags) in parse_ansi_line(line) {
        let start_offset = iter.offset();
        buffer.insert(&mut iter, &text);
        if !tags.is_empty() {
            let start_iter = buffer.iter_at_offset(start_offset);
            for tag in tags {
                buffer.apply_tag_by_name(tag, &start_iter, &iter);
            }
        }
    }
}

fn parse_ansi_line(line: &str) -> Vec<(String, Vec<&'static str>)> {
    let mut segments = Vec::new();
    let mut current_text = String::new();
    let mut active_tags: Vec<&'static str> = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != ESC || chars.peek() != Some(&'[') {
            current_text.push(ch);
            continue;
        }
        chars.next();

        let mut code = String::new();
        let mut terminator = None;
        for next in chars.by_ref() {
            if next.is_ascii_digit() || next == ';' {
                code.push(next);
            } else {
                terminator = Some(next);
                break;
            }
        }

        if !current_text.is_empty() {
            segments.push((std::mem::take(&mut current_text), active_tags.clone()));
        }
        if terminator == Some('m') {
            apply_sgr_codes(&code, &mut active_tags);
        }
    }

    if !current_text.is_empty() {
        segments.push((current_text, active_tags));
    }

    segments
}

fn apply_sgr_codes(code: &str, active_tags: &mut Vec<&'static str>) {
    if code.is_empty() {
        active_tags.clear();
        return;
    }
    for part in code.split(';') {
        match part.parse::<u32>() {
            Ok(0) => active_tags.clear(),
            Ok(1) => push_unique(active_tags, "ansi-bold"),
            Ok(21) | Ok(22) => active_tags.retain(|tag| *tag != "ansi-bold"),
            Ok(code @ (30..=37 | 90..=97)) => {
                active_tags.retain(|tag| !tag.starts_with("ansi-fg-"));
                active_tags.push(fg_tag(code % 10));
            }
            Ok(39) => active_tags.retain(|tag| !tag.starts_with("ansi-fg-")),
            _ => {}
        }
    }
}

fn push_unique(active_tags: &mut Vec<&'static str>, tag: &'static str) {
    if !active_tags.contains(&tag) {
        active_tags.push(tag);
    }
}

fn fg_tag(index: u32) -> &'static str {
    match index {
        0 => "ansi-fg-black",
        1 => "ansi-fg-red",
        2 => "ansi-fg-green",
        3 => "ansi-fg-yellow",
        4 => "ansi-fg-blue",
        5 => "ansi-fg-magenta",
        6 => "ansi-fg-cyan",
        _ => "ansi-fg-white",
    }
}

#[cfg(test)]
mod tests {
    use super::parse_ansi_line;

    #[test]
    fn parse_ansi_line_strips_escape_codes_from_plain_text() {
        let segments = parse_ansi_line("no colors here");
        assert_eq!(segments, vec![(String::from("no colors here"), vec![])]);
    }

    #[test]
    fn parse_ansi_line_extracts_a_single_color() {
        let segments = parse_ansi_line("\u{1b}[32mINFO\u{1b}[0m starting up");
        assert_eq!(
            segments,
            vec![
                (String::from("INFO"), vec!["ansi-fg-green"]),
                (String::from(" starting up"), vec![]),
            ]
        );
    }

    #[test]
    fn parse_ansi_line_combines_bold_and_color_in_one_sequence() {
        let segments = parse_ansi_line("\u{1b}[1;31mERROR\u{1b}[0m failed");
        assert_eq!(
            segments,
            vec![
                (String::from("ERROR"), vec!["ansi-bold", "ansi-fg-red"]),
                (String::from(" failed"), vec![]),
            ]
        );
    }

    #[test]
    fn parse_ansi_line_maps_bright_colors_to_the_base_tag() {
        let segments = parse_ansi_line("\u{1b}[93mWARN\u{1b}[0m");
        assert_eq!(
            segments,
            vec![(String::from("WARN"), vec!["ansi-fg-yellow"])]
        );
    }
}
