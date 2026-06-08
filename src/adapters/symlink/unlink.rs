use crate::domain::error::SymmError;
use std::fs;
use std::fs::Metadata;
use std::path::Path;

pub fn unlink(link: &Path) -> Result<(), SymmError> {
    match fs::symlink_metadata(link) {
        Ok(meta) => {
            let file_type = meta.file_type();
            let is_junction = is_junction_like(&meta);
            if !file_type.is_symlink() && !is_junction {
                return Ok(());
            }
            let is_dir_link = is_junction || fs::metadata(link).is_ok_and(|m| m.is_dir());
            if is_dir_link {
                fs::remove_dir(link).map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
            } else {
                fs::remove_file(link).map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(SymmError::IoError {
            message: e.to_string(),
        }),
    }
}

#[cfg(windows)]
fn is_junction_like(meta: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    meta.is_dir() && (meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0
}

#[cfg(not(windows))]
fn is_junction_like(_meta: &Metadata) -> bool {
    false
}
