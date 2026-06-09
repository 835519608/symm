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
    #[error(
        "文件系统已变更但链接记录写入失败：operation={operation}, link={link_path}, target={target_path}；{message}"
    )]
    FilesystemAppliedButDbFailed {
        operation: String,
        link_path: String,
        target_path: String,
        message: String,
    },
    #[error(
        "文件系统已变更但链接记录删除失败：operation={operation}, link={link_path}, target={target_path}；{message}"
    )]
    FilesystemAppliedButRecordDeleteFailed {
        operation: String,
        link_path: String,
        target_path: String,
        message: String,
    },
    #[error(
        "文件系统已变更但链接记录已保留：operation={operation}, link={link_path}, target={target_path}；{message}"
    )]
    FilesystemAppliedButRecordKept {
        operation: String,
        link_path: String,
        target_path: String,
        message: String,
    },
    #[error("批量操作包含文件系统已变更但记录未完成的失败：{message}")]
    BatchFilesystemAppliedButRecordIncomplete { message: String },
    #[error(
        "实体已迁移但创建 link 失败：link={link_path}, target={target_path}；链接记录尚未写入；{message}"
    )]
    EntityMigratedButLinkCreateFailed {
        link_path: String,
        target_path: String,
        message: String,
    },
    #[error("批量操作失败：{message}")]
    BatchFailure { message: String },
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
            SymmError::FilesystemAppliedButDbFailed { .. } => "filesystem_applied_but_db_failed",
            SymmError::FilesystemAppliedButRecordDeleteFailed { .. } => {
                "filesystem_applied_but_record_delete_failed"
            }
            SymmError::FilesystemAppliedButRecordKept { .. } => {
                "filesystem_applied_but_record_kept"
            }
            SymmError::BatchFilesystemAppliedButRecordIncomplete { .. } => {
                "filesystem_applied_but_record_incomplete"
            }
            SymmError::EntityMigratedButLinkCreateFailed { .. } => {
                "entity_migrated_but_link_create_failed"
            }
            SymmError::BatchFailure { .. } => "batch_failure",
        }
    }
}
