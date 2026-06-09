use crate::adapters::settings as settings_store;
use crate::domain::error::SymmError;
use crate::domain::gui_settings::{GuiSettings, data_dir_from_settings};
use crate::gui::state::AppState;
use std::path::PathBuf;

pub fn load_into(state: &mut AppState) {
    let settings = settings_store::load();
    apply(state, &settings);
    if let Some(home) = crate::adapters::paths::runtime_paths::symm_home_override() {
        state.data_dir = home.to_string_lossy().to_string();
        state.data_dir_runtime_override = true;
    }
}

pub fn apply(state: &mut AppState, settings: &GuiSettings) {
    state.theme = settings.theme;
    state.color_scheme = settings.color_scheme;
    state.locale = settings.locale;
    state.sidebar_width = settings.sidebar_width;
    state.transient_sidebar_width = settings.sidebar_width;
    state.font_size_pt = crate::domain::gui_settings::sanitize_font_size_pt(settings.font_size_pt);
    let data_dir = data_dir_from_settings(settings);
    state.data_dir = data_dir.clone();
    state.persisted_data_dir = data_dir;
    state.data_dir_runtime_override = false;
}

pub fn save(settings: &GuiSettings) -> Result<(), SymmError> {
    settings_store::save(settings)
}

pub fn resolve_data_dir(data_dir: &str) -> Result<PathBuf, String> {
    let trimmed = data_dir.trim();
    if trimmed.is_empty() {
        return crate::adapters::paths::runtime_paths::default_data_home()
            .map_err(|e| e.to_string());
    }
    let path = PathBuf::from(trimmed);
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    std::fs::canonicalize(&path).map_err(|e| e.to_string())
}
