use crate::domain::error::SymmError;
use std::path::{Path, PathBuf};

pub const DB_FILE_NAME: &str = super::home::DB_FILE_NAME;

pub fn data_home() -> Result<PathBuf, SymmError> {
    super::home::data_home()
}

pub fn db_path() -> Result<PathBuf, SymmError> {
    super::home::db_path()
}

pub fn normalize_target(path: &Path) -> Result<String, SymmError> {
    super::normalize::normalize_target(path)
}

pub fn normalize_target_known_exists(path: &Path) -> Result<String, SymmError> {
    super::normalize::normalize_target_known_exists(path)
}

pub fn normalize_link(path: &Path) -> String {
    super::normalize::normalize_link(path)
}
