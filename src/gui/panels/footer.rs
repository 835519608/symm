use crate::gui::state::AppState;
use crate::gui::theme;
use egui::{RichText, Ui};

pub fn show_footer(ui: &mut Ui, state: &AppState) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("symm v{}", env!("CARGO_PKG_VERSION")))
                .size(11.0)
                .color(theme::TEXT_SECONDARY),
        );
        if let Some(home) = &state.data_home {
            ui.label(
                RichText::new(format!("· 数据目录 {}", home.display()))
                    .size(11.0)
                    .color(theme::TEXT_SECONDARY),
            );
        }
        if let Some(msg) = &state.toast {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(msg).size(11.0).color(theme::ACCENT));
            });
        }
    });
}
