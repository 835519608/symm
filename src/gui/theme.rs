use egui::{
    Color32, Context, FontData, FontDefinitions, FontFamily, Frame, Margin, Rounding, Stroke,
    Visuals,
};

pub const BG_WORKSPACE: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF7);
pub const BG_PANEL: Color32 = Color32::WHITE;
pub const BORDER: Color32 = Color32::from_rgb(0xE0, 0xE0, 0xE0);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1A);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);
pub const ACCENT: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
pub const CODE_BG: Color32 = Color32::from_rgb(0xF3, 0xF4, 0xF6);

pub const RADIUS: u8 = 6;
pub const SPACING: f32 = 8.0;
pub const CARD_PADDING: f32 = 14.0;

pub fn apply(ctx: &Context) {
    setup_fonts(ctx);

    let mut visuals = Visuals::light();
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_WORKSPACE;
    visuals.extreme_bg_color = BG_WORKSPACE;
    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.inactive.bg_fill = BG_PANEL;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0xF0, 0xF0, 0xF2);
    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(37, 99, 235, 40);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.fg_stroke.color = TEXT_PRIMARY;
    visuals.widgets.inactive.fg_stroke.color = TEXT_PRIMARY;
    visuals.window_rounding = Rounding::ZERO;
    visuals.menu_rounding = Rounding::ZERO;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(SPACING, SPACING);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    ctx.set_style(style);
}

fn setup_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    if let Some(bytes) = load_system_cjk_font() {
        fonts
            .font_data
            .insert("cjk".to_owned(), FontData::from_owned(bytes).into());
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, "cjk".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}

fn load_system_cjk_font() -> Option<Vec<u8>> {
    let candidates: &[&str] = if cfg!(windows) {
        &[
            r"C:\Windows\Fonts\msyh.ttc",
            r"C:\Windows\Fonts\msyhbd.ttc",
            r"C:\Windows\Fonts\simhei.ttf",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        ]
    };

    for path in candidates {
        if let Ok(bytes) = std::fs::read(path)
            && !bytes.is_empty()
        {
            return Some(bytes);
        }
    }
    None
}

/// 白底卡片 / 面板常用边框样式。
pub fn card_frame() -> Frame {
    Frame::none()
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .rounding(Rounding::same(f32::from(RADIUS)))
}

/// 顶栏 / 侧栏 / 底栏：贴边无圆角，避免与中央区之间露出背景缝隙。
pub fn panel_frame() -> Frame {
    Frame::none()
        .fill(BG_PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .rounding(Rounding::ZERO)
        .inner_margin(Margin::symmetric(12.0, 8.0))
}

pub fn status_color(status: crate::domain::model::LinkStatus) -> Color32 {
    use crate::domain::model::LinkStatus;
    match status {
        LinkStatus::Ok => Color32::from_rgb(0x16, 0xA3, 0x4A),
        LinkStatus::Broken => Color32::from_rgb(0xEA, 0x58, 0x0C),
        LinkStatus::Missing => Color32::from_rgb(0xDC, 0x26, 0x26),
        LinkStatus::Stale => Color32::from_rgb(0x6B, 0x72, 0x80),
        LinkStatus::Drift => Color32::from_rgb(0xCA, 0x8A, 0x04),
    }
}
