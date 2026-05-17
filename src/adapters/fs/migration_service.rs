use crate::adapters::errors::io_map::ioe;
use crate::adapters::fs::path_ops;
use crate::adapters::fs::rebase::rebase_symlinks_in_tree;
use crate::adapters::platform::{PlatformFs, format_relocate_failure, fs_platform};
use crate::domain::error::SymmError;
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
        files_copied: u64,
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
        rebase_symlinks_in_tree(dst, src)?;
        return Ok(());
    }

    if let Some(acl_file) = fs_platform().snapshot_dir_acl(src)? {
        super::copy_with_progress::copy_path_with_progress(src, dst, reporter)?;
        let _ = fs_platform().restore_dir_acl(dst, &acl_file);
    } else {
        super::copy_with_progress::copy_path_with_progress(src, dst, reporter)?;
    }

    reporter(MigrationEvent::RemovingSource {
        source: src.display().to_string(),
    })?;
    if let Err(remove_err) = path_ops::remove_path_any(src) {
        return Err(SymmError::IoError {
            message: format!(
                "跨磁盘复制已完成但无法删除源路径：{remove_err}（目标已存在于 {}，请人工处理）",
                dst.display()
            ),
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

    fs_platform().same_volume(src, dst_parent)
}

pub fn fs_extra_error(e: fs_extra::error::Error) -> SymmError {
    SymmError::IoError {
        message: e.to_string(),
    }
}

pub fn move_path_with_retry(src: &Path, dst: &Path, role: &str) -> Result<(), SymmError> {
    fs_platform()
        .relocate_path(src, dst)
        .map_err(|failure| format_relocate_failure(role, failure))
}

#[cfg(test)]
mod tests {
    use super::{MigrationEvent, migrate_path, move_path_without_progress};
    use crate::adapters::fs::copy_with_progress::copy_path_with_progress;
    use crate::adapters::fs::rebase;
    use crate::domain::error::SymmError;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[cfg(windows)]
    fn symlink_file(target: &Path, link: &Path) {
        std::os::windows::fs::symlink_file(target, link).expect("symlink");
    }

    #[cfg(unix)]
    fn symlink_file(target: &Path, link: &Path) {
        symlink(target, link).expect("symlink");
    }

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
    fn migrate_path_rebases_internal_symlink_on_same_volume() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("agent");
        let dst = temp.path().join("agent1");
        fs::create_dir_all(src.join("data")).expect("dir");
        fs::write(src.join("data").join("x.txt"), "ok").expect("write");
        symlink_file(&src.join("data").join("x.txt"), &src.join("lnk"));

        migrate_path(&src, &dst, &mut |_event| Ok(())).expect("migrate");

        assert!(!src.exists());
        assert_eq!(
            fs::read_link(dst.join("lnk")).expect("read"),
            dst.join("data").join("x.txt")
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

    #[test]
    fn rebase_noop_for_tree_without_symlinks() {
        let temp = tempdir().expect("temp dir");
        let root = temp.path().join("plain");
        fs::create_dir_all(root.join("nested")).expect("dir");
        fs::write(root.join("nested").join("f.txt"), "x").expect("write");
        rebase::rebase_symlinks_in_tree(&root, &root).expect("no-op rebase");
        assert_eq!(
            fs::read_to_string(root.join("nested").join("f.txt")).expect("read"),
            "x"
        );
    }
}
