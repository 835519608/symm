//! 创建软链接（Unix 直调 platform；Windows 含按需提权策略）。

#[cfg(not(windows))]
use crate::adapters::platform::{PlatformFs, fs_platform};
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::path::Path;

pub fn create_link(target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
    #[cfg(windows)]
    {
        crate::adapters::fs::link_windows::create_link(target, link)
    }
    #[cfg(not(windows))]
    {
        fs_platform().create_link(target, link)
    }
}

pub fn write_symlink(link: &Path, target: &Path) -> Result<(), SymmError> {
    #[cfg(windows)]
    {
        crate::adapters::fs::link_windows::write_symlink(link, target)
    }
    #[cfg(not(windows))]
    {
        fs_platform().write_symlink(link, target)
    }
}
