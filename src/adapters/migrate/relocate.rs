//! 同盘移动软链失败时，经 `symlink::write_symlink` 重建（含 Windows UAC 策略）。

use crate::adapters::errors::io_map::ioe;
use crate::adapters::paths::{path_ops, rebase_paths};
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use std::fs;
use std::path::Path;

pub fn relocate_symlink(src: &Path, dst: &Path) -> Result<(), SymmError> {
    if dst.exists() {
        path_ops::remove_path_any(dst)?;
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(ioe)?;
    }
    let link_target = fs::read_link(src).map_err(ioe)?;
    let roots = rebase_paths::source_roots(src);
    let rebased = rebase_paths::internal_target(dst, src, &link_target, &roots);
    symlink::write_symlink(dst, &rebased)?;
    path_ops::remove_path_any(src)?;
    Ok(())
}
