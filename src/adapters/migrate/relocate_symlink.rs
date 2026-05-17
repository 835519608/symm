//! 同盘移动软链失败时，经 `symlink::write_symlink` 重建（含 Windows UAC 策略）。

use crate::adapters::errors::io::ioe;
use crate::adapters::paths::{rebase_paths, remove};
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use std::fs;
use std::path::Path;

pub fn relocate_symlink(src: &Path, dst: &Path) -> Result<(), SymmError> {
    if dst.exists() {
        remove::remove_any(dst)?;
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(ioe)?;
    }
    let link_target = fs::read_link(src).map_err(ioe)?;
    let roots = rebase_paths::source_roots(src);
    let rebased = rebase_paths::internal_target(dst, src, &link_target, &roots);
    symlink::write_symlink(dst, &rebased)?;
    remove::remove_any(src)?;
    Ok(())
}
