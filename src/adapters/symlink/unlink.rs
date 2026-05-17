use crate::domain::error::SymmError;
use std::fs;
use std::path::Path;

pub fn unlink(link: &Path) -> Result<(), SymmError> {
    match fs::symlink_metadata(link) {
        Ok(meta) => {
            let file_type = meta.file_type();
            if file_type.is_dir() {
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
