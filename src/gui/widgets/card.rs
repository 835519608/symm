use crate::gui::theme;
use egui::{Margin, RichText, Stroke, Ui};

pub fn card(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    theme::card_frame(ui)
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
            .color(theme::primary_text(ui))
            .size(15.0),
    );
    ui.add_space(6.0);
}

pub fn subtle_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(theme::primary_text(ui)))
            .fill(egui::Color32::TRANSPARENT)
            .stroke(Stroke::NONE),
    )
}

pub fn primary_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(13.0).color(egui::Color32::WHITE))
            .fill(theme::ACCENT)
            .stroke(Stroke::NONE),
    )
}
