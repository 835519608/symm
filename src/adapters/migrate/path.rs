use super::relocate_symlink::{relocate_symlink, relocate_symlink_preserving_target};
use super::{copy_file, rebase};
use crate::adapters::paths::{presence, remove};
use crate::adapters::platform::{HostFs, format_relocate_failure, host_platform};
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use std::fs;
use std::path::Path;
use std::sync::Arc;

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
        current_item: Option<Arc<str>>,
    },
    RemovingSource {
        source: String,
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
    ensure_destination_missing(dst)?;

    if can_use_fast_move(src, dst)? {
        reporter(MigrationEvent::FastMove {
            source: src.display().to_string(),
            target: dst.display().to_string(),
        })?;
        if try_move_path_with_retry(src, dst, "迁移项")? {
            if !path_is_link(dst)? {
                rebase::rebase_symlinks_in_tree(dst, src).map_err(|err| {
                    SymmError::EntityMovedButPostMoveFailed {
                        source_path: src.display().to_string(),
                        target_path: dst.display().to_string(),
                        message: format!(
                            "同盘移动已完成，但迁移后重写内部链接失败：{err}；源路径已不存在，请检查 target 后手动处理"
                        ),
                    }
                })?;
            }
            return Ok(());
        }
    }

    if let Some(acl_file) = host_platform().snapshot_dir_acl(src)? {
        let acl_file = TempAclSnapshot::new(acl_file);
        copy_file::copy_path_with_progress(src, dst, reporter)?;
        if let Err(err) = host_platform().restore_dir_acl(dst, acl_file.path()) {
            let _ = remove::remove_any(dst);
            return Err(err);
        }
    } else {
        copy_file::copy_path_with_progress(src, dst, reporter)?;
    }

    reporter(MigrationEvent::RemovingSource {
        source: src.display().to_string(),
    })?;
    if let Err(remove_err) = remove::remove_any(src) {
        return Err(SymmError::EntityCopiedButSourceCleanupFailed {
            source_path: src.display().to_string(),
            target_path: dst.display().to_string(),
            message: format!(
                "复制迁移完成，但源路径删不掉：{remove_err}；已保留 target 副本，请确认源路径残留后手动处理"
            ),
        });
    }
    Ok(())
}

fn ensure_destination_missing(dst: &Path) -> Result<(), SymmError> {
    if !presence::path_itself_exists(dst)? {
        return Ok(());
    }
    Err(SymmError::InvalidArgument {
        message: format!("迁移失败：目标路径已存在：{}", dst.display()),
    })
}

fn path_is_link(path: &Path) -> Result<bool, SymmError> {
    let meta = fs::symlink_metadata(path).map_err(|e| SymmError::IoError {
        message: format!("无法读取迁移路径元数据：{e}"),
    })?;
    Ok(symlink::kind_from_path_and_metadata(path, &meta)?.is_some())
}

#[cfg(test)]
fn move_path_without_progress(src: &Path, dst: &Path) -> Result<(), SymmError> {
    let mut noop = |_event: MigrationEvent| Ok(());
    migrate_path(src, dst, &mut noop)
}

pub fn can_use_fast_move(src: &Path, dst: &Path) -> Result<bool, SymmError> {
    let dst_parent = dst.parent().ok_or_else(|| SymmError::InvalidArgument {
        message: "无法解析目标父目录".to_string(),
    })?;
    if !presence::target_exists(dst_parent)? {
        return Err(SymmError::TargetNotFound {
            path: dst_parent.display().to_string(),
        });
    }

    host_platform().same_volume(src, dst_parent)
}

fn try_move_path_with_retry(src: &Path, dst: &Path, role: &str) -> Result<bool, SymmError> {
    match host_platform().relocate_path(src, dst) {
        Ok(()) => Ok(true),
        Err(failure) if failure.no_replace_unsupported => Ok(false),
        Err(failure) if failure.symlink_needs_recreate => {
            let relocate = if path_is_link(src)? {
                relocate_symlink_preserving_target(src, dst)
            } else {
                relocate_symlink(src, dst)
            };
            relocate.map_err(|inner| SymmError::IoError {
                message: format!("无法移动 {role}：{inner}"),
            })?;
            Ok(true)
        }
        Err(failure) => Err(format_relocate_failure(role, failure)),
    }
}

struct TempAclSnapshot {
    path: std::path::PathBuf,
}

