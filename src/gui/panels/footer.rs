use crate::gui::state::AppState;
use crate::gui::theme::{self, rich_small};
use egui::Ui;

pub fn show_footer(ui: &mut Ui, state: &AppState) {
    let p = theme::resolve(state.theme, state.color_scheme);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(rich_small(
            &format!("v{}", env!("CARGO_PKG_VERSION")),
            p.text_muted,
        ));
    });
}
