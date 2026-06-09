use crate::domain::error::SymmError;
use std::path::{Path, PathBuf};

pub fn normalize_target(path: &Path) -> Result<String, SymmError> {
    if !crate::adapters::paths::presence::target_exists(path)? {
        return Err(SymmError::TargetNotFound {
            path: display_path(path),
        });
    }
    normalize_target_known_exists(path)
}

/// 调用方已确认 `target` 存在时，跳过 `exists()`，仅规范化路径。
pub fn normalize_target_known_exists(path: &Path) -> Result<String, SymmError> {
    match dunce::canonicalize(path) {
        Ok(path) => path_to_storage_string(&path),
        Err(_) if !crate::adapters::paths::presence::target_exists(path)? => {
            Err(SymmError::TargetNotFound {
                path: display_path(path),
            })
        }
        Err(_) => path_to_storage_string(&absolute_lexical(path)),
    }
}

pub fn normalize_link(path: &Path) -> Result<String, SymmError> {
    path_to_storage_string(&absolute_lexical(path))
}

fn absolute_lexical(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    crate::adapters::paths::lexical::clean(&absolute)
}

fn path_to_storage_string(path: &Path) -> Result<String, SymmError> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| SymmError::InvalidArgument {
            message: format!(
                "路径必须是有效 Unicode，无法写入链接记录：{}",
                display_path(path)
            ),
        })
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
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

    #[cfg(unix)]
    #[test]
    fn link_path_rejects_non_unicode_path_instead_of_lossy_storage() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let temp = tempdir().expect("temp dir");
        let raw = OsString::from_vec(vec![b'l', b'i', 0xff, b'k']);
        let path = temp.path().join(raw);

        let err = normalize_link(&path).expect_err("non unicode path should fail");

        assert!(matches!(err, SymmError::InvalidArgument { .. }));
        assert!(err.to_string().contains("有效 Unicode"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn target_path_rejects_non_unicode_path_instead_of_lossy_storage() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let temp = tempdir().expect("temp dir");
        let raw = OsString::from_vec(vec![b't', b'a', 0xff, b'g']);
        let path = temp.path().join(raw);
        std::fs::write(&path, "payload").expect("write non unicode target");

        let err = normalize_target(&path).expect_err("non unicode path should fail");

        assert!(matches!(err, SymmError::InvalidArgument { .. }));
        assert!(err.to_string().contains("有效 Unicode"));
    }
}