impl TempAclSnapshot {
    fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempAclSnapshot {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::{MigrationEvent, migrate_path, move_path_without_progress};
    use crate::adapters::migrate::copy_file::copy_path_with_progress;
    use crate::adapters::migrate::rebase;
    use crate::domain::error::SymmError;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

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
    fn migrate_path_rejects_existing_destination_without_overwrite() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src.txt");
        let dst = temp.path().join("dst.txt");
        fs::write(&src, "source").expect("write source");
        fs::write(&dst, "destination").expect("write destination");

        migrate_path(&src, &dst, &mut |_event| Ok(())).expect_err("existing dst should fail");

        assert_eq!(fs::read_to_string(&src).expect("read source"), "source");
        assert_eq!(
            fs::read_to_string(&dst).expect("read destination"),
            "destination"
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

    #[cfg(unix)]
    #[test]
    fn fast_move_rebase_failure_reports_half_moved_entity() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src-dir");
        let dst = temp.path().join("dst-dir");
        let protected = src.join("protected");
        fs::create_dir_all(src.join("data")).expect("data dir");
        fs::create_dir_all(&protected).expect("protected dir");
        fs::write(src.join("data").join("x.txt"), "ok").expect("write data");
        symlink(src.join("data").join("x.txt"), protected.join("link")).expect("symlink");
        fs::set_permissions(&protected, fs::Permissions::from_mode(0o555))
            .expect("make protected dir readonly");

        let err = migrate_path(&src, &dst, &mut |_event| Ok(()))
            .expect_err("rebase failure after fast move should be reported");

        let moved_protected = dst.join("protected");
        if moved_protected.exists() {
            fs::set_permissions(&moved_protected, fs::Permissions::from_mode(0o755))
                .expect("restore moved protected permissions");
        }
        assert!(
            matches!(err, SymmError::EntityMovedButPostMoveFailed { .. }),
            "unexpected error: {err:?}"
        );
        assert!(
            !src.exists() && dst.exists(),
            "fast move already moved the entity when rebase failed"
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
    fn copy_path_with_progress_rejects_existing_file_without_overwrite() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src.txt");
        let dst = temp.path().join("dst.txt");
        fs::write(&src, "source").expect("write source");
        fs::write(&dst, "destination").expect("write destination");

        copy_path_with_progress(&src, &dst, &mut |_event| Ok(()))
            .expect_err("existing file should fail");

        assert_eq!(fs::read_to_string(&src).expect("read source"), "source");
        assert_eq!(
            fs::read_to_string(&dst).expect("read destination"),
            "destination"
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_path_with_progress_preserves_file_mode() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src.sh");
        let dst = temp.path().join("dst.sh");
        fs::write(&src, "#!/bin/sh\n").expect("write source");
        fs::set_permissions(&src, fs::Permissions::from_mode(0o755)).expect("chmod source");

        copy_path_with_progress(&src, &dst, &mut |_event| Ok(())).expect("copy file");

        let mode = fs::metadata(&dst)
            .expect("dst metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o755);
    }

    #[cfg(unix)]
    #[test]
    fn copy_path_with_progress_preserves_top_level_symlink() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let src = temp.path().join("src-link");
        let dst = temp.path().join("dst-link");
        fs::write(&target, "payload").expect("write target");
        symlink(&target, &src).expect("symlink");

        copy_path_with_progress(&src, &dst, &mut |_event| Ok(())).expect("copy symlink");

        assert_eq!(fs::read_link(&dst).expect("read copied symlink"), target);
        assert_eq!(
            fs::read_to_string(&dst).expect("read through copied symlink"),
            "payload"
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_path_with_progress_cleans_top_level_symlink_when_reporter_aborts() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let src = temp.path().join("src-link");
        let dst = temp.path().join("dst-link");
        fs::write(&target, "payload").expect("write target");
        symlink(&target, &src).expect("symlink");

        let err = copy_path_with_progress(&src, &dst, &mut |_event| {
            Err(SymmError::IoError {
                message: "stop".to_string(),
            })
        })
        .expect_err("reporter abort should stop top-level symlink copy");

        assert!(
            matches!(err, SymmError::IoError { ref message } if message == "stop"),
            "unexpected error: {err:?}"
        );
        assert!(
            fs::symlink_metadata(&dst).is_err(),
            "partial symlink destination should be cleaned"
        );
        assert!(fs::symlink_metadata(&src).is_ok(), "source should remain");
    }

    #[cfg(unix)]
    #[test]
    fn copy_path_with_progress_preserves_directory_modes() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src");
        let nested = src.join("nested");
        let dst = temp.path().join("dst");
        fs::create_dir_all(&nested).expect("create dirs");
        fs::write(nested.join("file.txt"), "payload").expect("write nested file");
        fs::set_permissions(&src, fs::Permissions::from_mode(0o555)).expect("chmod root");
        fs::set_permissions(&nested, fs::Permissions::from_mode(0o555)).expect("chmod nested");

        copy_path_with_progress(&src, &dst, &mut |_event| Ok(())).expect("copy dir");

        let root_mode = fs::metadata(&dst)
            .expect("dst metadata")
            .permissions()
            .mode()
            & 0o777;
        let nested_mode = fs::metadata(dst.join("nested"))
            .expect("nested metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            fs::read_to_string(dst.join("nested").join("file.txt")).expect("read copied file"),
            "payload"
        );
        assert_eq!(root_mode, 0o555);
        assert_eq!(nested_mode, 0o555);
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
