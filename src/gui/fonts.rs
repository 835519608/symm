//! GUI 字体：内嵌 Noto Sans SC（中文）+ egui-phosphor（Phosphor Regular 图标），不扫描系统字体。

use egui::{FontData, FontDefinitions, FontFamily, FontId};
use egui_phosphor::Variant;

pub const CJK_FONT: &str = "cjk";
pub const ICON_FONT: &str = "phosphor";

pub fn embedded_cjk_data() -> FontData {
    let mut data =
        FontData::from_owned(include_bytes!("../../assets/fonts/NotoSansSC-Regular.otf").to_vec());
    data.index = 0;
    data
}

pub fn apply_cjk_fonts(definitions: &mut FontDefinitions) {
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        definitions
            .families
            .entry(family)
            .or_default()
            .insert(0, CJK_FONT.to_owned());
    }
}

/// 注册内嵌 CJK 与 Phosphor 图标字体（图标在 `Proportional` 族中作为 fallback）。
pub fn setup_all_fonts(definitions: &mut FontDefinitions) {
    definitions
        .font_data
        .insert(CJK_FONT.to_owned(), embedded_cjk_data().into());
    apply_cjk_fonts(definitions);
    egui_phosphor::add_to_fonts(definitions, Variant::Regular);
    definitions
        .families
        .entry(FontFamily::Name(ICON_FONT.into()))
        .or_default()
        .push(ICON_FONT.to_owned());
}

pub fn icon_font_id(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(ICON_FONT.into()))
}
