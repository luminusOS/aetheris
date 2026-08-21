use std::borrow::Cow;

use relm4::adw;
use relm4::gtk;
use relm4::gtk::prelude::*;
use zcolorizer::config::Config;
use zcolorizer::rules::{Rule, compile_all};

const MAX_HIGHLIGHT_BYTES: usize = 64 * 1024;
const LOG_MODULES: &[&str] = &["json", "logfmt", "spring", "python", "rust-log", "rails"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogStyleRole {
    Error,
    Warning,
    Success,
    Info,
    Debug,
    Metadata,
    Identity,
    Numeric,
    TextValue,
    Link,
    Keyword,
    Key,
}

impl LogStyleRole {
    const ALL: [Self; 12] = [
        Self::Error,
        Self::Warning,
        Self::Success,
        Self::Info,
        Self::Debug,
        Self::Metadata,
        Self::Identity,
        Self::Numeric,
        Self::TextValue,
        Self::Link,
        Self::Keyword,
        Self::Key,
    ];

    const fn tag_name(self) -> &'static str {
        match self {
            Self::Error => "log-error",
            Self::Warning => "log-warning",
            Self::Success => "log-success",
            Self::Info => "log-info",
            Self::Debug => "log-debug",
            Self::Metadata => "log-metadata",
            Self::Identity => "log-identity",
            Self::Numeric => "log-numeric",
            Self::TextValue => "log-text-value",
            Self::Link => "log-link",
            Self::Keyword => "log-keyword",
            Self::Key => "log-key",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HighlightedSpan {
    pub start: usize,
    pub end: usize,
    pub role: LogStyleRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HighlightedLine {
    pub text: String,
    pub spans: Vec<HighlightedSpan>,
}

#[derive(Debug)]
pub(super) struct LogHighlighter {
    rules: Vec<Rule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    Unclaimed,
    Plain,
    Styled(LogStyleRole),
}

impl LogHighlighter {
    pub(super) fn new() -> zcolorizer::Result<Self> {
        debug_assert!(
            LOG_MODULES
                .iter()
                .all(|name| zcolorizer::modules::get(name).is_some())
        );
        let config = Config {
            modules: LOG_MODULES.iter().map(|name| (*name).to_string()).collect(),
            ..Config::default()
        };
        Ok(Self {
            rules: compile_all(&config.resolve_rule_defs())?,
        })
    }

    pub(super) const fn disabled() -> Self {
        Self { rules: Vec::new() }
    }

    pub(super) fn highlight_line(&self, line: &str) -> HighlightedLine {
        let text = sanitize(line).into_owned();
        if text.is_empty() || text.len() > MAX_HIGHLIGHT_BYTES || self.rules.is_empty() {
            return HighlightedLine {
                text,
                spans: Vec::new(),
            };
        }

        let mut owners = vec![Owner::Unclaimed; text.len()];
        for rule in &self.rules {
            if rule.has_named_groups {
                claim_named_rule(&mut owners, rule, &text);
            } else {
                claim_whole_rule(&mut owners, rule, &text);
            }
        }

        HighlightedLine {
            spans: styled_spans(&owners),
            text,
        }
    }
}

fn claim_named_rule(owners: &mut [Owner], rule: &Rule, text: &str) {
    for captures in rule.regex.captures_iter(text) {
        for (index, token) in rule.group_tokens.iter().enumerate() {
            let (Some(token), Some(found)) = (token, captures.get(index)) else {
                continue;
            };
            claim(owners, found.start(), found.end(), style_role(token));
        }
    }
}

fn claim_whole_rule(owners: &mut [Owner], rule: &Rule, text: &str) {
    for found in rule.regex.find_iter(text) {
        claim(
            owners,
            found.start(),
            found.end(),
            style_role(&rule.whole_token),
        );
    }
}

fn claim(owners: &mut [Owner], start: usize, end: usize, role: Option<LogStyleRole>) {
    let owner = role.map_or(Owner::Plain, Owner::Styled);
    for slot in &mut owners[start..end] {
        if *slot == Owner::Unclaimed {
            *slot = owner;
        }
    }
}

fn styled_spans(owners: &[Owner]) -> Vec<HighlightedSpan> {
    let mut spans = Vec::new();
    let mut index = 0;
    while index < owners.len() {
        let Owner::Styled(role) = owners[index] else {
            index += 1;
            continue;
        };
        let start = index;
        index += 1;
        while index < owners.len() && owners[index] == Owner::Styled(role) {
            index += 1;
        }
        spans.push(HighlightedSpan {
            start,
            end: index,
            role,
        });
    }
    spans
}

fn style_role(token: &str) -> Option<LogStyleRole> {
    match token {
        "error" | "bad" => Some(LogStyleRole::Error),
        "warning" => Some(LogStyleRole::Warning),
        "good" => Some(LogStyleRole::Success),
        "info" => Some(LogStyleRole::Info),
        "debug" | "unknown" => Some(LogStyleRole::Debug),
        "date" | "time" | "duration" | "latency" => Some(LogStyleRole::Metadata),
        "process" | "pid" | "thread" | "facility" | "host" | "service" | "user" => {
            Some(LogStyleRole::Identity)
        }
        "number" | "size" | "percentage" | "address" | "ip" | "mac" | "hash" | "http_code"
        | "version" => Some(LogStyleRole::Numeric),
        "string" | "file" | "dir" => Some(LogStyleRole::TextValue),
        "uri" | "email" => Some(LogStyleRole::Link),
        "http_method" | "protocol" | "keyword" | "signal" | "system" | "level" => {
            Some(LogStyleRole::Keyword)
        }
        "json_key" | "field" | "bracket" => Some(LogStyleRole::Key),
        _ => None,
    }
}

fn sanitize(line: &str) -> Cow<'_, str> {
    let is_control = |ch: char| {
        let code = ch as u32;
        (code < 0x20 && ch != '\t') || code == 0x7f || (0x80..=0x9f).contains(&code)
    };
    if !line.chars().any(is_control) {
        return Cow::Borrowed(line);
    }

    let mut output = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\u{1b}' => match chars.peek() {
                Some('[') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    while let Some(next) = chars.next() {
                        if next == '\u{07}' {
                            break;
                        }
                        if next == '\u{1b}' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                Some(_) => {
                    chars.next();
                }
                None => {}
            },
            '\t' => output.push('\t'),
            ch if is_control(ch) => {}
            ch => output.push(ch),
        }
    }
    Cow::Owned(output)
}

pub(super) fn setup_log_tags(view: &gtk::TextView, buffer: &gtk::TextBuffer) {
    for role in LogStyleRole::ALL {
        if buffer.tag_table().lookup(role.tag_name()).is_none() {
            let _ = buffer.create_tag(Some(role.tag_name()), &[]);
        }
    }
    refresh_log_tags(view, buffer);

    let view = view.downgrade();
    let buffer = buffer.downgrade();
    adw::StyleManager::default().connect_dark_notify(move |_| {
        if let (Some(view), Some(buffer)) = (view.upgrade(), buffer.upgrade()) {
            refresh_log_tags(&view, &buffer);
        }
    });
}

fn refresh_log_tags(view: &gtk::TextView, buffer: &gtk::TextBuffer) {
    let foreground = view.color();
    let (error, warning, success, accent) = if adw::StyleManager::default().is_dark() {
        (
            gtk::gdk::RGBA::new(1.0, 0.48, 0.39, 1.0),
            gtk::gdk::RGBA::new(0.97, 0.89, 0.36, 1.0),
            gtk::gdk::RGBA::new(0.34, 0.89, 0.54, 1.0),
            gtk::gdk::RGBA::new(0.47, 0.68, 0.93, 1.0),
        )
    } else {
        (
            gtk::gdk::RGBA::new(0.75, 0.11, 0.16, 1.0),
            gtk::gdk::RGBA::new(0.56, 0.35, 0.01, 1.0),
            gtk::gdk::RGBA::new(0.15, 0.64, 0.41, 1.0),
            gtk::gdk::RGBA::new(0.11, 0.44, 0.85, 1.0),
        )
    };
    let mut dim = foreground;
    dim.set_alpha(0.65);

    for role in LogStyleRole::ALL {
        let Some(tag) = buffer.tag_table().lookup(role.tag_name()) else {
            continue;
        };
        let color = match role {
            LogStyleRole::Error => &error,
            LogStyleRole::Warning => &warning,
            LogStyleRole::Success | LogStyleRole::TextValue => &success,
            LogStyleRole::Link => &accent,
            LogStyleRole::Debug => &dim,
            LogStyleRole::Info
            | LogStyleRole::Metadata
            | LogStyleRole::Numeric
            | LogStyleRole::Keyword => &accent,
            LogStyleRole::Identity | LogStyleRole::Key => &foreground,
        };
        tag.set_foreground_rgba(Some(color));
        tag.set_weight(
            if matches!(
                role,
                LogStyleRole::Error
                    | LogStyleRole::Warning
                    | LogStyleRole::Success
                    | LogStyleRole::Identity
                    | LogStyleRole::Keyword
                    | LogStyleRole::Key
            ) {
                700
            } else {
                400
            },
        );
        tag.set_style(if role == LogStyleRole::Debug {
            gtk::pango::Style::Italic
        } else {
            gtk::pango::Style::Normal
        });
        tag.set_underline(if role == LogStyleRole::Link {
            gtk::pango::Underline::Single
        } else {
            gtk::pango::Underline::None
        });
    }
}

pub(super) fn insert_highlighted_line(buffer: &gtk::TextBuffer, line: &HighlightedLine) {
    let mut iter = buffer.end_iter();
    let mut cursor = 0;
    for span in &line.spans {
        if cursor < span.start {
            buffer.insert(&mut iter, &line.text[cursor..span.start]);
        }
        let start_offset = iter.offset();
        buffer.insert(&mut iter, &line.text[span.start..span.end]);
        let start = buffer.iter_at_offset(start_offset);
        buffer.apply_tag_by_name(span.role.tag_name(), &start, &iter);
        cursor = span.end;
    }
    if cursor < line.text.len() {
        buffer.insert(&mut iter, &line.text[cursor..]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlighter() -> LogHighlighter {
        LogHighlighter::new().expect("builtin zcolorizer rules must compile")
    }

    fn styled_text(line: &HighlightedLine, role: LogStyleRole) -> Vec<&str> {
        line.spans
            .iter()
            .filter(|span| span.role == role)
            .map(|span| &line.text[span.start..span.end])
            .collect()
    }

    #[test]
    fn highlights_severity_and_metadata() {
        let line = highlighter()
            .highlight_line("2026-08-21T12:30:45Z WARN api[42]: request failed after 15ms");
        assert!(styled_text(&line, LogStyleRole::Warning).contains(&"WARN"));
        assert!(styled_text(&line, LogStyleRole::Metadata).contains(&"2026-08-21"));
        assert!(styled_text(&line, LogStyleRole::Identity).contains(&"api"));
    }

    #[test]
    fn structured_modules_win_over_generic_rules() {
        let line = highlighter()
            .highlight_line(r#"{"level":"error","message":"failed","status":500,"ok":false}"#);
        assert!(styled_text(&line, LogStyleRole::Key).contains(&"\"level\""));
        assert!(styled_text(&line, LogStyleRole::Numeric).contains(&"500"));
        assert!(
            styled_text(&line, LogStyleRole::Error)
                .iter()
                .any(|text| text.contains("error"))
        );
    }

    #[test]
    fn supports_logfmt_and_framework_lines() {
        let logfmt = highlighter().highlight_line("level=info method=GET status=200 duration=12ms");
        assert!(styled_text(&logfmt, LogStyleRole::Info).contains(&"info"));
        assert!(styled_text(&logfmt, LogStyleRole::Numeric).contains(&"200"));

        let python = highlighter()
            .highlight_line("2026-08-21 12:30:45,123 - worker.api - ERROR - request failed");
        assert!(styled_text(&python, LogStyleRole::Error).contains(&"ERROR"));
    }

    #[test]
    fn strips_terminal_sequences_and_preserves_unicode() {
        let line = highlighter().highlight_line("\u{1b}[31mERRO café 🚀 42\u{1b}[0m\u{07}");
        assert_eq!(line.text, "ERRO café 🚀 42");
        for span in &line.spans {
            assert!(line.text.is_char_boundary(span.start));
            assert!(line.text.is_char_boundary(span.end));
        }
    }

    #[test]
    fn long_lines_are_sanitized_but_not_classified() {
        let input = format!("ERROR {}", "x".repeat(MAX_HIGHLIGHT_BYTES));
        let line = highlighter().highlight_line(&input);
        assert_eq!(line.text, input);
        assert!(line.spans.is_empty());
    }

    #[test]
    fn disabled_highlighter_falls_back_to_plain_sanitized_text() {
        let line = LogHighlighter::disabled().highlight_line("\u{1b}[32mplain\u{1b}[0m");
        assert_eq!(line.text, "plain");
        assert!(line.spans.is_empty());
    }
}
