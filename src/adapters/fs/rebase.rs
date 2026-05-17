use crate::adapters::errors::io_map::ioe;
use crate::adapters::fs::path_ops;
use crate::domain::error::SymmError;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[cfg(unix)]
use crate::adapters::platform::{PlatformFs, fs_platform};

#[cfg(windows)]
use crate::adapters::platform::fs::write_symlink_direct;

pub fn recreate_symlink(
    src_link: &Path,
    dst_link: &Path,
    rebase: Option<(&Path, &Path)>,
) -> Result<(), SymmError> {
    let link_target = fs::read_link(src_link).map_err(ioe)?;
    let rebased_target = match rebase {
        Some((src_root, dst_root)) => {
            let roots = source_roots(src_root);
            internal_target(dst_root, src_link, &link_target, &roots)
        }
        None => link_target,
    };
    write_symlink_at(dst_link, &rebased_target)
}

/// 目录树内是否存在软链接（发现首个即返回，用于避免无意义的 rebase 重写遍历）。
pub fn tree_contains_symlink(root: &Path) -> Result<bool, SymmError> {
    let meta = fs::symlink_metadata(root).map_err(ioe)?;
    if !meta.is_dir() {
        return Ok(false);
    }
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|e| SymmError::IoError {
            message: format!("扫描软链接失败：{e}"),
        })?;
        if entry.path() == root {
            continue;
        }
        if entry.file_type().is_symlink() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// 同盘 `rename` 迁移后，将树内仍指向旧根路径的软链接 rebase 到 `dst_root`。
pub fn rebase_symlinks_in_tree(dst_root: &Path, src_root: &Path) -> Result<(), SymmError> {
    let meta = fs::symlink_metadata(dst_root).map_err(ioe)?;
    if !meta.is_dir() {
        return Ok(());
    }

    let roots = source_roots(src_root);

    for entry in WalkDir::new(dst_root).follow_links(false) {
        let entry = entry.map_err(|e| SymmError::IoError {
            message: format!("扫描软链接 rebase 失败：{e}"),
        })?;
        let link_path = entry.path();
        if link_path == dst_root {
            continue;
        }
        if !entry.file_type().is_symlink() {
            continue;
        }
        let raw = fs::read_link(link_path).map_err(ioe)?;
        let rebased = internal_target(dst_root, link_path, &raw, &roots);
        if rebased.as_os_str() == raw.as_os_str() {
            continue;
        }
        path_ops::remove_path_any(link_path)?;
        write_symlink_at(link_path, &rebased)?;
    }
    Ok(())
}

fn write_symlink_at(link: &Path, target: &Path) -> Result<(), SymmError> {
    #[cfg(windows)]
    {
        return write_symlink_direct(link, target);
    }
    #[cfg(not(windows))]
    {
        fs_platform().write_symlink(link, target)
    }
}

pub(crate) fn internal_target(
    dst_root: &Path,
    src_link: &Path,
    raw_target: &Path,
    source_roots: &[PathBuf],
) -> PathBuf {
    let resolved = if raw_target.is_absolute() {
        raw_target.to_path_buf()
    } else {
        src_link
            .parent()
            .unwrap_or_else(|| {
                source_roots
                    .first()
                    .map(PathBuf::as_path)
                    .unwrap_or(dst_root)
            })
            .join(raw_target)
    };

    for base in source_roots {
        if let Ok(rel) = resolved.strip_prefix(base) {
            return dst_root.join(rel);
        }
    }

    raw_target.to_path_buf()
}

pub(crate) fn source_roots(src_root: &Path) -> Vec<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::{rebase_symlinks_in_tree, recreate_symlink, tree_contains_symlink};
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    fn symlink_file(target: &Path, link: &Path) {
        #[cfg(unix)]
        symlink(target, link).expect("symlink");

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(target, link).expect("symlink");
    }

    #[test]
    fn tree_contains_symlink_false_for_plain_tree() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("plain");
        fs::create_dir_all(root.join("nested")).expect("dir");
        fs::write(root.join("nested").join("f.txt"), "x").expect("write");
        assert!(!tree_contains_symlink(&root).expect("scan"));
    }

    #[test]
    fn tree_contains_symlink_true_when_present() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("with_link");
        fs::create_dir_all(&root).expect("dir");
        fs::write(root.join("f.txt"), "x").expect("write");
        symlink_file(&root.join("f.txt"), &root.join("lnk"));
        assert!(tree_contains_symlink(&root).expect("scan"));
    }

    #[test]
    fn rebase_symlinks_in_tree_rewrites_absolute_targets_under_old_root() {
        let temp = tempdir().expect("temp dir");
        let src_root = temp.path().join("agent");
        let dst_root = temp.path().join("agent1");
        let nested = src_root.join("nested");
        fs::create_dir_all(&nested).expect("create nested");
        fs::write(nested.join("file.txt"), "ok").expect("write file");

        let internal_link = src_root.join("link_to_nested");
        symlink_file(&nested.join("file.txt"), &internal_link);

        fs::rename(&src_root, &dst_root).expect("simulate same-volume rename");
        rebase_symlinks_in_tree(&dst_root, &src_root).expect("rebase symlinks");

        let rebased_target = fs::read_link(dst_root.join("link_to_nested")).expect("read link");
        assert_eq!(rebased_target, dst_root.join("nested").join("file.txt"));
        assert_eq!(
            fs::read_to_string(dst_root.join("link_to_nested")).expect("read through link"),
            "ok"
        );
    }

    #[test]
    fn rebase_symlinks_in_tree_includes_staging_root_alias() {
        let temp = tempdir().expect("temp dir");
        let original = temp.path().join("agent");
        let staging = temp.path().join("agent.__symm_staging__");
        let dst_root = temp.path().join("agent1");
        fs::create_dir_all(original.join("data")).expect("create data");
        fs::write(original.join("data").join("x.txt"), "x").expect("write");

        symlink_file(
            &original.join("data").join("x.txt"),
            &original.join("abs_link"),
        );
        fs::rename(&original, &staging).expect("stage");
        fs::rename(&staging, &dst_root).expect("move to target");

        rebase_symlinks_in_tree(&dst_root, &staging).expect("rebase");
        let target = fs::read_link(dst_root.join("abs_link")).expect("read");
        assert_eq!(target, dst_root.join("data").join("x.txt"));
    }

    #[test]
    fn recreate_symlink_rebases_when_roots_given() {
        let temp = tempdir().expect("temp dir");
        let src_root = temp.path().join("src");
        let dst_root = temp.path().join("dst");
        fs::create_dir_all(&src_root).expect("dir");
        fs::create_dir_all(&dst_root).expect("dir");
        fs::write(src_root.join("f.txt"), "body").expect("write");
        let link = src_root.join("lnk");
        symlink_file(&src_root.join("f.txt"), &link);

        let dst_link = dst_root.join("lnk");
        recreate_symlink(&link, &dst_link, Some((&src_root, &dst_root))).expect("recreate");

        assert_eq!(
            fs::read_link(&dst_link).expect("read"),
            dst_root.join("f.txt")
        );
    }
}
