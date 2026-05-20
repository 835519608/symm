use crate::gui::theme;
use egui::{RichText, Ui};

pub use super::button::{PrimaryButton, SecondaryButton};

pub fn card(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    theme::card_frame(ui).show(ui, |ui| {
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
    ui.add_space(8.0);
}

pub fn subtle_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(SecondaryButton::new(label))
}

pub fn primary_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(PrimaryButton::new(label))
}
