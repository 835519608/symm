use super::HostFs;
use super::error::{RelocateFailure, map_link_io_error};
use crate::adapters::errors::io::ioe;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

pub struct Host;

impl HostFs for Host {
    fn create_link(&self, target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
        symlink(target, link).map_err(map_link_io_error)?;
        Ok(LinkKind::Symlink)
    }

    fn write_symlink(&self, link: &Path, target: &Path) -> Result<(), SymmError> {
        symlink(target, link).map_err(ioe)?;
        Ok(())
    }

    fn same_volume(&self, a: &Path, b: &Path) -> Result<bool, SymmError> {
        use std::os::unix::fs::MetadataExt;
        let a_meta = fs::metadata(a).map_err(ioe)?;
        let b_meta = fs::metadata(b).map_err(ioe)?;
        Ok(a_meta.dev() == b_meta.dev())
    }

    fn relocate_path(&self, src: &Path, dst: &Path) -> Result<(), RelocateFailure> {
        rename_no_replace(src, dst)
    }

    fn snapshot_dir_acl(&self, _src_dir: &Path) -> Result<Option<PathBuf>, SymmError> {
        Ok(None)
    }

    fn restore_dir_acl(&self, _dst_dir: &Path, _snapshot: &Path) -> Result<(), SymmError> {
        Ok(())
    }
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn rename_no_replace(src: &Path, dst: &Path) -> Result<(), RelocateFailure> {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int, c_long};
    use std::os::unix::ffi::OsStrExt;

    const AT_FDCWD: c_int = -100;
    const RENAME_NOREPLACE: u32 = 1;

    #[cfg(target_arch = "x86_64")]
    const SYS_RENAMEAT2: c_long = 316;
    #[cfg(target_arch = "aarch64")]
    const SYS_RENAMEAT2: c_long = 276;

    unsafe extern "C" {
        fn syscall(num: c_long, ...) -> c_long;
    }

    let src = CString::new(src.as_os_str().as_bytes()).map_err(|_| {
        RelocateFailure::from_io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "路径包含 NUL 字节",
        ))
    })?;
    let dst = CString::new(dst.as_os_str().as_bytes()).map_err(|_| {
        RelocateFailure::from_io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "路径包含 NUL 字节",
        ))
    })?;

    let result = unsafe {
        syscall(
            SYS_RENAMEAT2,
            AT_FDCWD,
            src.as_ptr() as *const c_char,
            AT_FDCWD,
            dst.as_ptr() as *const c_char,
            RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    if matches!(
        err.raw_os_error(),
        Some(38) | Some(22) | Some(95) | Some(89)
    ) {
        return Err(RelocateFailure::no_replace_unsupported());
    }
    Err(RelocateFailure::from_io(err))
}

#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
fn rename_no_replace(_src: &Path, _dst: &Path) -> Result<(), RelocateFailure> {
    Err(RelocateFailure::no_replace_unsupported())
}
