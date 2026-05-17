//! 通过 [`runas`](https://crates.io/crates/runas) 启动提权子进程（Windows UAC / Unix sudo）。

use crate::domain::error::SymmError;
use std::ffi::OsStr;
use std::path::Path;

/// 以管理员/root 启动 `symm <args...>` 并等待；用户取消授权时返回错误。
pub fn run_elevated<I, S>(args: I) -> Result<(), SymmError>
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

    let status = cmd.status().map_err(map_spawn_error)?;

    if status.success() {
        return Ok(());
    }

    let code = status.code().unwrap_or(-1);
    Err(SymmError::PermissionDenied {
        message: format!("提权子进程失败（退出码 {code}）"),
    })
}

pub fn run_elevated_link(target: &Path, link: &Path) -> Result<(), SymmError> {
    run_elevated([
        OsStr::new("__elevated-create-link"),
        target.as_os_str(),
        link.as_os_str(),
    ])
}

fn map_spawn_error(error: std::io::Error) -> SymmError {
    #[cfg(windows)]
    if error.raw_os_error() == Some(1223) {
        return SymmError::PermissionDenied {
            message: "需要管理员权限，但用户已取消 UAC 授权".to_string(),
        };
    }
    SymmError::IoError {
        message: format!("无法启动提权子进程：{error}"),
    }
}
