use egui::IconData;

/// 窗口标题栏 / 任务栏图标（`assets/icon.png` 建议 64×64，避免解码占用过多内存）。
pub fn viewport_icon() -> IconData {
    let bytes = include_bytes!("../../assets/icon.png");
    let img = image::load_from_memory(bytes).expect("assets/icon.png");
    let img = img.to_rgba8();
    let (width, height) = img.dimensions();
    IconData {
        rgba: img.into_raw(),
        width,
        height,
    }
}
