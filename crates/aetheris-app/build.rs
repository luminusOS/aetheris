fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=aetheris.rc");
        println!("cargo:rerun-if-changed=../../data/icons/windows/aetheris.ico");
        embed_resource::compile("aetheris.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
