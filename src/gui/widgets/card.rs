use crate::gui::theme;
use egui::{Margin, RichText, Stroke, Ui};

pub fn card(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    theme::card_frame()
        .inner_margin(Margin::same(theme::CARD_PADDING))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });
}

pub fn card_header(ui: &mut Ui, title: &str) {
    ui.label(
        RichText::new(title)
            .strong()
            .color(theme::TEXT_PRIMARY)
            .size(15.0),
    );
    ui.add_space(6.0);
}

pub fn subtle_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(theme::TEXT_PRIMARY))
            .fill(egui::Color32::TRANSPARENT)
            .stroke(Stroke::NONE),
    )
}
