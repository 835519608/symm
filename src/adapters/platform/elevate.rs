//! Windows：按需 UAC 提升，启动当前可执行文件的子命令并等待结束。

use crate::domain::error::SymmError;
use std::ffi::OsStr;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

    let ps_args = arg_tokens
        .iter()
        .map(|token| ps_single_quote(token))
        .collect::<Vec<_>>()
        .join(", ");

    let script = format!(
        "$p = Start-Process -FilePath {} -ArgumentList @({}) -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode",
        ps_single_quote(&exe.to_string_lossy()),
        ps_args
    );

    let status = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .status()
        .map_err(|e| SymmError::IoError {
            message: format!("无法启动提权子进程：{e}"),
        })?;

    if status.success() {
        return Ok(());
    }

    let code = status.code().unwrap_or(-1);
    if code == 1223 {
        return Err(SymmError::PermissionDenied {
            message: "需要管理员权限，但用户已取消 UAC 授权".to_string(),
        });
    }

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

fn ps_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
