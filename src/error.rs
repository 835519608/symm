use thiserror::Error;

#[derive(Debug, Error)]
pub enum SymmError {
    #[error("invalid argument: {message}")]
    InvalidArgument { message: String },
    #[error("permission denied: {message}")]
    PermissionDenied { message: String },
    #[error("target not found: {path}")]
    TargetNotFound { path: String },
    #[error("name conflict: {name}")]
    NameConflict { name: String },
    #[error("path conflict: {path}")]
    PathConflict { path: String },
    #[error("not found: {selector}")]
    NotFound { selector: String },
    #[error("database error: {message}")]
    DbError { message: String },
    #[error("io error: {message}")]
    IoError { message: String },
}

impl SymmError {
    pub fn code(&self) -> &'static str {
        match self {
            SymmError::InvalidArgument { .. } => "invalid_argument",
            SymmError::PermissionDenied { .. } => "permission_denied",
            SymmError::TargetNotFound { .. } => "target_not_found",
            SymmError::NameConflict { .. } => "name_conflict",
            SymmError::PathConflict { .. } => "path_conflict",
            SymmError::NotFound { .. } => "not_found",
            SymmError::DbError { .. } => "db_error",
            SymmError::IoError { .. } => "io_error",
        }
    }
}
