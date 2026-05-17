//! Windows：按需 UAC 提升，启动当前可执行文件的子命令并等待结束。

use crate::domain::error::SymmError;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

const UAC_CANCELLED_HINSTANCE: isize = 1223;

/// 以管理员启动 `symm <args...>` 并等待；用户取消 UAC 时返回错误。
pub fn run_elevated<I, S>(args: I) -> Result<(), SymmError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let exe = std::env::current_exe().map_err(|e| SymmError::IoError {
        message: format!("无法定位当前可执行文件：{e}"),
    })?;

    let arg_tokens: Vec<String> = args
        .into_iter()
        .map(|s| s.as_ref().to_string_lossy().into_owned())
        .collect();
    if arg_tokens.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "提权子进程缺少参数".to_string(),
        });
    }

    let parameters = arg_tokens
        .iter()
        .map(|token| escape_windows_cmd_arg(token))
        .collect::<Vec<_>>()
        .join(" ");

    let work_dir = exe
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string());

    run_elevated_shell_execute(&exe, &parameters, Path::new(&work_dir))
}

pub fn run_elevated_link(target: &Path, link: &Path) -> Result<(), SymmError> {
    run_elevated([
        OsStr::new("__elevated-create-link"),
        target.as_os_str(),
        link.as_os_str(),
    ])
}

#[cfg(windows)]
fn run_elevated_shell_execute(
    exe: &Path,
    parameters: &str,
    work_dir: &Path,
) -> Result<(), SymmError> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let verb = wide_null("runas");
    let file = wide_null(&exe.to_string_lossy());
    let params = wide_null(parameters);
    let directory = wide_null(&work_dir.to_string_lossy());

    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = params.as_ptr();
    info.lpDirectory = directory.as_ptr();
    info.nShow = SW_HIDE;

    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok == 0 {
        return Err(shell_execute_error(unsafe { GetLastError() }));
    }

    if (info.hInstApp as isize) <= 32 {
        return Err(shell_execute_hinstance_error(info.hInstApp as isize));
    }

    if info.hProcess.is_null() {
        return Err(SymmError::PermissionDenied {
            message: "提权子进程未返回进程句柄".to_string(),
        });
    }

    let wait = unsafe { WaitForSingleObject(info.hProcess, u32::MAX) };
    if wait != WAIT_OBJECT_0 {
        unsafe { CloseHandle(info.hProcess) };
        return Err(SymmError::PermissionDenied {
            message: format!("等待提权子进程结束失败（{wait}）"),
        });
    }

    let mut exit_code = 0u32;
    let got_code = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    unsafe { CloseHandle(info.hProcess) };

    if got_code == 0 {
        return Err(SymmError::PermissionDenied {
            message: "无法读取提权子进程退出码".to_string(),
        });
    }

    if exit_code == 0 {
        return Ok(());
    }

    Err(SymmError::PermissionDenied {
        message: format!("提权子进程失败（退出码 {exit_code}）"),
    })
}

#[cfg(windows)]
fn shell_execute_error(code: u32) -> SymmError {
    if code == 1223 {
        return SymmError::PermissionDenied {
            message: "需要管理员权限，但用户已取消 UAC 授权".to_string(),
        };
    }
    SymmError::PermissionDenied {
        message: format!("无法启动提权子进程（Win32 错误 {code}）"),
    }
}

#[cfg(windows)]
fn shell_execute_hinstance_error(code: isize) -> SymmError {
    if code == UAC_CANCELLED_HINSTANCE {
        return SymmError::PermissionDenied {
            message: "需要管理员权限，但用户已取消 UAC 授权".to_string(),
        };
    }
    SymmError::PermissionDenied {
        message: format!("无法启动提权子进程（ShellExecute 代码 {code}）"),
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

/// Windows `CreateProcess` 命令行参数转义。
fn escape_windows_cmd_arg(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }
    if !value
        .chars()
        .any(|ch| ch.is_ascii_whitespace() || ch == '"' || ch == '\\')
    {
        return value.to_string();
    }

    let mut escaped = String::from('"');
    let mut backslashes = 0usize;
    for ch in value.chars() {
        if ch == '\\' {
            backslashes += 1;
            continue;
        }
        if ch == '"' {
            escaped.push_str(&"\\".repeat(backslashes * 2 + 1));
            escaped.push('"');
        } else {
            escaped.push_str(&"\\".repeat(backslashes));
            escaped.push(ch);
        }
        backslashes = 0;
    }
    escaped.push_str(&"\\".repeat(backslashes));
    escaped.push('"');
    escaped
}
