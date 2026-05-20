use super::icons::{self, Icon};
use crate::gui::theme;
use egui::{Button, Color32, Frame, Margin, Response, RichText, Sense, Stroke, Ui, Vec2, Widget};

pub fn toolbar_button(ui: &mut Ui, icon: Icon, label: &str, tip: &str) -> Response {
    let dark = theme::is_dark_ui(ui);
    let desired = Vec2::new((label.len() as f32 * 6.5 + 44.0).clamp(72.0, 140.0), 32.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    let hovered = resp.hovered();

    let bg = if hovered {
        theme::control_hover(dark)
    } else {
        Color32::TRANSPARENT
    };
    let fg = if hovered {
        theme::text_primary(dark)
    } else {
        theme::text_secondary(dark)
    };

    ui.painter()
        .rect(rect, theme::control_rounding(), bg, Stroke::NONE);

    let icon_rect = egui::Rect::from_center_size(
        rect.left_center() + Vec2::new(18.0, 0.0),
        Vec2::splat(icons::ICON_SIZE),
    );
    icons::paint_icon(
        ui,
        icon_rect,
        icon,
        if hovered { theme::ACCENT } else { fg },
    );

    ui.painter().text(
        rect.left_center() + Vec2::new(34.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        fg,
    );

    resp.on_hover_text(tip)
}

pub fn icon_button(ui: &mut Ui, icon: Icon, tip: &str) -> Response {
    let dark = theme::is_dark_ui(ui);
    let size = Vec2::splat(30.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hovered = resp.hovered();
    if hovered {
        ui.painter().rect(
            rect,
            theme::control_rounding(),
            theme::control_hover(dark),
            Stroke::NONE,
        );
    }
    let fg = if hovered {
        theme::ACCENT
    } else {
        theme::text_secondary(dark)
    };
    icons::paint_icon(ui, rect.shrink2(Vec2::splat(7.0)), icon, fg);
    resp.on_hover_text(tip)
}

pub fn search_field(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    let dark = theme::is_dark_ui(ui);
    let frame = Frame::none()
        .fill(theme::input_fill(dark))
        .stroke(theme::input_stroke(ui, false))
        .rounding(theme::control_rounding())
        .inner_margin(Margin::symmetric(10.0, 8.0));

    frame
        .show(ui, |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                let (icon_rect, _) =
                    ui.allocate_exact_size(Vec2::splat(icons::ICON_SIZE), Sense::hover());
                icons::paint_icon(ui, icon_rect, Icon::Search, theme::text_muted(dark));
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(text)
                        .hint_text(hint)
                        .frame(false)
                        .desired_width(ui.available_width()),
                )
            })
            .inner
        })
        .inner
}

pub struct PrimaryButton<'a> {
    label: &'a str,
}

impl<'a> PrimaryButton<'a> {
    pub fn new(label: &'a str) -> Self {
        Self { label }
    }
}

impl Widget for PrimaryButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.add(
            Button::new(RichText::new(self.label).size(13.0).color(Color32::WHITE))
                .fill(theme::ACCENT)
                .stroke(Stroke::NONE)
                .rounding(theme::control_rounding()),
        )
    }
}

pub struct SecondaryButton<'a> {
    label: &'a str,
}

impl<'a> SecondaryButton<'a> {
    pub fn new(label: &'a str) -> Self {
        Self { label }
    }
}

impl Widget for SecondaryButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let dark = theme::is_dark_ui(ui);
        ui.add(
            Button::new(
                RichText::new(self.label)
                    .size(13.0)
                    .color(theme::text_primary(dark)),
            )
            .fill(theme::control_hover(dark))
            .stroke(theme::input_stroke(ui, false))
            .rounding(theme::control_rounding()),
        )
    }
}
