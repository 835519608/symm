//! 同盘移动软链失败时，经 `symlink::write_symlink` 重建（含 Windows UAC 策略）。

use crate::adapters::errors::io::ioe;
use crate::adapters::paths::{presence, rebase_paths};
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
        link_target.clone()
    };
    let recreate_spec = symlink::capture_recreate_spec(src)?;
    symlink::write_symlink_from_spec(recreate_spec, dst, &rebased)?;
    if let Err(err) = remove_expected_relocated_link(src, recreate_spec, &link_target) {
        let _ = symlink::unlink(dst);
        return Err(err);
    }
    Ok(())
}

fn remove_expected_relocated_link(
    src: &Path,
    recreate_spec: symlink::LinkRecreateSpec,
    expected_target: &Path,
) -> Result<(), SymmError> {
    let meta = fs::symlink_metadata(src).map_err(ioe)?;
    if symlink::kind_from_path_and_metadata(src, &meta)?.is_none() {
        return Err(source_link_changed(src));
    }
    if symlink::capture_recreate_spec(src).map_err(|_| source_link_changed(src))? != recreate_spec {
        return Err(source_link_changed(src));
    }
    let current_target = fs::read_link(src).map_err(ioe)?;
    if current_target != expected_target {
        return Err(source_link_changed(src));
    }
    symlink::unlink(src)
}

fn source_link_changed(src: &Path) -> SymmError {
    SymmError::InvalidArgument {
        message: format!("源链接状态已变化，无法安全清理：{}", src.display()),
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::{relocate_symlink_preserving_target, remove_expected_relocated_link};
    use crate::adapters::symlink as symlink_adapter;
    use crate::domain::error::SymmError;
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

    #[test]
    fn remove_expected_relocated_link_refuses_replaced_entity() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link");
        fs::write(&target, "payload").expect("write data");
        symlink(&target, &link).expect("symlink");
        let spec = symlink_adapter::capture_recreate_spec(&link).expect("capture spec");
        fs::remove_file(&link).expect("remove source link");
        fs::write(&link, "external entity").expect("replace source");

        let err = remove_expected_relocated_link(&link, spec, &target)
            .expect_err("replaced entity should not be removed");

        assert!(
            matches!(err, SymmError::InvalidArgument { ref message } if message.contains("源链接状态已变化")),
            "unexpected error: {err:?}"
        );
        assert_eq!(
            fs::read_to_string(&link).expect("entity should remain"),
            "external entity"
        );
    }
}
