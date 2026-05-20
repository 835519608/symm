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

    /// 顶栏主题切换按钮图标：跟随系统 / 浅色 / 深色。
    pub fn toggle_icon(self) -> crate::gui::widgets::Icon {
        use crate::gui::widgets::Icon;
        match self {
            ThemeMode::System => Icon::Monitor,
            ThemeMode::Light => Icon::Sun,
            ThemeMode::Dark => Icon::Moon,
        }
    }
}

pub const ACCENT: Color32 = Color32::from_rgb(0x4F, 0x46, 0xE5);
pub const RADIUS: u8 = 12;
pub const CONTROL_RADIUS: f32 = 8.0;

pub fn control_rounding() -> Rounding {
    Rounding::same(CONTROL_RADIUS)
}
pub const SPACING: f32 = 12.0;
pub const CARD_PADDING: f32 = 18.0;

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

    let workspace = workspace_color(dark);
    let panel = panel_color(dark);
    let hover = control_hover(dark);
    let border = divider_color(dark);

    visuals.panel_fill = panel;
    visuals.window_fill = workspace;
    visuals.extreme_bg_color = workspace;
    visuals.faint_bg_color = hover;
    visuals.widgets.noninteractive.bg_fill = panel;
    visuals.widgets.inactive.bg_fill = input_fill(dark);
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.active.bg_fill = hover;
    visuals.widgets.open.bg_fill = hover;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(if dark { 0.35 } else { 0.18 });
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text_primary(dark));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text_primary(dark));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, text_primary(dark));
    visuals.widgets.active.fg_stroke = Stroke::new(1.2, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = Color32::from_rgb(0xF5, 0x9E, 0x0B);
    visuals.error_fg_color = Color32::from_rgb(0xEF, 0x44, 0x44);
    visuals.window_rounding = Rounding::ZERO;
    visuals.menu_rounding = control_rounding();
    visuals.window_stroke = Stroke::NONE;
    visuals.panel_fill = panel;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(SPACING, SPACING);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.interact_size = egui::vec2(36.0, 32.0);
    let r = control_rounding();
    style.visuals.widgets.inactive.rounding = r;
    style.visuals.widgets.hovered.rounding = r;
    style.visuals.widgets.active.rounding = r;
    ctx.set_style(style);

    let _ = border;
}

pub fn workspace_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x0C, 0x0F, 0x14)
    } else {
        Color32::from_rgb(0xF1, 0xF5, 0xF9)
    }
}

pub fn panel_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x14, 0x17, 0x1E)
    } else {
        Color32::WHITE
    }
}

pub fn divider_color(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x26, 0x2B, 0x36)
    } else {
        Color32::from_rgb(0xE2, 0xE8, 0xF0)
    }
}

pub fn control_hover(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x1F, 0x24, 0x2E)
    } else {
        Color32::from_rgb(0xF8, 0xFA, 0xFC)
    }
}

pub fn input_fill(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x0C, 0x0F, 0x14)
    } else {
        Color32::from_rgb(0xF8, 0xFA, 0xFC)
    }
}

pub fn text_primary(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0xF1, 0xF5, 0xF9)
    } else {
        Color32::from_rgb(0x0F, 0x17, 0x2A)
    }
}

pub fn text_secondary(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x94, 0xA3, 0xB8)
    } else {
        Color32::from_rgb(0x64, 0x74, 0x8B)
    }
}

