use crate::domain::gui_settings::ThemeMode;
use egui::{
    Color32, Context, FontData, FontDefinitions, FontFamily, Frame, Margin, Rounding, Stroke,
    Visuals,
};

pub type ThemePreference = ThemeMode;

impl ThemeMode {
    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::System => "跟随系统",
            ThemeMode::Light => "浅色",
            ThemeMode::Dark => "深色",
        }
    }

    pub fn is_dark(self) -> bool {
        match self {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            ThemeMode::System => system_prefers_dark(),
        }
    }
}

pub const ACCENT: Color32 = Color32::from_rgb(0x3B, 0x82, 0xF6);
pub const RADIUS: u8 = 10;
pub const SPACING: f32 = 10.0;
pub const CARD_PADDING: f32 = 16.0;

pub fn system_prefers_dark() -> bool {
    match dark_light::detect() {
        Ok(dark_light::Mode::Dark) => true,
        Ok(dark_light::Mode::Light) => false,
        Ok(dark_light::Mode::Unspecified) | Err(_) => false,
    }
}

pub fn apply(ctx: &Context, preference: ThemePreference) {
    setup_fonts(ctx);
    let dark = preference.is_dark();
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    let workspace = if dark {
        Color32::from_rgb(0x0F, 0x11, 0x15)
    } else {
        Color32::from_rgb(0xF4, 0xF6, 0xFA)
    };
    let panel = if dark {
        Color32::from_rgb(0x18, 0x1B, 0x22)
    } else {
        Color32::WHITE
    };
    let border = if dark {
        Color32::from_rgb(0x2E, 0x33, 0x3D)
    } else {
        Color32::from_rgb(0xE5, 0xE7, 0xEB)
    };
    let hover = if dark {
        Color32::from_rgb(0x25, 0x2A, 0x34)
    } else {
        Color32::from_rgb(0xF3, 0xF4, 0xF6)
    };

    visuals.panel_fill = panel;
    visuals.window_fill = workspace;
    visuals.extreme_bg_color = workspace;
    visuals.faint_bg_color = hover;
    visuals.widgets.noninteractive.bg_fill = panel;
    visuals.widgets.inactive.bg_fill = panel;
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.active.bg_fill = hover;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.28);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text_primary(dark));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text_primary(dark));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, text_primary(dark));
    visuals.widgets.active.fg_stroke = Stroke::new(1.2, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = Color32::from_rgb(0xF5, 0x9E, 0x0B);
    visuals.error_fg_color = Color32::from_rgb(0xEF, 0x44, 0x44);
    visuals.window_rounding = Rounding::ZERO;
    visuals.menu_rounding = Rounding::same(6.0);
    visuals.window_stroke = Stroke::new(1.0, border);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(SPACING, SPACING);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.interact_size = egui::vec2(36.0, 28.0);
    ctx.set_style(style);
}

pub fn text_primary(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0xF3, 0xF4, 0xF6)
    } else {
        Color32::from_rgb(0x11, 0x18, 0x27)
    }
}

pub fn text_secondary(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x9C, 0xA3, 0xAF)
    } else {
        Color32::from_rgb(0x6B, 0x72, 0x80)
    }
}

pub fn is_dark_ui(ui: &egui::Ui) -> bool {
    ui.visuals().dark_mode
}

pub fn primary_text(ui: &egui::Ui) -> Color32 {
    text_primary(is_dark_ui(ui))
}

pub fn secondary_text(ui: &egui::Ui) -> Color32 {
    text_secondary(is_dark_ui(ui))
}

pub fn workspace_fill(ui: &egui::Ui) -> Color32 {
    ui.visuals().window_fill
}

pub fn panel_fill(ui: &egui::Ui) -> Color32 {
    ui.visuals().panel_fill
}

pub fn border_stroke(ui: &egui::Ui) -> Stroke {
    ui.visuals().window_stroke
}

pub fn card_frame(ui: &egui::Ui) -> Frame {
    Frame::none()
        .fill(panel_fill(ui))
        .stroke(border_stroke(ui))
        .rounding(Rounding::same(f32::from(RADIUS)))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: Color32::from_black_alpha(if is_dark_ui(ui) { 48 } else { 12 }),
        })
}

pub fn panel_frame(ui: &egui::Ui) -> Frame {
    Frame::none()
        .fill(panel_fill(ui))
        .stroke(border_stroke(ui))
        .rounding(Rounding::ZERO)
        .inner_margin(Margin::symmetric(12.0, 8.0))
}

pub fn status_color(status: crate::domain::model::LinkStatus) -> Color32 {
    use crate::domain::model::LinkStatus;
    match status {
        LinkStatus::Ok => Color32::from_rgb(0x22, 0xC5, 0x5E),
        LinkStatus::Broken => Color32::from_rgb(0xF9, 0x73, 0x16),
        LinkStatus::Missing => Color32::from_rgb(0xEF, 0x44, 0x44),
        LinkStatus::Stale => Color32::from_rgb(0x94, 0xA3, 0xB8),
        LinkStatus::Drift => Color32::from_rgb(0xEA, 0xB3, 0x08),
    }
}

fn setup_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    if let Some(bytes) = load_system_cjk_font() {
        fonts
            .font_data
            .insert("cjk".to_owned(), FontData::from_owned(bytes).into());
        // 仅 UI 正文字体挂 CJK；等宽字体保留 egui 默认（路径/命令用 ASCII 即可），减少 atlas 体积。
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "cjk".to_owned());
    }
    ctx.set_fonts(fonts);
}

fn load_system_cjk_font() -> Option<Vec<u8>> {
    let candidates: &[&str] = if cfg!(windows) {
        &[
            r"C:\Windows\Fonts\simhei.ttf",
            r"C:\Windows\Fonts\msyh.ttc",
            r"C:\Windows\Fonts\msyhbd.ttc",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
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
