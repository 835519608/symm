//! 建链 / 写软链（Unix 直调 platform；Windows 走 `windows` 策略层）。

#[cfg(not(windows))]
use crate::adapters::platform::{HostFs, host_platform};
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::path::Path;

pub fn create_link(target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
    #[cfg(windows)]
    {
        super::windows::create_link(target, link)
    }
    #[cfg(not(windows))]
    {
        host_platform().create_link(target, link)
    }
}

pub fn write_symlink(link: &Path, target: &Path) -> Result<(), SymmError> {
    #[cfg(windows)]
    {
        super::windows::write_symlink(link, target)
    }
    #[cfg(not(windows))]
    {
        host_platform().write_symlink(link, target)
    }
}
