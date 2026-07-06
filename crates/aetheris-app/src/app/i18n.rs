use std::path::PathBuf;

use relm4::gtk::glib;

use crate::GETTEXT_PACKAGE;

pub(crate) fn init() {
    #[cfg(all(unix, not(target_os = "macos")))]
    bind_textdomain();
}

pub(crate) fn tr(message: &str) -> String {
    glib::dgettext(Some(GETTEXT_PACKAGE), message).to_string()
}

pub(crate) fn trn(singular: &str, plural: &str, n: u32) -> String {
    glib::dngettext(Some(GETTEXT_PACKAGE), singular, plural, n.into()).to_string()
}

pub(crate) fn tr_format(message: &str, replacements: &[(&str, String)]) -> String {
    let mut translated = tr(message);
    for (key, value) in replacements {
        translated = translated.replace(key, value);
    }
    translated
}

#[cfg(all(unix, not(target_os = "macos")))]
fn bind_textdomain() {
    let Some(locale_dir) = locale_dir() else {
        return;
    };
    let _ = gettextrs::bindtextdomain(GETTEXT_PACKAGE, locale_dir);
    let _ = gettextrs::bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8");
}

#[cfg(all(unix, not(target_os = "macos")))]
fn locale_dir() -> Option<PathBuf> {
    std::env::var_os("AETHERIS_LOCALEDIR")
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(installed_locale_dir)
        .or_else(source_locale_dir)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn installed_locale_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let prefix = exe.parent()?.parent()?;
    let locale_dir = prefix.join("share").join("locale");
    locale_dir.exists().then_some(locale_dir)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn source_locale_dir() -> Option<PathBuf> {
    let locale_dir = std::env::current_dir().ok()?.join("po");
    locale_dir.exists().then_some(locale_dir)
}
