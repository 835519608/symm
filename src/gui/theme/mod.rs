mod palette;
mod typography;

use crate::domain::gui_settings::{ColorScheme, ThemeMode, sanitize_font_size_pt};
use egui::{
    Color32, Context, FontDefinitions, Frame, Margin, Rounding, Stroke, Ui, Visuals, epaint::Shadow,
};
pub use palette::UiPalette;
pub use typography::{
    UiTypography, apply_text_styles, rich_body, rich_body_muted, rich_detail_title, rich_section,
    rich_small, typography_from_ui,
};

pub const SIDEBAR_PANEL_ID: &str = "sidebar";
pub const TOP_BAR_PANEL_ID: &str = "top_bar";
pub const FOOTER_PANEL_ID: &str = "footer";

pub const SIDEBAR_WIDTH_MIN: f32 = 280.0;
pub const SIDEBAR_DEFAULT_WIDTH: f32 = 280.0;

pub const RADIUS: f32 = 8.0;

pub type ThemePreference = ThemeMode;

fn palette_id() -> egui::Id {
    egui::Id::new("symm_active_palette")
}

impl ThemeMode {
    pub fn icon(self) -> crate::gui::icons::Icon {
        use crate::gui::icons::Icon;
        match self {
            ThemeMode::System => Icon::Monitor,
            ThemeMode::Light => Icon::Sun,
            ThemeMode::Dark => Icon::Moon,
        }
    }
}

impl ColorScheme {
    pub fn icon(self) -> crate::gui::icons::Icon {
        crate::gui::icons::Icon::Palette
    }
}

pub fn system_prefers_dark() -> bool {
    match dark_light::detect() {
        Ok(dark_light::Mode::Dark) => true,
        Ok(dark_light::Mode::Light) => false,
        Ok(dark_light::Mode::Unspecified) | Err(_) => false,
    }
}

pub fn resolve_dark(theme: ThemeMode) -> bool {
    match theme {
        ThemeMode::Dark => true,
        ThemeMode::Light => false,
        ThemeMode::System => system_prefers_dark(),
    }
}

pub fn resolve(theme: ThemeMode, scheme: ColorScheme) -> UiPalette {
    palette::for_scheme(scheme, resolve_dark(theme))
}

fn set_ctx_palette(ctx: &Context, palette: UiPalette) {
    ctx.data_mut(|d| d.insert_temp(palette_id(), palette));
}

pub fn rounding() -> Rounding {
    Rounding::same(RADIUS)
}

pub fn apply(ctx: &Context, theme: ThemeMode, scheme: ColorScheme, font_size_pt: f32) {
    let typo = UiTypography::from_body_pt(sanitize_font_size_pt(font_size_pt));
    apply_text_styles(ctx, &typo);
    let p = resolve(theme, scheme);
    set_ctx_palette(ctx, p);

    let mut visuals = if p.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    let r = rounding();
    visuals.dark_mode = p.dark;
    visuals.panel_fill = p.surface;
    visuals.window_fill = p.bg;
    visuals.extreme_bg_color = p.bg;
    visuals.faint_bg_color = p.surface_alt;
    visuals.override_text_color = Some(p.text);
    visuals.selection.bg_fill = p.accent_soft;
    visuals.selection.stroke = Stroke::new(1.0, p.accent);
    visuals.hyperlink_color = p.accent_text;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    // 顶栏 / 侧栏 / 底栏与主区之间的分隔线（egui Panel 默认绘制，读此 stroke）
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    visuals.widgets.inactive.bg_fill = p.surface;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    visuals.widgets.inactive.rounding = r;
    visuals.widgets.hovered.bg_fill = p.surface_hover;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text_hover);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent.gamma_multiply(0.45));
    visuals.widgets.hovered.rounding = r;
    visuals.widgets.active.bg_fill = p.surface_active;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, p.accent_text);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    visuals.widgets.active.rounding = r;
    visuals.widgets.open.bg_fill = p.surface_hover;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, p.text_hover);
    visuals.window_rounding = r;
    visuals.window_stroke = Stroke::new(1.0, p.border);
    visuals.window_shadow = Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 16.0,
        spread: 0.0,
        color: p.shadow,
    };
    visuals.popup_shadow = visuals.window_shadow;
    ctx.set_visuals(visuals);
}

pub fn status_color_for_palette(
    status: crate::domain::model::LinkStatus,
    p: &UiPalette,
) -> Color32 {
    status_color_for_darkness(status, p.dark)
}

