//! GUI 偏好读写：`data/settings.json`（与 `symm.db` 分离）。

use crate::adapters::paths::runtime_paths;
use crate::domain::error::SymmError;
use crate::domain::gui_settings::GuiSettings;
use std::fs;
use std::path::{Path, PathBuf};

pub const SETTINGS_FILE_NAME: &str = "settings.json";

pub fn settings_path() -> Result<PathBuf, SymmError> {
    Ok(runtime_paths::data_home()?.join(SETTINGS_FILE_NAME))
}

/// 读取设置；文件不存在或解析失败时返回默认值（不报错）。
pub fn load() -> GuiSettings {
    match settings_path() {
        Ok(path) => load_from(&path),
        Err(_) => GuiSettings::default(),
    }
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
    use std::env;
    use tempfile::tempdir;

    fn with_symm_home(dir: &Path, f: impl FnOnce(&Path)) {
        let home = dir.to_string_lossy().to_string();
        unsafe {
            env::set_var("SYMM_HOME", &home);
        }
        f(dir);
        unsafe {
            env::remove_var("SYMM_HOME");
        }
    }

    #[test]
    fn missing_file_returns_default() {
        let dir = tempdir().expect("tempdir");
        with_symm_home(dir.path(), |home| {
            let path = home.join(SETTINGS_FILE_NAME);
            assert!(!path.exists());
            assert_eq!(load(), GuiSettings::default());
        });
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        let settings = GuiSettings {
            theme: ThemeMode::Dark,
            locale: "en".to_string(),
            sidebar_width: 320.0,
        };
        save_to(&path, &settings).expect("save");
        assert_eq!(load_from(&path), settings);
    }

    #[test]
    fn corrupt_file_returns_default() {
        let dir = tempdir().expect("tempdir");
        with_symm_home(dir.path(), |home| {
            let path = home.join(SETTINGS_FILE_NAME);
            fs::write(&path, "{not json").expect("write");
            assert_eq!(load(), GuiSettings::default());
        });
    }
}
