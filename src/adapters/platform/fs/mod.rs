//! 平台文件系统能力：编译期选择实现，静态分发（无 `dyn`）。

mod error;
mod unsupported;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::path::{Path, PathBuf};

pub use error::{RelocateFailure, format_relocate_failure, map_link_io_error};

pub trait PlatformFs {
    fn create_link(&self, target: &Path, link: &Path) -> Result<LinkKind, SymmError>;
    fn write_symlink(&self, link: &Path, target: &Path) -> Result<(), SymmError>;
    fn same_volume(&self, a: &Path, b: &Path) -> Result<bool, SymmError>;
    fn relocate_path(&self, src: &Path, dst: &Path) -> Result<(), RelocateFailure>;
    fn snapshot_dir_acl(&self, src_dir: &Path) -> Result<Option<PathBuf>, SymmError>;
    fn restore_dir_acl(&self, dst_dir: &Path, snapshot: &Path) -> Result<(), SymmError>;
}

#[cfg(unix)]
pub use unix::Platform;
#[cfg(not(any(unix, windows)))]
pub use unsupported::Platform;
#[cfg(windows)]
pub use windows::Platform;

pub fn fs() -> &'static Platform {
    static INSTANCE: Platform = Platform;
    &INSTANCE
}
