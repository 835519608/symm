use crate::gui::icons::Icon;
use crate::gui::state::AppState;
use crate::gui::widgets::{button, split_row};
use egui::Ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBarAction {
    AddLink,
    CycleTheme,
    CycleLocale,
    OpenSettings,
    None,
}

pub fn show_top_bar(ui: &mut Ui, state: &AppState) -> TopBarAction {
    let t = state.texts();
    let theme_tip = t.theme_tip(t.theme_mode_label(state.theme));

    let (left_action, right_action) = split_row(
        ui,
        |ui| {
            if button(ui)
                .icon(Icon::Add)
                .label(t.add_link())
                .tip(t.add_link_tip())
                .enabled(!state.busy)
                .show()
                .clicked()
            {
                TopBarAction::AddLink
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
            if button(ui)
                .icon(Icon::Globe)
                .tip(t.locale_tip())
                .enabled(!state.busy)
                .show()
                .clicked()
            {
                return TopBarAction::CycleLocale;
            }
            if button(ui)
                .icon(state.theme.icon())
                .tip(&theme_tip)
                .enabled(!state.busy)
                .show()
                .clicked()
            {
                return TopBarAction::CycleTheme;
            }
            TopBarAction::None
        },
    );
    match right_action {
        TopBarAction::None => left_action,
        _ => right_action,
    }
}
