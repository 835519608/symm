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
            "。文件可能被其它程序占用；请关闭编辑器、资源管理器窗口等后重试。不必对整个终端「以管理员运行」——普通终端执行 add，UAC 点「是」即可扫描占用",
        );
    }
}

#[cfg(not(windows))]
fn append_windows_lock_hint(_error: &std::io::Error, _message: &mut String) {}
