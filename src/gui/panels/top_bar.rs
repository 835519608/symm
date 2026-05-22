use crate::gui::icons::Icon;
use crate::gui::state::AppState;
use crate::gui::widgets::button;
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
    let mut action = TopBarAction::None;
    let t = state.texts();
    let theme_tip = t.theme_tip(t.theme_mode_label(state.theme));

    ui.horizontal(|ui| {
        if button(ui)
            .icon(Icon::Add)
            .label(t.add_link())
            .tip(t.add_link_tip())
            .show()
            .clicked()
        {
            action = TopBarAction::AddLink;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if button(ui)
                .icon(Icon::Gear)
                .tip(t.settings_open_tip())
                .show()
                .clicked()
            {
                action = TopBarAction::OpenSettings;
            }
            if button(ui)
                .icon(Icon::Globe)
                .tip(t.locale_tip())
                .show()
                .clicked()
            {
                action = TopBarAction::CycleLocale;
            }
            if button(ui)
                .icon(state.theme.icon())
                .tip(&theme_tip)
                .show()
                .clicked()
            {
                action = TopBarAction::CycleTheme;
            }
        });
    });
    action
}