fn status_color_for_darkness(status: crate::domain::model::LinkStatus, dark: bool) -> Color32 {
    use crate::domain::model::LinkStatus;
    match (dark, status) {
        (false, LinkStatus::Ok) => Color32::from_rgb(0x15, 0x80, 0x3D),
        (false, LinkStatus::Broken) => Color32::from_rgb(0xC2, 0x41, 0x0C),
        (false, LinkStatus::Missing) => Color32::from_rgb(0xDC, 0x26, 0x26),
        (false, LinkStatus::Stale) => Color32::from_rgb(0x47, 0x55, 0x69),
        (false, LinkStatus::Drift) => Color32::from_rgb(0xA1, 0x62, 0x07),
        (false, LinkStatus::Unknown) => Color32::from_rgb(0x6B, 0x72, 0x80),
        (true, LinkStatus::Ok) => Color32::from_rgb(0x4A, 0xDE, 0x80),
        (true, LinkStatus::Broken) => Color32::from_rgb(0xFB, 0x92, 0x24),
        (true, LinkStatus::Missing) => Color32::from_rgb(0xF8, 0x71, 0x71),
        (true, LinkStatus::Stale) => Color32::from_rgb(0xCB, 0xD5, 0xE1),
        (true, LinkStatus::Drift) => Color32::from_rgb(0xFD, 0xBA, 0x74),
        (true, LinkStatus::Unknown) => Color32::from_rgb(0xD1, 0xD5, 0xDB),
    }
}

pub fn gap_sm(ui: &Ui) -> f32 {
    ui.spacing().item_spacing.y * 0.75
}

pub fn gap(ui: &Ui) -> f32 {
    ui.spacing().item_spacing.y
}

pub fn gap_lg(ui: &Ui) -> f32 {
    ui.spacing().item_spacing.y * 1.5
}

/// 顶栏 / 侧栏 / 主区 / 底栏共用边距，避免 Frame 后左右列对不齐。
const PANEL_MARGIN_H: f32 = 16.0;
const PANEL_MARGIN_V: f32 = 10.0;

fn panel_frame(p: &UiPalette) -> Frame {
    Frame::none()
        .fill(p.surface)
        .inner_margin(Margin::symmetric(PANEL_MARGIN_H, PANEL_MARGIN_V))
}

pub fn top_bar_frame(p: &UiPalette) -> Frame {
    panel_frame(p)
}

pub fn footer_frame(p: &UiPalette) -> Frame {
    panel_frame(p)
}

pub fn paint_footer_separator(ctx: &Context, rect: egui::Rect, p: &UiPalette) {
    let px = ctx.pixels_per_point().recip();
    let y = (rect.top() / px).round() * px + px * 0.5;
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("footer_panel_separator"),
    ))
    .hline(
        egui::Rangef::new(rect.left(), rect.right()),
        y,
        Stroke::new(1.0, p.border),
    );
}

pub fn sidebar_frame(p: &UiPalette) -> Frame {
    panel_frame(p)
}

pub fn central_panel_frame(p: &UiPalette) -> Frame {
    panel_frame(p)
}

pub fn sidebar_max_width(ctx: &Context) -> f32 {
    (ctx.screen_rect().width() * 0.5).max(SIDEBAR_WIDTH_MIN)
}

/// 将侧栏持久化宽度设为指定值（设置里「应用」后生效）。
pub fn pin_side_panel_width(ctx: &Context, panel_id: &str, width: f32) {
    use egui::containers::panel::PanelState;
    let id = egui::Id::new(panel_id);
    let screen = ctx.screen_rect();
    let w = width.clamp(SIDEBAR_WIDTH_MIN, sidebar_max_width(ctx));
    let rect = if let Some(state) = PanelState::load(ctx, id) {
        let mut r = state.rect;
        r.set_width(w);
        r
    } else {
        egui::Rect::from_min_size(screen.left_top(), egui::vec2(w, screen.height()))
    };
    ctx.data_mut(|d| d.insert_persisted(id, PanelState { rect }));
}

pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    crate::gui::fonts::setup_all_fonts(&mut fonts);
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_max_width_uses_screen_width_not_remaining_available_width() {
        let ctx = Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            ..Default::default()
        };

        let _ = ctx.run(input, |ctx| {
            egui::TopBottomPanel::top("sidebar_max_width_test_top").show(ctx, |_| {});
            egui::SidePanel::left("sidebar_max_width_test_sidebar")
                .default_width(480.0)
                .show(ctx, |_| {});

            assert!(
                ctx.available_rect().width() < ctx.screen_rect().width(),
                "test should run after a panel has reduced available_rect"
            );
            assert_eq!(sidebar_max_width(ctx), 640.0);
        });
    }
}
