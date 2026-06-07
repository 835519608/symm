use super::copy_dir;
use super::path::MigrationEvent;
use crate::adapters::errors::io::ioe;
use crate::adapters::paths::remove;
use crate::domain::error::SymmError;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

pub(crate) const COPY_BUFFER_SIZE: usize = 1024 * 1024;

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
                message: "迁移失败：目标目录已存在".to_string(), // keep
            });
        }
        fs::create_dir_all(dst).map_err(|e| SymmError::IoError {
            message: format!("无法创建目标目录：{e}"),
        })?;

        if let Err(err) = copy_dir::copy_dir_tree_with_progress(src, dst, reporter) {
            let _ = remove::remove_any(dst);
            return Err(err);
        }
        return Ok(());
    }

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
            message: format!("无法创建目标父目录：{e}"),
        })?;
    }

    let current_item = src
        .file_name()
        .map(|s| Arc::<str>::from(s.to_string_lossy()));
    let mut buf = vec![0u8; COPY_BUFFER_SIZE];
    if let Err(err) = copy_file_buffered(src, dst, current_item, &mut buf, reporter) {
        let _ = remove::remove_any(dst);
        return Err(err);
    }
    Ok(())
}

fn copy_file_buffered<F>(
    src: &Path,
    dst: &Path,
    current_item: Option<Arc<str>>,
    buf: &mut [u8],
    reporter: &mut F,
) -> Result<(), SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    let mut reader = fs::File::open(src).map_err(ioe)?;
    let mut writer = fs::File::create(dst).map_err(ioe)?;
    let mut copied_bytes = 0u64;
    loop {
        let n = reader.read(buf).map_err(ioe)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n]).map_err(ioe)?;
        copied_bytes = copied_bytes.saturating_add(n as u64);
        reporter(MigrationEvent::Copying {
            copied_bytes,
            files_copied: 1,
            current_item: current_item.clone(),
        })?;
    }
    writer.flush().map_err(ioe)?;
    copy_permissions(src, dst)
}

pub(crate) fn copy_permissions(src: &Path, dst: &Path) -> Result<(), SymmError> {
    let permissions = fs::metadata(src).map_err(ioe)?.permissions();
    fs::set_permissions(dst, permissions).map_err(ioe)
}
