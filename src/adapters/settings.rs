//! GUI 偏好读写：默认 `data/settings.json`（与可切换的 `symm.db` 数据目录分离）。

use crate::adapters::paths::runtime_paths;
use crate::domain::error::SymmError;
use crate::domain::gui_settings::GuiSettings;
use std::fs;
use std::io::Write;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
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
    load_file(path)
        .or_else(|| load_file(&backup_path(path)))
        .unwrap_or_default()
}

fn load_file(path: &Path) -> Option<GuiSettings> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(_) => return None,
    };
    serde_json::from_str(&raw).ok()
}

fn save_to(path: &Path, settings: &GuiSettings) -> Result<(), SymmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_err)?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| SymmError::InvalidArgument {
        message: format!("设置序列化失败：{e}"),
    })?;
    let tmp = temp_path(path);
    let content = format!("{json}\n");
    let result = write_temp_then_replace(path, &tmp, content.as_bytes());
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn write_temp_then_replace(path: &Path, tmp: &Path, content: &[u8]) -> Result<(), SymmError> {
    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(tmp)
            .map_err(io_err)?;
        file.write_all(content).map_err(io_err)?;
        file.sync_all().map_err(io_err)?;
    }
    if path.exists() {
        fs::copy(path, backup_path(path)).map_err(io_err)?;
    }
    replace_file(tmp, path)
}

fn io_err(e: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: e.to_string(),
    }
}

#[cfg(not(windows))]
fn replace_file(src: &Path, dst: &Path) -> Result<(), SymmError> {
    fs::rename(src, dst).map_err(io_err)
}

#[cfg(windows)]
fn replace_file(src: &Path, dst: &Path) -> Result<(), SymmError> {
    use windows::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    use windows::core::PCWSTR;

    let src = path_to_wide(src);
    let dst = path_to_wide(dst);
    unsafe {
        MoveFileExW(
            PCWSTR(src.as_ptr()),
            PCWSTR(dst.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|err| SymmError::IoError {
        message: err.to_string(),
    })
}

#[cfg(windows)]
fn path_to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn backup_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| SETTINGS_FILE_NAME.into());
    path.with_file_name(format!("{file_name}.bak"))
}

fn temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| SETTINGS_FILE_NAME.into());
    let pid = std::process::id();
    for n in 0..1000u32 {
        let candidate = path.with_file_name(format!(".{file_name}.{pid}.{n}.tmp"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.with_file_name(format!(".{file_name}.{pid}.tmp"))
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

    #[test]
    fn corrupt_file_falls_back_to_backup() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        let settings = GuiSettings {
            data_dir: Some("/tmp/symm-custom".to_string()),
            ..GuiSettings::default()
        };
        fs::write(&path, "{not json").expect("write corrupt");
        fs::write(
            backup_path(&path),
            serde_json::to_string_pretty(&settings).expect("json"),
        )
        .expect("write backup");

        assert_eq!(load_from(&path), settings);
    }

    #[test]
    fn save_keeps_backup_of_previous_settings() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        let previous = GuiSettings {
            data_dir: Some("/tmp/previous".to_string()),
            ..GuiSettings::default()
        };
        let next = GuiSettings {
            data_dir: Some("/tmp/next".to_string()),
            ..GuiSettings::default()
        };
        save_to(&path, &previous).expect("save previous");
        save_to(&path, &next).expect("save next");

        assert_eq!(load_from(&path), next);
        assert_eq!(load_file(&backup_path(&path)), Some(previous));
    }

    #[test]
    fn save_failure_removes_temporary_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join(SETTINGS_FILE_NAME);
        let previous = GuiSettings {
            data_dir: Some("/tmp/previous".to_string()),
            ..GuiSettings::default()
        };
        let next = GuiSettings {
            data_dir: Some("/tmp/next".to_string()),
            ..GuiSettings::default()
        };
        save_to(&path, &previous).expect("save previous");
        fs::create_dir(backup_path(&path)).expect("block backup file path");

        save_to(&path, &next).expect_err("backup failure should fail save");

        assert_eq!(load_file(&path), Some(previous));
        let temp_files = fs::read_dir(dir.path())
            .expect("read settings dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.starts_with(".settings.json.") && name.ends_with(".tmp"))
            .collect::<Vec<_>>();
        assert!(
            temp_files.is_empty(),
            "temporary files left: {temp_files:?}"
        );
    }
}
