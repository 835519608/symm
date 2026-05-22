use crate::domain::gui_settings::sanitize_font_size_pt;
use egui::{Context, FontId, RichText, TextStyle, Ui};
use std::sync::Arc;

/// 由正文字号推导的全局排版；DPI 仍由 `pixels_per_point` 处理。
#[derive(Debug, Clone, Copy)]
pub struct UiTypography {
    pub body: f32,
    pub scale: f32,
    pub small: f32,
    pub button: f32,
    pub heading: f32,
    pub section: f32,
    pub detail_title: f32,
    pub icon: f32,
    pub field_row_h: f32,
    pub btn_h: f32,
    pub icon_btn: egui::Vec2,
    pub interact: egui::Vec2,
}

impl UiTypography {
    /// `body_pt` 为设置里的正文字号（px）。
    pub fn from_body_pt(body_pt: f32) -> Self {
        let body = sanitize_font_size_pt(body_pt);
        let scale = body / 14.0;
        Self {
            body,
            scale,
            small: 11.0 * scale,
            button: 13.0 * scale,
            heading: 20.0 * scale,
            section: 15.0 * scale,
            detail_title: 18.0 * scale,
            icon: 14.0 * scale,
            field_row_h: 34.0 * scale,
            btn_h: 34.0 * scale,
            icon_btn: egui::vec2(36.0 * scale, 32.0 * scale),
            interact: egui::vec2(36.0 * scale, 32.0 * scale),
        }
    }

    pub fn button_font(&self) -> FontId {
        FontId::proportional(self.button)
    }
}

fn typo_id() -> egui::Id {
    egui::Id::new("symm_typography")
}

pub fn set_ctx_typography(ctx: &Context, typo: UiTypography) {
    ctx.data_mut(|d| d.insert_temp(typo_id(), typo));
}

pub fn typography_from_ui(ui: &Ui) -> UiTypography {
    ui.ctx()
        .data(|d| d.get_temp::<UiTypography>(typo_id()))
        .unwrap_or_else(|| UiTypography::from_body_pt(14.0))
}

fn ts_section() -> TextStyle {
    TextStyle::Name(Arc::from("Section"))
}

fn ts_detail_title() -> TextStyle {
    TextStyle::Name(Arc::from("DetailTitle"))
}

pub fn apply_text_styles(ctx: &Context, typo: &UiTypography) {
    set_ctx_typography(ctx, *typo);

    let mut style = (*ctx.style()).clone();
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(typo.small));
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(typo.body));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(typo.button));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(typo.heading));
    style
        .text_styles
        .insert(ts_section(), FontId::proportional(typo.section));
    style
        .text_styles
        .insert(ts_detail_title(), FontId::proportional(typo.detail_title));

    let s = typo.scale;
    style.spacing.item_spacing = egui::vec2(10.0 * s, 8.0 * s);
    style.spacing.button_padding = egui::vec2(12.0 * s, 7.0 * s);
    style.spacing.interact_size = typo.interact;
    ctx.set_style(style);
}

pub fn rich_heading(text: &str, color: egui::Color32) -> RichText {
    RichText::new(text)
        .text_style(TextStyle::Heading)
        .strong()
        .color(color)
}

pub fn rich_section(text: &str, color: egui::Color32) -> RichText {
    RichText::new(text)
        .text_style(ts_section())
        .strong()
        .color(color)
}

pub fn rich_detail_title(text: &str, color: egui::Color32) -> RichText {
    RichText::new(text)
        .text_style(ts_detail_title())
        .strong()
        .color(color)
}

pub fn rich_body(text: &str, color: egui::Color32) -> RichText {
    RichText::new(text).text_style(TextStyle::Body).color(color)
}

pub fn rich_body_muted(text: &str, color: egui::Color32) -> RichText {
    RichText::new(text).text_style(TextStyle::Body).color(color)
}

pub fn rich_small(text: &str, color: egui::Color32) -> RichText {
    RichText::new(text)
        .text_style(TextStyle::Small)
        .color(color)
}
