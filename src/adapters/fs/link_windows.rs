//! Windows：建链失败时按需 UAC（策略层，非 OS API）。

use crate::adapters::platform::elevate;
use crate::adapters::platform::fs::{
    create_link_direct, infer_link_kind_after_elevated, needs_link_elevation, write_symlink_direct,
};
use crate::adapters::platform::privilege;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::path::Path;

pub fn create_link(target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
    try_direct_or_elevate(
        || create_link_direct(target, link),
        || {
            elevate::run_elevated_link(target, link)?;
            infer_link_kind_after_elevated(target, link)
        },
    )
}

pub fn write_symlink(link: &Path, target: &Path) -> Result<(), SymmError> {
    try_direct_or_elevate(
        || write_symlink_direct(link, target),
        || elevate::run_elevated_link(target, link),
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
