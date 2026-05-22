fn main() {
    #[cfg(windows)]
    {
        embed_resource::compile("symm-manifest.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }

    let font_path = std::path::Path::new("assets/fonts/NotoSansSC-Regular.otf");
    println!("cargo:rerun-if-changed=assets/fonts/NotoSansSC-Regular.otf");

    if std::env::var("CARGO_FEATURE_GUI").is_ok() && !font_path.exists() {
        panic!(
            "GUI 构建需要 assets/fonts/NotoSansSC-Regular.otf。\n\
             请运行: scripts/fetch-gui-font.sh"
        );
    }
}
