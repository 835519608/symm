use super::PlatformFs;
use super::error::{RelocateFailure, map_link_io_error};
use crate::adapters::errors::io_map::ioe;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

pub struct Platform;

impl PlatformFs for Platform {
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
        fs::rename(src, dst).map_err(RelocateFailure::from_io)
    }

    fn snapshot_dir_acl(&self, _src_dir: &Path) -> Result<Option<PathBuf>, SymmError> {
        Ok(None)
    }

    fn restore_dir_acl(&self, _dst_dir: &Path, _snapshot: &Path) -> Result<(), SymmError> {
        Ok(())
    }
}
