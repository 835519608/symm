use crate::domain::error::SymmError;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

pub(crate) fn target_exists(path: &Path) -> Result<bool, SymmError> {
    match fs::metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(false),
        Err(err) => Err(SymmError::IoError {
            message: format!("无法确认路径是否存在 {}：{err}", path.display()),
        }),
    }
}

pub(crate) fn path_itself_exists(path: &Path) -> Result<bool, SymmError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(false),
        Err(err) => Err(SymmError::IoError {
            message: format!("无法确认路径是否存在 {}：{err}", path.display()),
        }),
    }
}
