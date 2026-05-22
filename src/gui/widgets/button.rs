use crate::gui::fonts::icon_font_id;
use crate::gui::icons::Icon;
use crate::gui::theme::{self, typography_from_ui};
use egui::{Button, Response, RichText, Ui, Vec2};

const BTN_MIN_W: f32 = 72.0;

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

fn default_min_size(typo: &theme::UiTypography, icon: Option<Icon>, label: &str) -> Vec2 {
    if label.is_empty() && icon.is_some() {
        return typo.icon_btn;
    }
    let text_len = label.len().max(4) as f32;
    Vec2::new(
        (text_len * 7.5 * typo.scale + 36.0 * typo.scale).max(BTN_MIN_W * typo.scale),
        typo.btn_h,
    )
}

/// 链式配置按钮（egui [`Button`] + 主题 `Visuals`）。
pub struct UiButton<'a> {
    ui: &'a mut Ui,
    icon: Option<Icon>,
    label: &'a str,
    tip: &'a str,
}

impl<'a> UiButton<'a> {
    pub fn new(ui: &'a mut Ui) -> Self {
        Self {
            ui,
            icon: None,
            label: "",
            tip: "",
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

    pub fn show(self) -> Response {
        let typo = typography_from_ui(self.ui);
        let text = widget_text(self.icon, self.label, &typo);
        let size = default_min_size(&typo, self.icon, self.label);
        let mut resp = self.ui.add(Button::new(text).min_size(size));
        if !self.tip.is_empty() {
            resp = resp.on_hover_text(self.tip);
        }
        resp
    }
}

pub fn button(ui: &mut Ui) -> UiButton<'_> {
    UiButton::new(ui)
}
