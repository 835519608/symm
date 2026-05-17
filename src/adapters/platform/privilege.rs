//! 当前进程权限检测，以及通过 [`runas`](https://crates.io/crates/runas) 启动提权子进程。

use crate::domain::error::SymmError;
use std::ffi::OsStr;
#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
pub fn is_privileged() -> bool {
    use windows::Win32::UI::Shell::IsUserAnAdmin;
    unsafe { IsUserAnAdmin().as_bool() }
}

#[cfg(unix)]
pub fn is_privileged() -> bool {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|uid| uid.trim() == "0")
        .unwrap_or(false)
}

/// 以管理员/root 启动 `symm <args...>` 并等待（Windows UAC / Unix sudo）。
pub fn spawn_elevated_subcommand<I, S>(args: I) -> Result<(), SymmError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let exe = std::env::current_exe().map_err(|e| SymmError::IoError {
        message: format!("无法定位当前可执行文件：{e}"),
    })?;

    let arg_list: Vec<_> = args
        .into_iter()
        .map(|s| s.as_ref().to_os_string())
        .collect();
    if arg_list.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "提权子进程缺少参数".to_string(),
        });
    }

    let mut cmd = runas::Command::new(&exe);
    for arg in &arg_list {
        cmd.arg(arg);
    }
    #[cfg(windows)]
    cmd.show(false);

    let status = cmd.status().map_err(map_runas_spawn_error)?;

    if status.success() {
        return Ok(());
    }

    let code = status.code().unwrap_or(-1);
    Err(SymmError::PermissionDenied {
        message: format!("提权子进程失败（退出码 {code}）"),
    })
}

/// Windows：提权子进程创建软链（`__elevated-create-link`）。
#[cfg(windows)]
pub fn spawn_elevated_create_link(target: &Path, link: &Path) -> Result<(), SymmError> {
    spawn_elevated_subcommand([
        OsStr::new("__elevated-create-link"),
        target.as_os_str(),
        link.as_os_str(),
    ])
}

fn map_runas_spawn_error(error: std::io::Error) -> SymmError {
    #[cfg(windows)]
    if error.raw_os_error() == Some(1223) {
        return SymmError::PermissionDenied {
            message: "需要管理员权限，但已取消 UAC".to_string(),
        };
    }
    SymmError::IoError {
        message: format!("无法启动提权进程：{error}"),
    }
}
