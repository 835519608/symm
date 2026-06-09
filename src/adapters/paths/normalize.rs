use crate::domain::error::SymmError;
use std::path::Path;

pub fn normalize_target(path: &Path) -> Result<String, SymmError> {
    if !crate::adapters::paths::presence::target_exists(path)? {
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
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    crate::adapters::paths::lexical::clean(&absolute)
        .to_string_lossy()
        .to_string()
}

fn canonicalish(path: &Path) -> String {
    dunce::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| {
            crate::adapters::paths::lexical::clean(path)
                .to_string_lossy()
                .to_string()
        })
}
