#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use relm4::RelmApp;

pub const APP_ID: &str = "org.luminusos.Aetheris";
pub const GETTEXT_PACKAGE: &str = APP_ID;

#[cfg(windows)]
fn configure_bundled_runtime() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(root) = exe.parent().and_then(|bin| bin.parent()) else {
        return;
    };

    let share = root.join("share");
    let vars = [
        ("XDG_DATA_DIRS", share.clone()),
        (
            "GSETTINGS_SCHEMA_DIR",
            share.join("glib-2.0").join("schemas"),
        ),
        (
            "GDK_PIXBUF_MODULEDIR",
            root.join("lib")
                .join("gdk-pixbuf-2.0")
                .join("2.10.0")
                .join("loaders"),
        ),
        (
            "GIO_MODULE_DIR",
            root.join("lib").join("gio").join("modules"),
        ),
        (
            "SSL_CERT_FILE",
            root.join("ssl").join("certs").join("ca-bundle.crt"),
        ),
    ];

    for (key, value) in vars {
        // SAFETY: called once, single-threaded, before GTK/GLib or any other
        // thread reads the environment.
        unsafe { std::env::set_var(key, value) };
    }
}

fn main() {
    #[cfg(windows)]
    configure_bundled_runtime();

    app::i18n::init();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aetheris=info,aetheris_kube=info".into()),
        )
        .init();

    let app = RelmApp::new(APP_ID);
    app.run::<app::App>(());
}
