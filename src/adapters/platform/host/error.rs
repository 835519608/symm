use crate::domain::error::SymmError;

/// `relocate_path` 失败时的结构化信息（避免用字符串匹配 `os error 5`）。
#[derive(Debug)]
pub struct RelocateFailure {
    pub inner: SymmError,
    pub access_denied: bool,
    /// Windows：`rename` 软链遇 ACCESS_DENIED 时由 migrate 层经 `symlink::write_symlink` 兜底。
    pub symlink_needs_recreate: bool,
    /// 当前平台没有原子 no-clobber rename 能力；迁移层应回退到复制路径。
    pub no_replace_unsupported: bool,
}

impl RelocateFailure {
    pub fn from_io(err: std::io::Error) -> Self {
        Self {
            access_denied: err.raw_os_error() == Some(5),
            inner: map_io_error(err),
            symlink_needs_recreate: false,
            no_replace_unsupported: false,
        }
    }

    pub fn symlink_rename_denied(err: std::io::Error) -> Self {
        Self {
            access_denied: true,
            inner: map_io_error(err),
            symlink_needs_recreate: true,
            no_replace_unsupported: false,
        }
    }

    pub fn no_replace_unsupported() -> Self {
        Self {
            access_denied: false,
            inner: SymmError::IoError {
                message: "当前平台不支持原子不覆盖移动".to_string(),
            },
            symlink_needs_recreate: false,
            no_replace_unsupported: true,
        }
    }
}

pub fn map_link_io_error(e: std::io::Error) -> SymmError {
    map_io_error(e)
}

fn map_io_error(e: std::io::Error) -> SymmError {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        SymmError::PermissionDenied {
            message: e.to_string(),
        }
    } else {
        SymmError::IoError {
            message: e.to_string(),
        }
    }
}

pub fn error_detail(err: SymmError) -> String {
    match err {
        SymmError::IoError { message } => message,
        other => other.to_string(),
    }
}

pub fn format_relocate_failure(role: &str, failure: RelocateFailure) -> SymmError {
    let detail = error_detail(failure.inner);
    let mut message = format!("无法移动 {role}：{detail}");
    if failure.access_denied {
        message.push_str(
            "。拒绝访问（错误 5）：可能仍有程序占用；结束占用时授权 UAC，或检查目标路径权限",
        );
    }
    SymmError::IoError { message }
}
