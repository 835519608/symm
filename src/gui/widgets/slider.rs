use crate::gui::theme::{UiPalette, rich_body};
use egui::{Response, Ui};

pub fn value_slider(
    ui: &mut Ui,
    p: &UiPalette,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    slider_width: f32,
    value_text: &str,
) -> Response {
    ui.horizontal(|ui| {
        let resp = ui
            .scope(|ui| {
                apply_slider_visuals(ui, p);
                ui.add_sized(
                    egui::vec2(slider_width, ui.spacing().interact_size.y),
                    egui::Slider::new(value, range)
                        .show_value(false)
                        .smart_aim(false)
                        .trailing_fill(true),
                )
            })
            .inner;
        ui.label(rich_body(value_text, p.text));
        resp
    })
    .inner
}

fn apply_slider_visuals(ui: &mut Ui, p: &UiPalette) {
    let track = if p.dark {
        p.surface_hover
    } else {
        egui::Color32::from_rgb(0xCB, 0xD5, 0xE1)
    };
    let visuals = &mut ui.style_mut().visuals;
    visuals.widgets.inactive.bg_fill = track;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, p.accent_text);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
    visuals.widgets.hovered.bg_fill = p.accent_soft;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, p.accent_text);
    visuals.widgets.active.bg_fill = p.accent_active;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, p.accent_text);
    visuals.selection.bg_fill = p.accent_soft;
}
