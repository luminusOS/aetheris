mod app;

use relm4::RelmApp;

pub const APP_ID: &str = "org.luminusos.Aetheris";

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aetheris=info,aetheris_kube=info".into()),
        )
        .init();

    let app = RelmApp::new(APP_ID);
    app.run::<app::App>(());
}
