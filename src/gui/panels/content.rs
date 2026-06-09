use crate::domain::model::LinkView;
use crate::gui::state::AppState;
use crate::gui::theme;
use crate::gui::widgets::{card, detail_field, detail_path_field, vertical_when_overflow};
use egui::Ui;

pub fn show_content(ui: &mut Ui, state: &AppState, view: Option<&LinkView>) {
    vertical_when_overflow(ui, "main_content", |ui| {
        match view {
            Some(v) => show_detail(ui, state, v),
            None => show_empty(ui, state),
        };
    });
}

fn show_empty(ui: &mut Ui, state: &AppState) {
    let p = theme::resolve(state.theme, state.color_scheme);
    card(ui, |ui| {
        ui.label(theme::rich_body(state.texts().no_selection(), p.text_muted));
    });
}

fn show_detail(ui: &mut Ui, state: &AppState, view: &LinkView) {
    let t = state.texts();
    let p = theme::resolve(state.theme, state.color_scheme);

    card(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let title = view.display_name();
            let title_resp =
                ui.add(egui::Label::new(theme::rich_detail_title(title.as_ref(), p.text)).wrap());
            if title.chars().count() > 24 {
                title_resp.on_hover_text(title.as_ref());
            }
            ui.label(theme::rich_body(
                t.link_status(view.status),
                theme::status_color_for_palette(view.status, &p),
            ));
        });
    });
    ui.add_space(theme::gap_lg(ui));

    card(ui, |ui| {
        detail_field(ui, &p, t.field_name(), &view.name);
        detail_field(ui, &p, t.field_kind(), t.link_kind(view.link_kind));
        detail_field(ui, &p, t.field_status(), t.link_status(view.status));
        if let Some(err) = &view.status_error {
            detail_field(ui, &p, t.field_status_error(), err);
        }
        detail_path_field(
            ui,
            &p,
            t.field_link_path(),
            &view.link_path,
            &t.copy_field_tip(t.field_link_path()),
        );
        detail_path_field(
            ui,
            &p,
            t.field_target_path(),
            &view.target_path,
            &t.copy_field_tip(t.field_target_path()),
        );
        detail_field(ui, &p, t.field_index(), &view.index.to_string());
        detail_field(ui, &p, t.field_id(), &view.id.to_string());
    });
}
