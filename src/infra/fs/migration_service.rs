use crate::domain::error::SymmError;
use crate::infra::errors::io_map::ioe;
use crate::infra::fs::acl;
use crate::infra::fs::path_ops;
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
