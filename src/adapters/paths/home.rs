use crate::domain::error::SymmError;
use std::fs;
use std::path::{Path, PathBuf};

pub const DB_FILE_NAME: &str = "symm.db";

pub fn data_home() -> Result<PathBuf, SymmError> {
    if let Ok(v) = std::env::var("SYMM_HOME") {
        let p = PathBuf::from(v);
        ensure_dir(&p)?;
        return Ok(p);
    }

    let exe = std::env::current_exe().map_err(|e| SymmError::IoError {
        message: format!("无法获取可执行文件路径：{e}"),
    })?;
    let exe_dir = exe.parent().ok_or_else(|| SymmError::InvalidArgument {
        message: "无法解析可执行文件所在目录".to_string(),
    })?;
    let p = exe_dir.join("data");
    ensure_dir(&p)?;
    Ok(p)
}

pub fn db_path() -> Result<PathBuf, SymmError> {
    Ok(data_home()?.join(DB_FILE_NAME))
}

fn ensure_dir(path: &Path) -> Result<(), SymmError> {
    fs::create_dir_all(path).map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })
}
