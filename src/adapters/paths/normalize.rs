use crate::domain::error::SymmError;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

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
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    clean_lexical(&absolute).to_string_lossy().to_string()
}

fn canonicalish(path: &Path) -> String {
    dunce::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| clean_lexical(path).to_string_lossy().to_string())
}

fn clean_lexical(path: &Path) -> PathBuf {
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                clean.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                clean.push(component.as_os_str());
            }
        }
    }
    clean
}
