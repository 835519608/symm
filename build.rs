fn main() {
    #[cfg(windows)]
    {
        embed_resource::compile("symm-manifest.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