pub fn text_muted(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(0x64, 0x74, 0x8B)
    } else {
        Color32::from_rgb(0x94, 0xA3, 0xB8)
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

pub fn divider_stroke(ui: &egui::Ui) -> Stroke {
    Stroke::new(1.0, divider_color(is_dark_ui(ui)))
}

pub fn input_stroke(ui: &egui::Ui, focused: bool) -> Stroke {
    if focused {
        Stroke::new(1.5, ACCENT)
    } else {
        Stroke::new(1.0, divider_color(is_dark_ui(ui)))
    }
}

/// 顶栏：无外边缝，仅底部分割线。
pub fn top_bar_frame(dark: bool) -> Frame {
    Frame::none()
        .fill(panel_color(dark))
        .inner_margin(Margin::symmetric(16.0, 10.0))
}

/// 侧栏：与主区无缝，仅右侧细分隔。
pub fn sidebar_frame(dark: bool) -> Frame {
    Frame::none()
        .fill(panel_color(dark))
        .inner_margin(Margin::symmetric(12.0, 10.0))
}

/// 底栏：顶部分隔线。
pub fn footer_frame(dark: bool) -> Frame {
    Frame::none()
        .fill(panel_color(dark))
        .inner_margin(Margin::symmetric(16.0, 6.0))
}

pub fn card_frame(ui: &egui::Ui) -> Frame {
    card_frame_for_dark(is_dark_ui(ui))
}

pub fn card_frame_for_dark(dark: bool) -> Frame {
    Frame::none()
        .fill(panel_color(dark))
        .stroke(Stroke::new(1.0, divider_color(dark)))
        .rounding(Rounding::same(f32::from(RADIUS)))
        .inner_margin(Margin::same(CARD_PADDING))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 1.0),
            blur: if dark { 12.0 } else { 16.0 },
            spread: 0.0,
            color: Color32::from_black_alpha(if dark { 40 } else { 8 }),
        })
}

pub fn status_color(status: crate::domain::model::LinkStatus) -> Color32 {
    use crate::domain::model::LinkStatus;
    match status {
        LinkStatus::Ok => Color32::from_rgb(0x10, 0xB9, 0x81),
        LinkStatus::Broken => Color32::from_rgb(0xF9, 0x73, 0x16),
        LinkStatus::Missing => Color32::from_rgb(0xEF, 0x44, 0x44),
        LinkStatus::Stale => Color32::from_rgb(0x94, 0xA3, 0xB8),
        LinkStatus::Drift => Color32::from_rgb(0xEA, 0xB3, 0x08),
    }
}

/// 在面板底/顶绘制 1px 分隔线（替代默认 panel 缝隙）。
pub fn paint_hairline(ui: &egui::Ui, bottom: bool) {
    let rect = ui.max_rect();
    let y = if bottom { rect.bottom() } else { rect.top() };
    ui.painter().hline(rect.x_range(), y, divider_stroke(ui));
}

/// 侧栏右缘竖线。
pub fn paint_sidebar_edge(ui: &egui::Ui) {
    let rect = ui.max_rect();
    ui.painter()
        .vline(rect.right(), rect.y_range(), divider_stroke(ui));
}

fn setup_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    if let Some((name, data)) = load_cjk_font() {
        fonts.font_data.insert(name, data.into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "cjk".to_owned());
    }
    ctx.set_fonts(fonts);
}

/// 返回 `(font_id, FontData)`；优先 `SYMM_FONT_PATH`，其次系统路径（含 WSL 挂载的 Windows 字体）。
fn load_cjk_font() -> Option<(String, FontData)> {
    if let Ok(path) = std::env::var("SYMM_FONT_PATH") {
        let path = path.trim();
        if !path.is_empty()
            && let Some(data) = read_font_file(path)
        {
            return Some(("cjk".to_owned(), data));
        }
    }

    for path in cjk_font_candidates() {
        if let Some(data) = read_font_file(path) {
            return Some(("cjk".to_owned(), data));
        }
    }
    None
}

fn read_font_file(path: &str) -> Option<FontData> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let mut data = FontData::from_owned(bytes);
    if path.ends_with(".ttc") || path.ends_with(".TTC") {
        data.index = 0;
    }
    Some(data)
}

fn cjk_font_candidates() -> Vec<&'static str> {
    let mut paths: Vec<&'static str> = Vec::new();

    if cfg!(windows) {
        paths.extend([
            r"C:\Windows\Fonts\simhei.ttf",
            r"C:\Windows\Fonts\msyh.ttc",
            r"C:\Windows\Fonts\msyhbd.ttc",
        ]);
    } else if cfg!(target_os = "macos") {
        paths.extend([
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]);
    } else {
        // Linux / WSL：优先用宿主 Windows 字体（无需 apt 安装）
        paths.extend([
            "/mnt/c/Windows/Fonts/msyh.ttc",
            "/mnt/c/Windows/Fonts/msyhbd.ttc",
            "/mnt/c/Windows/Fonts/simhei.ttf",
            "/mnt/c/Windows/Fonts/simsun.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        ]);
    }

    paths
}
