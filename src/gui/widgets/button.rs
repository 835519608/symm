use crate::gui::fonts::icon_font_id;
use crate::gui::icons::Icon;
use crate::gui::theme::{self, typography_from_ui};
use egui::{Button, Color32, Response, RichText, Stroke, Ui, Vec2, WidgetInfo, WidgetType};

const BTN_MIN_W: f32 = 96.0;

fn widget_text(icon: Option<Icon>, label: &str, typo: &theme::UiTypography) -> RichText {
    let caption = match icon {
        Some(i) if label.is_empty() => i.glyph().to_string(),
        Some(i) => format!("{}  {label}", i.glyph()),
        None => label.to_string(),
    };
    let font_id = if icon.is_some() && label.is_empty() {
        icon_font_id(typo.icon)
    } else {
        typo.button_font()
    };
    RichText::new(caption).font(font_id)
}

pub(crate) fn default_min_size(
    typo: &theme::UiTypography,
    icon: Option<Icon>,
    label: &str,
) -> Vec2 {
    if label.is_empty() && icon.is_some() {
        return typo.icon_btn;
    }
    Vec2::new(BTN_MIN_W * typo.scale, typo.btn_h)
}

/// 链式配置按钮（egui [`Button`] + 主题 `Visuals`）。
pub struct UiButton<'a> {
    ui: &'a mut Ui,
    icon: Option<Icon>,
    label: &'a str,
    tip: &'a str,
    enabled: bool,
    danger: bool,
}

impl<'a> UiButton<'a> {
    pub fn new(ui: &'a mut Ui) -> Self {
        Self {
            ui,
            icon: None,
            label: "",
            tip: "",
            enabled: true,
            danger: false,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }

    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = tip;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    fn button(self) -> (Button<'a>, &'a mut Ui, &'a str, Option<&'a str>, bool, bool) {
        let typo = typography_from_ui(self.ui);
        let dark = if self.danger {
            theme::palette_from_ui(self.ui).dark
        } else {
            false
        };
        let text = widget_text(self.icon, self.label, &typo);
        let size = default_min_size(&typo, self.icon, self.label);
        let button = Button::new(text).min_size(size);
        let semantic_label = if self.label.is_empty() && self.icon.is_some() && !self.tip.is_empty()
        {
            Some(self.tip)
        } else {
            None
        };
        (button, self.ui, self.tip, semantic_label, self.danger, dark)
    }

    pub fn show(self) -> Response {
        let enabled = self.enabled;
        let (button, ui, tip, semantic_label, danger, dark) = self.button();
        let semantic_enabled = enabled && ui.is_enabled();
        let mut resp = if danger && semantic_enabled {
            ui.scope(|ui| {
                apply_danger_visuals(ui, dark);
                ui.add_enabled(enabled, button)
            })
            .inner
        } else {
            ui.add_enabled(enabled, button)
        };
        if let Some(label) = semantic_label {
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, semantic_enabled, label));
        }
        if !tip.is_empty() {
            resp = resp.on_hover_text(tip);
        }
        resp
    }

    pub fn show_sized(self, size: Vec2) -> Response {
        let enabled = self.enabled;
        let (button, ui, tip, semantic_label, danger, dark) = self.button();
        let semantic_enabled = enabled && ui.is_enabled();
        let mut resp = if danger && semantic_enabled {
            ui.scope(|ui| {
                apply_danger_visuals(ui, dark);
                ui.add_enabled_ui(enabled, |ui| ui.add_sized(size, button.min_size(size)))
                    .inner
            })
            .inner
        } else {
            ui.add_enabled_ui(enabled, |ui| ui.add_sized(size, button.min_size(size)))
                .inner
        };
        if let Some(label) = semantic_label {
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, semantic_enabled, label));
        }
        if !tip.is_empty() {
            resp = resp.on_hover_text(tip);
        }
        resp
    }
}

pub fn button(ui: &mut Ui) -> UiButton<'_> {
    UiButton::new(ui)
}

#[derive(Clone, Copy)]
struct DangerVisuals {
    text: Color32,
    text_hover: Color32,
    fill: Color32,
    fill_hover: Color32,
    fill_active: Color32,
    stroke: Color32,
    stroke_hover: Color32,
}

fn danger_visuals(dark: bool) -> DangerVisuals {
    if dark {
        DangerVisuals {
            text: Color32::from_rgb(0xFC, 0xA5, 0xA5),
            text_hover: Color32::from_rgb(0xFE, 0xCA, 0xCA),
            fill: Color32::from_rgb(0x3B, 0x1D, 0x25),
            fill_hover: Color32::from_rgb(0x4C, 0x1D, 0x25),
            fill_active: Color32::from_rgb(0x7F, 0x1D, 0x1D),
            stroke: Color32::from_rgb(0x7F, 0x1D, 0x1D),
            stroke_hover: Color32::from_rgb(0xEF, 0x44, 0x44),
        }
    } else {
        DangerVisuals {
            text: Color32::from_rgb(0xB9, 0x1C, 0x1C),
            text_hover: Color32::from_rgb(0x99, 0x1B, 0x1B),
            fill: Color32::from_rgb(0xFE, 0xE2, 0xE2),
            fill_hover: Color32::from_rgb(0xFE, 0xCA, 0xCA),
            fill_active: Color32::from_rgb(0xFC, 0xA5, 0xA5),
            stroke: Color32::from_rgb(0xFC, 0xA5, 0xA5),
            stroke_hover: Color32::from_rgb(0xDC, 0x26, 0x26),
        }
    }
}

fn apply_danger_visuals(ui: &mut Ui, dark: bool) {
    let p = danger_visuals(dark);
    let widgets = &mut ui.style_mut().visuals.widgets;

    widgets.inactive.weak_bg_fill = p.fill;
    widgets.inactive.bg_fill = p.fill;
    widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    widgets.inactive.bg_stroke = Stroke::new(1.0, p.stroke);

    widgets.hovered.weak_bg_fill = p.fill_hover;
    widgets.hovered.bg_fill = p.fill_hover;
    widgets.hovered.fg_stroke = Stroke::new(1.0, p.text_hover);
    widgets.hovered.bg_stroke = Stroke::new(1.3, p.stroke_hover);

    widgets.active.weak_bg_fill = p.fill_active;
    widgets.active.bg_fill = p.fill_active;
    widgets.active.fg_stroke = Stroke::new(1.0, p.text_hover);
    widgets.active.bg_stroke = Stroke::new(1.3, p.stroke_hover);

    widgets.open.weak_bg_fill = p.fill_hover;
    widgets.open.bg_fill = p.fill_hover;
    widgets.open.fg_stroke = Stroke::new(1.0, p.text_hover);
    widgets.open.bg_stroke = Stroke::new(1.3, p.stroke_hover);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::gui_settings::FONT_SIZE_PT_DEFAULT;

    #[test]
    fn chinese_text_button_has_room_for_label_and_padding() {
        let typo = theme::UiTypography::from_body_pt(FONT_SIZE_PT_DEFAULT);
        let size = default_min_size(&typo, None, "关闭");

        assert!(
            size.x >= 96.0 * typo.scale,
            "short CJK button labels still need enough horizontal padding, got {size:?}"
        );
    }
}
