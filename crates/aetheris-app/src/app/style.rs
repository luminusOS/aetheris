use super::*;

const APP_STYLE_RESOURCE: &str = "aetheris/style.css";

pub(super) fn load_app_css() {
    add_app_icon_search_paths();

    if let Some(path) = app_style_path() {
        if let Err(error) = relm4::set_global_css_from_file(&path) {
            tracing::warn!(
                "Unable to load application stylesheet {}: {error}",
                path.display()
            );
        } else {
            return;
        }
    }
    tracing::warn!("Unable to find application stylesheet {APP_STYLE_RESOURCE}");
}

fn add_app_icon_search_paths() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };

    let theme = gtk::IconTheme::for_display(&display);
    let icon_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/icons");
    let action_icons = icon_root.join("hicolor/scalable/actions");
    theme.add_search_path(icon_root);
    theme.add_search_path(action_icons);
}

fn app_style_path() -> Option<std::path::PathBuf> {
    app_style_candidates()
        .into_iter()
        .find(|path| path.is_file())
}

fn app_style_candidates() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        paths.push(std::path::PathBuf::from(data_home).join(APP_STYLE_RESOURCE));
    }

    if let Some(data_dirs) = std::env::var_os("XDG_DATA_DIRS") {
        paths.extend(std::env::split_paths(&data_dirs).map(|path| path.join(APP_STYLE_RESOURCE)));
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(bin_dir) = exe.parent()
    {
        paths.push(bin_dir.join("../share").join(APP_STYLE_RESOURCE));
        paths.push(bin_dir.join("share").join(APP_STYLE_RESOURCE));
        paths.push(bin_dir.join("../style.css"));
    }

    if let Some(app_dir) = std::env::var_os("APPDIR") {
        let app_dir = std::path::PathBuf::from(app_dir);
        paths.push(app_dir.join("usr/share").join(APP_STYLE_RESOURCE));
        paths.push(app_dir.join("usr/share/style.css"));
        paths.push(app_dir.join("style.css"));
    }

    paths.push(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/style.css"));
    paths.push(std::path::PathBuf::from("data/style.css"));
    paths
}
