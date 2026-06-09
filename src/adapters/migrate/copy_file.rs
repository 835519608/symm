use super::copy_dir;
use super::path::MigrationEvent;
use super::rebase;
use crate::adapters::errors::io::ioe;
use crate::adapters::paths::{presence, remove};
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use std::fs;
use std::io::ErrorKind;
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

    if symlink::kind_from_path_and_metadata(src, &meta)?.is_some() {
        copy_link_path(src, dst, reporter)?;
        return Ok(());
    }

    if meta.is_dir() {
        if presence::path_itself_exists(dst)? {
            return Err(SymmError::InvalidArgument {
                message: "迁移失败：目标目录已存在".to_string(), // keep
            });
        }
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
                message: format!("无法创建目标父目录：{e}"),
            })?;
        }
        fs::create_dir(dst).map_err(|e| SymmError::IoError {
            message: format!("无法创建目标目录：{e}"),
        })?;

        if let Err(err) = copy_dir::copy_dir_tree_with_progress(src, dst, reporter) {
            let _ = remove::remove_any(dst);
            return Err(err);
        }
        if let Err(err) = copy_permissions(src, dst) {
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
    if let Err(err) = copy_regular_file_with_progress(
        src,
        dst,
        0,
        current_item,
        &mut buf,
        &mut |copied_bytes, current_item| {
            reporter(MigrationEvent::Copying {
                copied_bytes,
                files_copied: 1,
                current_item,
            })
        },
    ) {
        if err.dst_created {
            let _ = remove::remove_any(dst);
        }
        return Err(err.err);
    }
    Ok(())
}

fn copy_link_path<F>(src: &Path, dst: &Path, reporter: &mut F) -> Result<(), SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    if presence::path_itself_exists(dst)? {
        return Err(SymmError::InvalidArgument {
            message: "迁移失败：目标路径已存在".to_string(),
        });
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
            message: format!("无法创建目标父目录：{e}"),
        })?;
    }
    if let Err(err) = rebase::recreate_symlink(src, dst, None).and_then(|()| {
        reporter(MigrationEvent::Copying {
            copied_bytes: 0,
            files_copied: 1,
            current_item: dst
                .file_name()
                .map(|s| Arc::<str>::from(s.to_string_lossy())),
        })
    }) {
        let _ = remove::remove_any(dst);
        return Err(err);
    }
    Ok(())
}

pub(crate) struct CopyFileFailure {
    pub(crate) err: SymmError,
    pub(crate) dst_created: bool,
}

impl CopyFileFailure {
    fn before_create(err: SymmError) -> Self {
        Self {
            err,
            dst_created: false,
        }
    }

    fn after_create(err: SymmError) -> Self {
        Self {
            err,
            dst_created: true,
        }
    }
}

pub(crate) fn copy_regular_file_with_progress<F>(
    src: &Path,
    dst: &Path,
    mut copied_bytes: u64,
    current_item: Option<Arc<str>>,
    buf: &mut [u8],
    reporter: &mut F,
) -> Result<u64, CopyFileFailure>
where
    F: FnMut(u64, Option<Arc<str>>) -> Result<(), SymmError>,
{
    let mut reader = fs::File::open(src).map_err(|err| CopyFileFailure::before_create(ioe(err)))?;
    let mut writer = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)
        .map_err(|err| {
            if err.kind() == ErrorKind::AlreadyExists {
                return CopyFileFailure::before_create(SymmError::InvalidArgument {
                    message: format!("迁移失败：目标路径已存在：{}", dst.display()),
                });
            }
            CopyFileFailure::before_create(ioe(err))
        })?;
    loop {
        let n = reader
            .read(buf)
            .map_err(|err| CopyFileFailure::after_create(ioe(err)))?;
        if n == 0 {
            break;
        }
        writer
            .write_all(&buf[..n])
            .map_err(|err| CopyFileFailure::after_create(ioe(err)))?;
        copied_bytes = copied_bytes.saturating_add(n as u64);
        reporter(copied_bytes, current_item.clone()).map_err(CopyFileFailure::after_create)?;
    }
    writer
        .flush()
        .map_err(|err| CopyFileFailure::after_create(ioe(err)))?;
    copy_permissions(src, dst).map_err(CopyFileFailure::after_create)?;
    Ok(copied_bytes)
}

pub(crate) fn copy_permissions(src: &Path, dst: &Path) -> Result<(), SymmError> {
    let permissions = fs::metadata(src).map_err(ioe)?.permissions();
    fs::set_permissions(dst, permissions).map_err(ioe)
}
