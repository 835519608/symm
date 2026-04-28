use crate::domain::error::SymmError;
use crate::infra::fs::migration_service::{MigrationEvent, fs_extra_error};
use crate::infra::fs::path_ops;
use fs_extra::{dir, file};
use std::fs;
use std::path::Path;

pub fn copy_path_with_progress<F>(src: &Path, dst: &Path, reporter: &mut F) -> Result<(), SymmError>
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
            let _ = path_ops::remove_path_any(dst);
            return Err(err);
        }
        if let Err(err) = copy_result {
            let _ = path_ops::remove_path_any(dst);
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
        let _ = path_ops::remove_path_any(dst);
        return Err(fs_extra_error(err));
    }
    Ok(())
}
