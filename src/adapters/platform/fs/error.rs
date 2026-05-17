use crate::domain::error::SymmError;

/// `relocate_path` 失败时的结构化信息（避免用字符串匹配 `os error 5`）。
#[derive(Debug)]
pub struct RelocateFailure {
    pub inner: SymmError,
    pub access_denied: bool,
}

impl RelocateFailure {
    pub fn from_io(err: std::io::Error) -> Self {
        Self {
            access_denied: err.raw_os_error() == Some(5),
            inner: map_io_error(err),
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
            "。系统拒绝访问（os error 5），可能仍有占用未被识别；可在解除占用时授权提升，或检查目标路径权限",
        );
    }
    SymmError::IoError { message }
}
