use crate::domain::model::LinkView;
use crate::gui::state::AppState;
use crate::gui::theme;
use crate::gui::widgets::{card, detail_field, detail_path_field, vertical_when_overflow};
use egui::Ui;

pub fn show_content(ui: &mut Ui, state: &AppState, view: Option<&LinkView>, p: &theme::UiPalette) {
    if let Some(v) = view {
        vertical_when_overflow(ui, "main_content", |ui| show_detail(ui, state, v, p));
    }
}

fn show_detail(ui: &mut Ui, state: &AppState, view: &LinkView, p: &theme::UiPalette) {
    let t = state.texts();

    card(ui, |ui| {
        detail_field(ui, p, t.field_name(), &view.name);
        detail_field(ui, p, t.field_kind(), t.link_kind(view.link_kind));
        detail_field(ui, p, t.field_status(), t.link_status(view.status));
        if let Some(err) = &view.status_error {
            detail_field(ui, p, t.field_status_error(), err);
        }
        detail_path_field(
            ui,
            p,
            t.field_link_path(),
            &view.link_path,
            &t.copy_field_tip(t.field_link_path()),
        );
        detail_path_field(
            ui,
            p,
            t.field_target_path(),
            &view.target_path,
            &t.copy_field_tip(t.field_target_path()),
        );
    });
}
