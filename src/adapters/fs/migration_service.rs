use crate::adapters::errors::io_map::ioe;
use crate::adapters::fs::acl;
use crate::adapters::fs::path_ops;
#[cfg(windows)]
use crate::adapters::fs::tree_copy::recreate_symlink;
use crate::domain::error::SymmError;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum MigrationEvent {
    Scanning {
        source: String,
        target: String,
    },
    FastMove {
        source: String,
        target: String,
    },
    Copying {
        copied_bytes: u64,
        total_bytes: u64,
        current_item: Option<String>,
    },
    RemovingSource {
        source: String,
    },
    CreatingLink {
        link: String,
        target: String,
    },
    PersistingDb {
        link: String,
    },
    Done {
        link: String,
    },
}

pub fn migrate_path<F>(src: &Path, dst: &Path, reporter: &mut F) -> Result<(), SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    reporter(MigrationEvent::Scanning {
        source: src.display().to_string(),
        target: dst.display().to_string(),
    })?;

    if can_use_fast_move(src, dst)? {
        reporter(MigrationEvent::FastMove {
            source: src.display().to_string(),
            target: dst.display().to_string(),
        })?;
        fs::rename(src, dst).map_err(ioe)?;
        return Ok(());
    }

    let acl_snapshot = acl::snapshot_dir_acl(src)?;
    super::copy_with_progress::copy_path_with_progress(src, dst, reporter)?;
    if let Some(snapshot) = acl_snapshot {
        let _ = acl::restore_dir_acl(dst, &snapshot);
    }
    reporter(MigrationEvent::RemovingSource {
        source: src.display().to_string(),
    })?;
    if let Err(remove_err) = path_ops::remove_path_any(src) {
        let _ = path_ops::remove_path_any(dst);
        return Err(SymmError::IoError {
            message: format!("跨磁盘复制完成后无法删除源路径：{remove_err}"),
        });
    }
    Ok(())
}

pub fn move_path_without_progress(src: &Path, dst: &Path) -> Result<(), SymmError> {
    let mut noop = |_event: MigrationEvent| Ok(());
    migrate_path(src, dst, &mut noop)
}

pub fn can_use_fast_move(src: &Path, dst: &Path) -> Result<bool, SymmError> {
    let dst_parent = dst.parent().ok_or_else(|| SymmError::InvalidArgument {
        message: "无法解析目标父目录".to_string(),
    })?;
    if !dst_parent.exists() {
        return Err(SymmError::TargetNotFound {
            path: dst_parent.display().to_string(),
        });
    }

    #[cfg(windows)]
    {
        Ok(path_prefix(src) == path_prefix(dst_parent))
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let src_meta = fs::metadata(src).map_err(ioe)?;
        let dst_meta = fs::metadata(dst_parent).map_err(ioe)?;
        Ok(src_meta.dev() == dst_meta.dev())
    }

    #[cfg(not(any(windows, unix)))]
    {
        let _ = src;
        let _ = dst_parent;
        Ok(false)
    }
}

#[cfg(windows)]
fn path_prefix(path: &Path) -> Option<String> {
    use std::path::Component;

    path.components().find_map(|component| match component {
        Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().to_string()),
        _ => None,
    })
}

pub fn fs_extra_error(e: fs_extra::error::Error) -> SymmError {
    SymmError::IoError {
        message: e.to_string(),
    }
}

pub fn staging_path(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.to_path_buf();
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "path".to_string());
    p.set_file_name(format!("{file_name}{suffix}"));
    p
}

pub fn stage_existing_path(
    path: &Path,
    suffix: &str,
    role: &str,
) -> Result<Option<PathBuf>, SymmError> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            let staged = staging_path(path, suffix);
            move_path_with_retry(path, &staged, role)?;
            Ok(Some(staged))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(SymmError::IoError {
            message: format!("无法读取 {role} 状态：{e}"),
        }),
    }
}

