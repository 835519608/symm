use crate::error::SymmError;
use fs_extra::{dir, file};
use std::fs;
use std::path::Path;

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
        fs::rename(src, dst).map_err(io_error)?;
        return Ok(());
    }

    copy_path_with_progress(src, dst, reporter)?;
    reporter(MigrationEvent::RemovingSource {
        source: src.display().to_string(),
    })?;
    if let Err(remove_err) = remove_path_any(src) {
        let _ = remove_path_any(dst);
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

fn copy_path_with_progress<F>(src: &Path, dst: &Path, reporter: &mut F) -> Result<(), SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    let meta = fs::symlink_metadata(src).map_err(|e| SymmError::IoError {
        message: format!("无法读取源路径元数据：{e}"),
    })?;

    if meta.file_type().is_symlink() {
        return Err(SymmError::InvalidArgument {
            message: "不支持复制软链接路径".to_string(),
        });
    }

    if meta.is_dir() {
        if dst.exists() {
            return Err(SymmError::InvalidArgument {
                message: "迁移失败：目标目录已存在".to_string(),
            });
        }
        fs::create_dir_all(dst).map_err(|e| SymmError::IoError {
            message: format!("无法创建目标目录：{e}"),
        })?;

        let mut options = dir::CopyOptions::new();
        options.content_only = true;
        options.copy_inside = true;
        let mut callback_error: Option<SymmError> = None;

        let copy_result = dir::copy_with_progress(src, dst, &options, |info| {
            match reporter(MigrationEvent::Copying {
                copied_bytes: info.copied_bytes,
                total_bytes: info.total_bytes,
                current_item: if info.file_name.is_empty() {
                    None
                } else {
                    Some(info.file_name)
                },
            }) {
                Ok(()) => dir::TransitProcessResult::ContinueOrAbort,
                Err(err) => {
                    callback_error = Some(err);
                    dir::TransitProcessResult::Abort
                }
            }
        });
        if let Some(err) = callback_error {
            let _ = remove_path_any(dst);
            return Err(err);
        }
        if let Err(err) = copy_result {
            let _ = remove_path_any(dst);
            return Err(fs_extra_error(err));
        }
        return Ok(());
    }

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
            message: format!("无法创建目标父目录：{e}"),
        })?;
    }

    let options = file::CopyOptions::new();
    if let Err(err) = file::copy_with_progress(src, dst, &options, |info| {
        let _ = reporter(MigrationEvent::Copying {
            copied_bytes: info.copied_bytes,
            total_bytes: info.total_bytes,
            current_item: src.file_name().map(|s| s.to_string_lossy().to_string()),
        });
    }) {
        let _ = remove_path_any(dst);
        return Err(fs_extra_error(err));
    }
    Ok(())
}

fn can_use_fast_move(src: &Path, dst: &Path) -> Result<bool, SymmError> {
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
        let src_meta = fs::metadata(src).map_err(io_error)?;
        let dst_meta = fs::metadata(dst_parent).map_err(io_error)?;
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

fn remove_path_any(path: &Path) -> Result<(), SymmError> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_dir() {
                fs::remove_dir_all(path).map_err(io_error)?;
            } else {
                fs::remove_file(path).map_err(io_error)?;
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(e)),
    }
}

fn io_error(e: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: e.to_string(),
    }
}

fn fs_extra_error(e: fs_extra::error::Error) -> SymmError {
    SymmError::IoError {
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MigrationEvent, copy_path_with_progress, migrate_path, move_path_without_progress,
    };
    use crate::error::SymmError;
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
