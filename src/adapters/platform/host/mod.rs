//! 宿主 OS 文件能力：编译期选择实现，静态分发（无 `dyn`）。

mod error;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::path::{Path, PathBuf};

pub use error::{RelocateFailure, format_relocate_failure, map_link_io_error};

pub trait HostFs {
    fn create_link(&self, target: &Path, link: &Path) -> Result<LinkKind, SymmError>;
    fn write_symlink(&self, link: &Path, target: &Path) -> Result<(), SymmError>;
    fn same_volume(&self, a: &Path, b: &Path) -> Result<bool, SymmError>;
    fn relocate_path(&self, src: &Path, dst: &Path) -> Result<(), RelocateFailure>;
    fn snapshot_dir_acl(&self, src_dir: &Path) -> Result<Option<PathBuf>, SymmError>;
    fn restore_dir_acl(&self, dst_dir: &Path, snapshot: &Path) -> Result<(), SymmError>;
}

#[cfg(unix)]
pub use unix::Host;
#[cfg(windows)]
pub use windows::Host;

pub fn host() -> &'static Host {
    static INSTANCE: Host = Host;
    &INSTANCE
}

/// 提权子进程入口（仅 Windows）。
#[cfg(windows)]
pub fn elevated_create_link_entry(
    target: &Path,
    link: &Path,
    link_kind: Option<&str>,
) -> Result<(), SymmError> {
    windows::elevated_create_link_entry(target, link, link_kind)
}

#[cfg(windows)]
pub(crate) use windows::{
    LinkWriteKind, create_link_direct, infer_link_kind_after_elevated, infer_link_write_kind,
    infer_repoint_write_kind, needs_link_elevation, write_link_kind_direct, write_symlink_direct,
};
