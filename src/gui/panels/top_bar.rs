use crate::gui::icons::Icon;
use crate::gui::state::AppState;
use crate::gui::widgets::{button, split_row};
use egui::Ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBarAction {
    OpenLinkOps,
    OpenSettings,
    None,
}

pub fn show_top_bar(ui: &mut Ui, state: &AppState) -> TopBarAction {
    let t = state.texts();

    let (left_action, right_action) = split_row(
        ui,
        |ui| {
            if button(ui)
                .icon(Icon::Add)
                .label(t.apply_link_op())
                .tip(t.apply_link_op_tip())
                .enabled(!state.busy)
                .show()
                .clicked()
            {
                TopBarAction::OpenLinkOps
            } else {
                TopBarAction::None
            }
        },
        |ui| {
            if button(ui)
                .icon(Icon::Gear)
                .tip(t.settings_open_tip())
                .enabled(!state.busy)
                .show()
                .clicked()
            {
                return TopBarAction::OpenSettings;
            }
            TopBarAction::None
        },
    );
    match right_action {
        TopBarAction::None => left_action,
        _ => right_action,
    }
}
