use crate::adapters::settings as settings_store;
use crate::domain::error::SymmError;
use crate::domain::gui_settings::GuiSettings;
use crate::gui::state::AppState;

pub fn load_into(state: &mut AppState) -> GuiSettings {
    let settings = settings_store::load();
    apply(state, &settings);
    settings
}

pub fn from_state(state: &AppState) -> GuiSettings {
    GuiSettings {
        theme: state.theme,
        locale: state.locale.clone(),
        sidebar_width: state.sidebar_width,
    }
}

pub fn apply(state: &mut AppState, settings: &GuiSettings) {
    state.theme = settings.theme;
    state.locale = settings.locale.clone();
    state.sidebar_width = settings.sidebar_width;
}

pub fn save_state(state: &AppState) -> Result<(), SymmError> {
    settings_store::save(&from_state(state))
}
