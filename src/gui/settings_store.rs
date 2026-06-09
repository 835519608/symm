use crate::adapters::settings as settings_store;
use crate::domain::error::SymmError;
use crate::domain::gui_settings::{GuiSettings, data_dir_from_settings};
use crate::gui::env::sync_symm_home;
use crate::gui::state::AppState;
use std::path::PathBuf;

pub fn load_into(state: &mut AppState) {
    let settings = settings_store::load();
    apply(state, &settings);
    if let Ok(home) = std::env::var("SYMM_HOME")
        && !home.trim().is_empty()
    {
        state.data_dir = home.trim().to_string();
        state.data_dir_runtime_override = true;
    }
    sync_symm_home(&state.data_dir);
}

pub fn apply(state: &mut AppState, settings: &GuiSettings) {
    state.theme = settings.theme;
    state.color_scheme = settings.color_scheme;
    state.locale = settings.locale;
    state.sidebar_width = settings.sidebar_width;
    state.font_size_pt = crate::domain::gui_settings::sanitize_font_size_pt(settings.font_size_pt);
    let data_dir = data_dir_from_settings(settings);
    state.data_dir = data_dir.clone();
    state.persisted_data_dir = data_dir;
    state.data_dir_runtime_override = false;
}

pub fn save(settings: &GuiSettings) -> Result<(), SymmError> {
    settings_store::save(settings)
}

pub fn restore_data_dir(data_dir: &str) {
    sync_symm_home(data_dir);
}

pub fn apply_data_dir(data_dir: &str) -> Result<PathBuf, String> {
    let trimmed = data_dir.trim();
    if trimmed.is_empty() {
        let data_home = crate::adapters::paths::runtime_paths::default_data_home()
            .map_err(|e| e.to_string())?;
        sync_symm_home("");
        return Ok(data_home);
    }
    let path = PathBuf::from(trimmed);
    crate::gui::env::ensure_data_dir(&path).map_err(|e| e.to_string())?;
    sync_symm_home(trimmed);
    Ok(path)
}
