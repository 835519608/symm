use super::PlatformFs;
use super::error::RelocateFailure;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub struct Platform;

impl PlatformFs for Platform {
    fn create_link(&self, _target: &Path, _link: &Path) -> Result<LinkKind, SymmError> {
        Err(SymmError::InvalidArgument {
            message: "不支持的平台".to_string(),
        })
    }

    fn write_symlink(&self, _link: &Path, _target: &Path) -> Result<(), SymmError> {
        Err(SymmError::InvalidArgument {
            message: "不支持的平台".to_string(),
        })
    }

    fn same_volume(&self, _a: &Path, _b: &Path) -> Result<bool, SymmError> {
        Ok(false)
    }

    fn relocate_path(&self, _src: &Path, _dst: &Path) -> Result<(), RelocateFailure> {
        Err(RelocateFailure {
            inner: SymmError::InvalidArgument {
                message: "不支持的平台".to_string(),
            },
            access_denied: false,
        })
    }

    fn snapshot_dir_acl(&self, _src_dir: &Path) -> Result<Option<PathBuf>, SymmError> {
        Ok(None)
    }

    fn restore_dir_acl(&self, _dst_dir: &Path, _snapshot: &Path) -> Result<(), SymmError> {
        Ok(())
    }
}
