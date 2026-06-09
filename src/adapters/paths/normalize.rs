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
    match dunce::canonicalize(path) {
        Ok(path) => Ok(path.to_string_lossy().to_string()),
        Err(_) if !crate::adapters::paths::presence::target_exists(path)? => {
            Err(SymmError::TargetNotFound {
                path: path.to_string_lossy().to_string(),
            })
        }
        Err(_) => Ok(absolute_lexical(path).to_string_lossy().to_string()),
    }
}

pub fn normalize_link(path: &Path) -> String {
    absolute_lexical(path).to_string_lossy().to_string()
}

fn absolute_lexical(path: &Path) -> std::path::PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    crate::adapters::paths::lexical::clean(&absolute)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn known_exists_returns_target_not_found_if_path_vanished() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("vanished.txt");

        let err = normalize_target_known_exists(&path).expect_err("missing path should fail");

        assert!(matches!(err, SymmError::TargetNotFound { .. }));
    }

    #[test]
    fn target_symlink_normalizes_to_real_data_path() {
        let temp = tempdir().expect("temp dir");
        let real = temp.path().join("real.txt");
        let link = temp.path().join("target-link.txt");
        std::fs::write(&real, "payload").expect("write real target");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &link).expect("symlink target");
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&real, &link).expect("symlink target");

        let normalized = normalize_target(&link).expect("normalize target link");

        assert_eq!(
            std::path::PathBuf::from(normalized),
            dunce::canonicalize(real).expect("canonical real")
        );
    }
}
