//! GUI 偏好读写：默认 `data/settings.json`（与可切换的 `symm.db` 数据目录分离）。

use crate::adapters::paths::runtime_paths;
use crate::domain::error::SymmError;
use crate::domain::gui_settings::GuiSettings;
use std::fs;
use std::path::{Path, PathBuf};

pub const SETTINGS_FILE_NAME: &str = "settings.json";

pub fn settings_path() -> Result<PathBuf, SymmError> {
    Ok(runtime_paths::default_data_home()?.join(SETTINGS_FILE_NAME))
}

/// 读取设置；文件不存在或解析失败时返回默认值（不报错）。
pub fn load() -> GuiSettings {
    let Ok(path) = settings_path() else {
        return GuiSettings::default();
    };
    load_from(&path)
}

pub fn save(settings: &GuiSettings) -> Result<(), SymmError> {
    let path = settings_path()?;
    save_to(&path, settings)
}

fn load_from(path: &Path) -> GuiSettings {
    let raw = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return GuiSettings::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn save_to(path: &Path, settings: &GuiSettings) -> Result<(), SymmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| SymmError::InvalidArgument {
        message: format!("设置序列化失败：{e}"),
    })?;
    fs::write(path, format!("{json}\n")).map_err(io_err)
}

fn io_err(e: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::gui_settings::ThemeMode;
    use tempfile::tempdir;

    #[test]
    fn missing_file_returns_default() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE_NAME);

        assert!(!path.exists());
        assert_eq!(load_from(&path), GuiSettings::default());
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        let settings = GuiSettings {
            theme: ThemeMode::Dark,
            locale: crate::domain::gui_settings::Locale::En,
            color_scheme: crate::domain::gui_settings::ColorScheme::Ocean,
            sidebar_width: 320.0,
            font_size_pt: 16.0,
            data_dir: None,
        };
        save_to(&path, &settings).expect("save");
        assert_eq!(load_from(&path), settings);
    }

    #[test]
    fn corrupt_file_returns_default() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE_NAME);

        fs::write(&path, "{not json").expect("write");
        assert_eq!(load_from(&path), GuiSettings::default());
    }
}
