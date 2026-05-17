use crate::gui::state::{AppState, ThemePreference};
use crate::gui::theme;
use egui::RichText;

pub fn show_settings_window(ctx: &egui::Context, state: &mut AppState) {
    let mut open = state.settings_open;
    egui::Window::new("设置")
        .collapsible(false)
        .resizable(false)
        .default_width(280.0)
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 48.0])
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(
                RichText::new("外观")
                    .strong()
                    .color(theme::primary_text(ui)),
            );
            ui.add_space(6.0);
            for pref in [
                ThemePreference::System,
                ThemePreference::Light,
                ThemePreference::Dark,
            ] {
                ui.radio_value(&mut state.theme, pref, pref.label());
            }
        });
    state.settings_open = open;
}
