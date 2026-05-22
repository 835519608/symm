use crate::adapters::settings as settings_store;
use crate::domain::error::SymmError;
use crate::domain::gui_settings::{GuiSettings, data_dir_from_settings};
use crate::gui::env::sync_symm_home;
use crate::gui::state::AppState;
pub fn load_into(state: &mut AppState) -> GuiSettings {
    let settings = settings_store::load();
    apply(state, &settings);
    sync_symm_home(&state.data_dir);
    settings
}

pub fn from_state(state: &AppState) -> GuiSettings {
    GuiSettings {
        theme: state.theme,
        color_scheme: state.color_scheme,
        locale: state.locale,
        sidebar_width: state.sidebar_width,
        font_size_pt: state.font_size_pt,
        data_dir: if state.data_dir.trim().is_empty() {
            None
        } else {
            Some(state.data_dir.clone())
        },
    }
}

pub fn apply(state: &mut AppState, settings: &GuiSettings) {
    state.theme = settings.theme;
    state.color_scheme = settings.color_scheme;
    state.locale = settings.locale;
    state.sidebar_width = settings.sidebar_width;
    state.font_size_pt = crate::domain::gui_settings::sanitize_font_size_pt(settings.font_size_pt);
    state.data_dir = data_dir_from_settings(settings);
}

pub fn save_state(state: &AppState) -> Result<(), SymmError> {
    settings_store::save(&from_state(state))
}
