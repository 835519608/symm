use crate::domain::error::SymmError;

pub fn format_io_error(error: &std::io::Error) -> String {
    let mut message = error.to_string();
    append_windows_lock_hint(error, &mut message);
    message
}

pub fn ioe(error: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: format_io_error(&error),
    }
}

pub fn io_ctx(context: &str, error: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: format!("{context}：{}", format_io_error(&error)),
    }
}

#[cfg(windows)]
fn append_windows_lock_hint(error: &std::io::Error, message: &mut String) {
    if error.raw_os_error() == Some(33) {
        message.push_str(
            "。该文件可能被其它程序独占锁定（常见于 Cursor 仍打开该目录时）；请完全退出 Cursor 后重试。无需对整个 symm「以管理员身份运行」——在普通终端执行 add，对 UAC 选「是」即可用于占用扫描",
        );
    }
}

#[cfg(not(windows))]
fn append_windows_lock_hint(_error: &std::io::Error, _message: &mut String) {}
