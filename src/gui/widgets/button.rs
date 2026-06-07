use crate::gui::fonts::icon_font_id;
use crate::gui::icons::Icon;
use crate::gui::theme::{self, typography_from_ui};
use egui::{Button, Response, RichText, Ui, Vec2, WidgetInfo, WidgetType};

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
}

impl<'a> UiButton<'a> {
    pub fn new(ui: &'a mut Ui) -> Self {
        Self {
            ui,
            icon: None,
            label: "",
            tip: "",
            enabled: true,
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

    fn button(self) -> (Button<'a>, &'a mut Ui, &'a str, Option<&'a str>) {
        let typo = typography_from_ui(self.ui);
        let text = widget_text(self.icon, self.label, &typo);
        let size = default_min_size(&typo, self.icon, self.label);
        let semantic_label = if self.label.is_empty() && self.icon.is_some() && !self.tip.is_empty()
        {
            Some(self.tip)
        } else {
            None
        };
        (
            Button::new(text).min_size(size),
            self.ui,
            self.tip,
            semantic_label,
        )
    }

    pub fn show(self) -> Response {
        let enabled = self.enabled;
        let (button, ui, tip, semantic_label) = self.button();
        let semantic_enabled = enabled && ui.is_enabled();
        let mut resp = ui.add_enabled(enabled, button);
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
        let (button, ui, tip, semantic_label) = self.button();
        let semantic_enabled = enabled && ui.is_enabled();
        let mut resp = ui
            .add_enabled_ui(enabled, |ui| ui.add_sized(size, button.min_size(size)))
            .inner;
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
