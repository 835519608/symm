use thiserror::Error;

#[derive(Debug, Error)]
pub enum SymmError {
    #[error("参数错误：{message}")]
    InvalidArgument { message: String },
    #[error("权限不足：{message}")]
    PermissionDenied { message: String },
    #[error("目标不存在：{path}")]
    TargetNotFound { path: String },
    #[error("名称冲突：{name}")]
    NameConflict { name: String },
    #[error("未找到记录：{selector}")]
    NotFound { selector: String },
    #[error("数据库错误：{message}")]
    DbError { message: String },
    #[error("IO 错误：{message}")]
    IoError { message: String },
}

impl SymmError {
    pub fn code(&self) -> &'static str {
        match self {
            SymmError::InvalidArgument { .. } => "invalid_argument",
            SymmError::PermissionDenied { .. } => "permission_denied",
            SymmError::TargetNotFound { .. } => "target_not_found",
            SymmError::NameConflict { .. } => "name_conflict",
            SymmError::NotFound { .. } => "not_found",
            SymmError::DbError { .. } => "db_error",
            SymmError::IoError { .. } => "io_error",
        }
    }
}
