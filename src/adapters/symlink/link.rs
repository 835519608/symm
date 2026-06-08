//! 建链 / 写软链（Unix 直调 platform；Windows 走 `windows` 策略层）。

#[cfg(not(windows))]
use crate::adapters::platform::{HostFs, host_platform};
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::path::Path;

#[derive(Clone, Copy)]
pub(crate) struct LinkRecreateSpec {
    #[cfg(windows)]
    kind: crate::adapters::platform::host::LinkWriteKind,
}

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

pub(crate) fn write_symlink_like(
    src_link: &Path,
    link: &Path,
    target: &Path,
) -> Result<(), SymmError> {
    let spec = capture_recreate_spec(src_link)?;
    write_symlink_from_spec(spec, link, target)
}

pub(crate) fn capture_recreate_spec(src_link: &Path) -> Result<LinkRecreateSpec, SymmError> {
    #[cfg(windows)]
    {
        Ok(LinkRecreateSpec {
            kind: crate::adapters::platform::host::infer_link_write_kind(src_link)?,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = src_link;
        Ok(LinkRecreateSpec {})
    }
}

pub(crate) fn write_symlink_from_spec(
    spec: LinkRecreateSpec,
    link: &Path,
    target: &Path,
) -> Result<(), SymmError> {
    #[cfg(windows)]
    {
        super::windows::write_symlink_with_kind(spec.kind, link, target)
    }
    #[cfg(not(windows))]
    {
        let _ = spec;
        host_platform().write_symlink(link, target)
    }
}
