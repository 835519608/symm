use crate::domain::error::SymmError;
use crate::infra::errors::io_map::ioe;
use crate::infra::fs::migration_service::MigrationEvent;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn copy_dir_tree_with_progress<F>(
    src: &Path,
    dst: &Path,
    reporter: &mut F,
) -> Result<(), SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    let mut total_bytes: u64 = 0;
    for entry in WalkDir::new(src).follow_links(false) {
        let entry = entry.map_err(|e| SymmError::IoError {
            message: format!("扫描迁移内容失败：{e}"),
        })?;
        let meta = fs::symlink_metadata(entry.path()).map_err(ioe)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_file() {
            total_bytes = total_bytes.saturating_add(meta.len());
        }
    }

    let mut copied_bytes: u64 = 0;
    for entry in WalkDir::new(src).follow_links(false) {
        let entry = entry.map_err(|e| SymmError::IoError {
            message: format!("扫描迁移内容失败：{e}"),
        })?;

        let src_path = entry.path();
        if src_path == src {
            continue;
        }

        let rel = src_path.strip_prefix(src).unwrap_or(src_path);
        let dst_path = dst.join(rel);

        let meta = fs::symlink_metadata(src_path).map_err(ioe)?;
        let file_type = meta.file_type();

        if file_type.is_symlink() {
            let target = fs::read_link(src_path).map_err(ioe)?;
            ensure_parent_dir(&dst_path)?;
            create_symlink_like(src, dst, src_path, &target, &dst_path)?;
            continue;
        }

        if meta.is_dir() {
            fs::create_dir_all(&dst_path).map_err(ioe)?;
            continue;
        }

        if meta.is_file() {
            ensure_parent_dir(&dst_path)?;
            copied_bytes =
                copy_file_with_progress(src_path, &dst_path, copied_bytes, total_bytes, reporter)?;
            continue;
        }
    }

    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<(), SymmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(ioe)?;
    }
    Ok(())
}

fn copy_file_with_progress<F>(
    src: &Path,
    dst: &Path,
    mut copied_bytes: u64,
    total_bytes: u64,
    reporter: &mut F,
) -> Result<u64, SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    let mut reader = fs::File::open(src).map_err(ioe)?;
    let mut writer = fs::File::create(dst).map_err(ioe)?;

    let current_item = src.file_name().map(|s| s.to_string_lossy().to_string());

    let mut buf = vec![0u8; 8 * 1024 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(ioe)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n]).map_err(ioe)?;
        copied_bytes = copied_bytes.saturating_add(n as u64);
        reporter(MigrationEvent::Copying {
            copied_bytes,
            total_bytes,
            current_item: current_item.clone(),
        })?;
    }
    writer.flush().map_err(ioe)?;
    Ok(copied_bytes)
}

fn create_symlink_like(
    src_root: &Path,
    dst_root: &Path,
    src_link: &Path,
    target: &Path,
    dst_link: &Path,
) -> Result<(), SymmError> {
    let rebased_target = rebase_internal_target(src_root, dst_root, src_link, target);

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let _ = src_link;
        symlink(&rebased_target, dst_link).map_err(ioe)?;
        Ok(())
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::{symlink_dir, symlink_file};

        // 尝试判断链接指向的类型；若目标不存在（常见于损坏的目录 junction），默认按目录链接处理
        let is_dir_link = match fs::metadata(src_link) {
            Ok(m) => m.is_dir(),
            Err(_) => true,
        };

        if is_dir_link {
            symlink_dir(&rebased_target, dst_link).map_err(ioe)?;
        } else {
            symlink_file(&rebased_target, dst_link).map_err(ioe)?;
        }
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (src_link, target, dst_link);
        Err(SymmError::InvalidArgument {
            message: "当前平台不支持复制符号链接".to_string(),
        })
    }
}

fn rebase_internal_target(
    src_root: &Path,
    dst_root: &Path,
    src_link: &Path,
    raw_target: &Path,
) -> PathBuf {
    let resolved = if raw_target.is_absolute() {
        raw_target.to_path_buf()
    } else {
        src_link.parent().unwrap_or(src_root).join(raw_target)
    };

    if let Ok(rel) = resolved.strip_prefix(src_root) {
        return dst_root.join(rel);
    }

    raw_target.to_path_buf()
}