pub fn rollback_staged_path(
    path: &Path,
    staged_path: Option<&Path>,
    role: &str,
) -> Result<(), SymmError> {
    let Some(staged) = staged_path else {
        return Ok(());
    };
    if path.exists() {
        path_ops::remove_path_any(path)?;
    }
    move_path_with_retry(staged, path, role).map_err(|e| SymmError::IoError {
        message: format!("回滚失败：无法恢复 {role}：{e}"),
    })
}

pub fn move_path_with_retry(src: &Path, dst: &Path, role: &str) -> Result<(), SymmError> {
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(e) => {
            #[cfg(windows)]
            if e.raw_os_error() == Some(5)
                && let Ok(meta) = fs::symlink_metadata(src)
                && meta.file_type().is_symlink()
            {
                return move_symlink_by_recreate(src, dst, role);
            }
            let mut message = format!("无法移动 {role}：{e}");
            if e.raw_os_error() == Some(5) {
                message.push_str(
                    "。系统拒绝访问（os error 5），可能仍有占用未被识别，或当前进程权限不足（可尝试以管理员身份运行）",
                );
            }
            Err(SymmError::IoError { message })
        }
    }
}

#[cfg(windows)]
fn move_symlink_by_recreate(src: &Path, dst: &Path, role: &str) -> Result<(), SymmError> {
    if dst.exists() {
        path_ops::remove_path_any(dst)?;
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
            message: format!("无法创建 {role} 目标父目录：{e}"),
        })?;
    }
    recreate_symlink(src, dst, Some((src, dst))).map_err(|e| SymmError::IoError {
        message: format!("重建软链接失败（{role}）：{e}"),
    })?;
    path_ops::remove_path_any(src)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MigrationEvent, migrate_path, move_path_without_progress};
    use crate::adapters::fs::copy_with_progress::copy_path_with_progress;
    use crate::domain::error::SymmError;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn move_path_without_progress_moves_file() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src.txt");
        let dst = temp.path().join("dst.txt");
        fs::write(&src, "payload").expect("write source");
        move_path_without_progress(&src, &dst).expect("move should succeed");
        assert!(!src.exists());
        assert_eq!(fs::read_to_string(&dst).expect("read target"), "payload");
    }

    #[test]
    fn migrate_path_reports_stages_for_same_volume_move() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src.txt");
        let dst = temp.path().join("dst.txt");
        fs::write(&src, "payload").expect("write source");
        let mut seen = Vec::new();
        migrate_path(&src, &dst, &mut |event| {
            seen.push(event);
            Ok(())
        })
        .expect("move should succeed");
        assert!(matches!(
            seen.first(),
            Some(MigrationEvent::Scanning { .. })
        ));
        assert!(
            seen.iter()
                .any(|event| matches!(event, MigrationEvent::FastMove { .. }))
        );
    }

    #[test]
    fn migrate_path_moves_directory_without_losing_contents() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src_dir");
        let nested = src.join("nested");
        let dst = temp.path().join("dst_dir");
        fs::create_dir_all(&nested).expect("create nested dir");
        fs::write(nested.join("file.txt"), "payload").expect("write payload");
        move_path_without_progress(&src, &dst).expect("move dir should succeed");
        assert!(!src.exists());
        assert_eq!(
            fs::read_to_string(dst.join("nested").join("file.txt")).expect("read target"),
            "payload"
        );
    }

    #[test]
    fn copy_path_with_progress_cleans_partial_target_when_reporter_aborts() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src_dir");
        let nested = src.join("nested");
        let dst = temp.path().join("dst_dir");
        fs::create_dir_all(&nested).expect("create nested dir");
        fs::write(nested.join("file.txt"), "payload").expect("write payload");
        let err = copy_path_with_progress(&src, &dst, &mut |_event| {
            Err(SymmError::IoError {
                message: "stop".to_string(),
            })
        })
        .expect_err("reporter abort should stop copy");
        assert!(
            matches!(err, SymmError::IoError { ref message } if message == "stop"),
            "unexpected error: {err:?}"
        );
        assert!(src.exists(), "source should stay in place on abort");
        assert!(
            !dst.exists(),
            "partial destination should be cleaned when copy aborts"
        );
    }
}
