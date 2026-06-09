use crate::domain::error::SymmError;
use std::fs;
use std::path::{Path, PathBuf};

pub const DB_FILE_NAME: &str = "symm.db";

pub fn data_home() -> Result<PathBuf, SymmError> {
    if let Some(p) = symm_home_override() {
        ensure_dir(&p)?;
        return Ok(p);
    }

    default_data_home()
}

pub fn symm_home_override() -> Option<PathBuf> {
    symm_home_override_value(std::env::var("SYMM_HOME").ok().as_deref())
}

pub fn default_data_home() -> Result<PathBuf, SymmError> {
    let exe = std::env::current_exe().map_err(|e| SymmError::IoError {
        message: format!("无法获取可执行文件路径：{e}"),
    })?;
    let exe_dir = exe.parent().ok_or_else(|| SymmError::InvalidArgument {
        message: "无法解析可执行文件所在目录".to_string(),
    })?;
    let p = data_dir_for_exe_dir(exe_dir);
    ensure_dir(&p)?;
    Ok(p)
}

/// 可执行文件旁 `data/`；CLI 在 `cli/` 子目录时（安装包 / 便携 zip）用应用根目录的 `data/`。
fn data_dir_for_exe_dir(exe_dir: &Path) -> PathBuf {
    if exe_dir
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("cli"))
        && let Some(app_root) = exe_dir.parent()
    {
        return app_root.join("data");
    }
    exe_dir.join("data")
}

pub fn db_path() -> Result<PathBuf, SymmError> {
    Ok(data_home()?.join(DB_FILE_NAME))
}

fn ensure_dir(path: &Path) -> Result<(), SymmError> {
    fs::create_dir_all(path).map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })
}

fn symm_home_override_value(value: Option<&str>) -> Option<PathBuf> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

#[cfg(test)]
mod tests {
    use super::{data_dir_for_exe_dir, symm_home_override_value};
    use std::path::Path;

    #[test]
    fn data_dir_next_to_exe_in_app_root() {
        let dir = Path::new("/apps/symm");
        assert_eq!(data_dir_for_exe_dir(dir), Path::new("/apps/symm/data"));
    }

    #[test]
    fn data_dir_cli_subdir_uses_app_root_data() {
        let dir = Path::new("/apps/symm/current/cli");
        assert_eq!(
            data_dir_for_exe_dir(dir),
            Path::new("/apps/symm/current/data")
        );
    }

    #[test]
    fn symm_home_override_ignores_empty_values() {
        assert_eq!(symm_home_override_value(None), None);
        assert_eq!(symm_home_override_value(Some("")), None);
        assert_eq!(symm_home_override_value(Some("  \t\n")), None);
    }

    #[test]
    fn symm_home_override_trims_non_empty_values() {
        assert_eq!(
            symm_home_override_value(Some("  /tmp/symm-home  ")),
            Some(Path::new("/tmp/symm-home").to_path_buf())
        );
    }
}
