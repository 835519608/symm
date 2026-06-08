//! Windows：建链失败时按需 UAC（策略层，非 OS API）。

use crate::adapters::platform::host::{
    create_link_direct, infer_link_kind_after_elevated, infer_link_write_kind,
    needs_link_elevation, write_link_kind_direct, write_symlink_direct,
};
use crate::adapters::platform::privilege;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::path::Path;

pub fn create_link(target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
    try_direct_or_elevate(
        || create_link_direct(target, link),
        || {
            privilege::spawn_elevated_create_link(target, link)?;
            infer_link_kind_after_elevated(target, link)
        },
    )
}

pub fn write_symlink(link: &Path, target: &Path) -> Result<(), SymmError> {
    try_direct_or_elevate(
        || write_symlink_direct(link, target),
        || privilege::spawn_elevated_create_link(target, link),
    )
}

pub fn write_symlink_like(src_link: &Path, link: &Path, target: &Path) -> Result<(), SymmError> {
    let kind = infer_link_write_kind(src_link)?;
    write_symlink_with_kind(kind, link, target)
}

pub(crate) fn write_symlink_with_kind(
    kind: crate::adapters::platform::host::LinkWriteKind,
    link: &Path,
    target: &Path,
) -> Result<(), SymmError> {
    try_direct_or_elevate(
        || write_link_kind_direct(kind, target, link),
        || privilege::spawn_elevated_create_link_with_kind(target, link, kind.as_arg()),
    )
}

fn try_direct_or_elevate<T>(
    direct: impl FnOnce() -> Result<T, SymmError>,
    on_elevated: impl FnOnce() -> Result<T, SymmError>,
) -> Result<T, SymmError> {
    direct().or_else(|err| {
        if privilege::is_privileged() || !needs_link_elevation(&err) {
            return Err(err);
        }
        on_elevated()
    })
}
