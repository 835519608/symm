use super::{
    MigrationEvent,
    copy_file::{COPY_BUFFER_SIZE, copy_permissions, copy_regular_file_with_progress},
    rebase,
};
use crate::adapters::errors::io::ioe;
use crate::domain::error::SymmError;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

pub fn copy_dir_tree_with_progress<F>(
    src: &Path,
    dst: &Path,
    reporter: &mut F,
) -> Result<(), SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    let mut copied_bytes: u64 = 0;
    let mut files_copied: u64 = 0;
    let mut deferred_symlinks: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut deferred_dir_permissions: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut buf = vec![0u8; COPY_BUFFER_SIZE];

    let mut entries = WalkDir::new(src).follow_links(false).into_iter();
    while let Some(entry) = entries.next() {
        let entry = entry.map_err(|e| SymmError::IoError {
            message: format!("扫描迁移内容失败：{e}"),
        })?;

        let src_path = entry.path();
        if src_path == src {
            continue;
        }

        let rel = src_path.strip_prefix(src).unwrap_or(src_path);
        let dst_path = dst.join(rel);
        let file_type = entry.file_type();

        if !file_type.is_file() && rebase::link_kind_at(src_path)?.is_some() {
            deferred_symlinks.push((src_path.to_path_buf(), dst_path));
            if file_type.is_dir() {
                entries.skip_current_dir();
            }
            continue;
        }

        if file_type.is_dir() {
            fs::create_dir_all(&dst_path).map_err(ioe)?;
            deferred_dir_permissions.push((src_path.to_path_buf(), dst_path));
            continue;
        }

        if file_type.is_file() {
            ensure_parent_dir(&dst_path)?;
            let current_item = src_path
                .file_name()
                .map(|s| Arc::<str>::from(s.to_string_lossy()));
            copied_bytes = copy_regular_file_with_progress(
                src_path,
                &dst_path,
                copied_bytes,
                current_item.clone(),
                &mut buf,
                &mut |copied_bytes, current_item| {
                    reporter(MigrationEvent::Copying {
                        copied_bytes,
                        files_copied,
                        current_item,
                    })
                },
            )?;
            files_copied += 1;
            reporter(MigrationEvent::Copying {
                copied_bytes,
                files_copied,
                current_item,
            })?;
        }
    }

    for (src_link, dst_link) in deferred_symlinks {
        ensure_parent_dir(&dst_link)?;
        rebase::recreate_symlink(&src_link, &dst_link, Some((src, dst)))?;
        files_copied += 1;
        reporter(MigrationEvent::Copying {
            copied_bytes,
            files_copied,
            current_item: dst_link
                .file_name()
                .map(|s| Arc::<str>::from(s.to_string_lossy())),
        })?;
    }

    apply_deferred_dir_permissions(deferred_dir_permissions)?;
    Ok(())
}

fn apply_deferred_dir_permissions(mut dirs: Vec<(PathBuf, PathBuf)>) -> Result<(), SymmError> {
    dirs.sort_by(|a, b| b.1.components().count().cmp(&a.1.components().count()));
    for (src, dst) in dirs {
        copy_permissions(&src, &dst)?;
    }
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<(), SymmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(ioe)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::copy_dir_tree_with_progress;
    use crate::adapters::migrate::MigrationEvent;
    use crate::domain::error::SymmError;
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
    fn copy_dir_tree_rebases_internal_symlink() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src_dir");
        let dst = temp.path().join("dst_dir");
        fs::create_dir_all(src.join("data")).expect("dir");
        fs::write(src.join("data").join("x.txt"), "payload").expect("write");
        symlink_file(&src.join("data").join("x.txt"), &src.join("abs_link"));

        copy_dir_tree_with_progress(&src, &dst, &mut |_event| Ok(())).expect("copy");

        let target = fs::read_link(dst.join("abs_link")).expect("read link");
        assert_eq!(target, dst.join("data").join("x.txt"));
        assert_eq!(
            fs::read_to_string(dst.join("abs_link")).expect("read"),
            "payload"
        );
    }

    #[test]
    fn copy_dir_tree_reports_copy_progress_without_prescan() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src_dir");
        let dst = temp.path().join("dst_dir");
        fs::create_dir_all(&src).expect("dir");
        fs::write(src.join("a.txt"), vec![0u8; 1024]).expect("write");

        let mut max_bytes = 0;
        let mut max_files = 0;
        copy_dir_tree_with_progress(&src, &dst, &mut |event| {
            if let MigrationEvent::Copying {
                copied_bytes,
                files_copied,
                ..
            } = event
            {
                max_bytes = max_bytes.max(copied_bytes);
                max_files = max_files.max(files_copied);
            }
            Ok(())
        })
        .expect("copy");

        assert_eq!(max_bytes, 1024);
        assert_eq!(max_files, 1);
    }

    #[test]
    fn copy_dir_tree_aborts_when_reporter_fails() {
        let temp = tempdir().expect("temp dir");
        let src = temp.path().join("src_dir");
        let dst = temp.path().join("dst_dir");
        fs::create_dir_all(&src).expect("dir");
        fs::write(src.join("a.txt"), "x").expect("write");

        let err = copy_dir_tree_with_progress(&src, &dst, &mut |_event| {
            Err(SymmError::IoError {
                message: "stop".to_string(),
            })
        })
        .expect_err("abort");

        assert!(
            matches!(err, SymmError::IoError { ref message } if message == "stop"),
            "unexpected: {err:?}"
        );
    }
}
