fn main() {
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=OPENCONNECT_NO_PKG_CONFIG");

    if std::env::var_os("CARGO_FEATURE_SYSTEM_LIBOPENCONNECT").is_none() {
        return;
    }

    if std::env::var_os("OPENCONNECT_NO_PKG_CONFIG").is_some() {
        println!("cargo:rustc-link-lib=openconnect");
        return;
    }

    pkg_config::Config::new()
        .atleast_version("8.20")
        .probe("openconnect")
        .expect("system-libopenconnect requires libopenconnect development files");
}
