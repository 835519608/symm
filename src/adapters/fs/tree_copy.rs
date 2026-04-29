use crate::adapters::errors::io_map::ioe;
use crate::adapters::fs::migration_service::MigrationEvent;
use crate::domain::error::SymmError;
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
            ensure_parent_dir(&dst_path)?;
            recreate_symlink(src_path, &dst_path, Some((src, dst)))?;
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

pub fn recreate_symlink(
    src_link: &Path,
    dst_link: &Path,
    rebase: Option<(&Path, &Path)>,
) -> Result<(), SymmError> {
    let link_target = fs::read_link(src_link).map_err(ioe)?;
    let rebased_target = match rebase {
        Some((src_root, dst_root)) => {
            rebase_internal_target(src_root, dst_root, src_link, &link_target)
        }
        None => link_target.clone(),
    };

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
        let _ = (src_link, dst_link);
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

    for base in rebase_source_roots(src_root) {
        if let Ok(rel) = resolved.strip_prefix(base) {
            return dst_root.join(rel);
        }
    }

    raw_target.to_path_buf()
}

fn rebase_source_roots(src_root: &Path) -> Vec<PathBuf> {
    let mut roots = vec![src_root.to_path_buf()];
    let Some(name) = src_root.file_name().and_then(|n| n.to_str()) else {
        return roots;
    };
    const STAGING_SUFFIX: &str = ".__symm_staging__";
    if let Some(original_name) = name.strip_suffix(STAGING_SUFFIX) {
        let mut original = src_root.to_path_buf();
        original.set_file_name(original_name);
        roots.push(original);
    }
    roots
}
