use crate::domain::error::SymmError;
use crate::infra::errors::io_map::ioe;
use std::fs;
use std::path::Path;

pub fn remove_path_any(path: &Path) -> Result<(), SymmError> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_dir() {
                fs::remove_dir_all(path).map_err(ioe)?;
            } else {
                fs::remove_file(path).map_err(ioe)?;
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ioe(e)),
    }
}
