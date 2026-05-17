use crate::gui::theme;
use crate::gui::widgets::card;
use egui::{RichText, Ui};

pub fn stat_card(ui: &mut Ui, icon: &str, title: &str, value: impl std::fmt::Display) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon).size(22.0).color(theme::TEXT_SECONDARY));
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(12.0).color(theme::TEXT_SECONDARY));
                ui.label(
                    RichText::new(value.to_string())
                        .size(28.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
            });
        });
    });
}
