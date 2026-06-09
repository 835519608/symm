//! 同盘移动软链失败时，经 `symlink::write_symlink` 重建（含 Windows UAC 策略）。

use crate::adapters::errors::io::ioe;
use crate::adapters::paths::{presence, rebase_paths, remove};
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use std::fs;
use std::path::Path;

pub fn relocate_symlink(src: &Path, dst: &Path) -> Result<(), SymmError> {
    relocate_symlink_with_rebase(src, dst, true)
}

pub fn relocate_symlink_preserving_target(src: &Path, dst: &Path) -> Result<(), SymmError> {
    relocate_symlink_with_rebase(src, dst, false)
}

fn relocate_symlink_with_rebase(
    src: &Path,
    dst: &Path,
    rebase_target: bool,
) -> Result<(), SymmError> {
    if presence::path_itself_exists(dst)? {
        return Err(SymmError::InvalidArgument {
            message: format!("迁移失败：目标路径已存在：{}", dst.display()),
        });
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(ioe)?;
    }
    let link_target = fs::read_link(src).map_err(ioe)?;
    let rebased = if rebase_target {
        let roots = rebase_paths::source_roots(src);
        rebase_paths::internal_target(dst, src, &link_target, &roots)
    } else {
        link_target
    };
    symlink::write_symlink_like(src, dst, &rebased)?;
    remove::remove_any(src)?;
    Ok(())
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::relocate_symlink_preserving_target;
    use std::fs;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    #[test]
    fn relocate_symlink_preserving_target_keeps_raw_target() {
        let temp = tempdir().expect("temp dir");
        let src_root = temp.path().join("src");
        let dst_root = temp.path().join("dst");
        fs::create_dir_all(&src_root).expect("src dir");
        fs::create_dir_all(&dst_root).expect("dst dir");
        fs::write(src_root.join("data.txt"), "payload").expect("write data");
        let link = src_root.join("link");
        symlink(src_root.join("data.txt"), &link).expect("symlink");
        let dst = dst_root.join("link");

        relocate_symlink_preserving_target(&link, &dst).expect("relocate preserving target");

        assert_eq!(
            fs::read_link(&dst).expect("read relocated link"),
            src_root.join("data.txt")
        );
    }

    #[test]
    fn relocate_symlink_preserving_target_rejects_existing_destination() {
        let temp = tempdir().expect("temp dir");
        let src_root = temp.path().join("src");
        let dst_root = temp.path().join("dst");
        fs::create_dir_all(&src_root).expect("src dir");
        fs::create_dir_all(&dst_root).expect("dst dir");
        fs::write(src_root.join("data.txt"), "payload").expect("write data");
        let link = src_root.join("link");
        symlink(src_root.join("data.txt"), &link).expect("symlink");
        let dst = dst_root.join("link");
        fs::write(&dst, "existing").expect("write dst");

        relocate_symlink_preserving_target(&link, &dst).expect_err("existing dst should fail");

        assert!(fs::symlink_metadata(&link).is_ok(), "source link remains");
        assert_eq!(fs::read_to_string(&dst).expect("read dst"), "existing");
    }
}
