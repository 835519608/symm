use crate::domain::error::SymmError;
use std::fs;
use std::path::Path;

pub fn normalize_target(path: &Path) -> Result<String, SymmError> {
    if !path.exists() {
        return Err(SymmError::TargetNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }
    normalize_target_known_exists(path)
}

/// 调用方已确认 `target` 存在时，跳过 `exists()`，仅规范化路径。
pub fn normalize_target_known_exists(path: &Path) -> Result<String, SymmError> {
    Ok(canonicalish(path))
}

pub fn normalize_link(path: &Path) -> String {
    if path.is_absolute() {
        return path.to_string_lossy().to_string();
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(path).to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

fn canonicalish(path: &Path) -> String {
    match fs::canonicalize(path) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}
