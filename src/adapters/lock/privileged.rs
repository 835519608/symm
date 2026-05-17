//! 查锁/杀进程：通过提权子进程执行（Windows UAC / Unix sudo）；调用方已在 `lock::mod` 完成分流。

use super::ProcInfo;
use super::snapshot::{read_snapshot, write_snapshot};
use crate::adapters::platform::process::{PlatformProcess, platform};
use crate::domain::error::SymmError;
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use crate::adapters::platform::elevate;

pub fn list_locking_processes(path: &Path) -> Result<Vec<ProcInfo>, SymmError> {
    let snapshot = temp_snapshot_path("list");
    if snapshot.exists() {
        let _ = std::fs::remove_file(&snapshot);
    }
    run_privileged_subcommand([
        OsStr::new("__elevated-list-locks"),
        OsStr::new("--out"),
        snapshot.as_os_str(),
        path.as_os_str(),
    ])?;
    if !snapshot.is_file() {
        return Err(SymmError::PermissionDenied {
            message: "提权扫锁未生成结果：可能未弹出或未在 UAC 对话框中点击「是」。请检查系统 UAC 是否开启后重试".to_string(),
        });
    }
    let procs = read_snapshot(&snapshot).map_err(|e| SymmError::PermissionDenied {
        message: format!(
            "提权扫锁结果无效（{}）。若未看到 UAC 对话框，请检查 UAC 设置；若已取消授权请重试",
            e
        ),
    })?;
    let _ = std::fs::remove_file(&snapshot);
    Ok(procs)
}

pub fn kill_processes(pids: &[u32]) -> Result<(), SymmError> {
    let joined = pids
        .iter()
        .map(|pid| pid.to_string())
        .collect::<Vec<_>>()
        .join(",");
    run_privileged_subcommand([OsStr::new("__elevated-kill"), OsStr::new(&joined)])
}

pub fn elevated_list_locks_entry(path: &Path, output: &Path) -> Result<(), SymmError> {
    let procs = platform().list_locking_processes_with_progress(path, &mut |_| {})?;
    write_snapshot(output, &procs)
}

fn run_privileged_subcommand<I, S>(args: I) -> Result<(), SymmError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    #[cfg(windows)]
    {
        elevate::run_elevated(args)
    }

    #[cfg(unix)]
    {
        let exe = env::current_exe().map_err(|e| SymmError::IoError {
            message: format!("无法定位当前可执行文件：{e}"),
        })?;
        let status = Command::new("sudo")
            .arg(&exe)
            .args(args.into_iter().map(|s| s.as_ref().to_os_string()))
            .status()
            .map_err(|e| SymmError::IoError {
                message: format!("无法通过 sudo 启动提权子进程：{e}"),
            })?;
        if status.success() {
            return Ok(());
        }
        Err(SymmError::PermissionDenied {
            message: format!(
                "需要管理员/root 权限（sudo 退出码 {}）",
                status.code().unwrap_or(-1)
            ),
        })
    }
}

fn temp_snapshot_path(kind: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let pid = std::process::id();
    let tick = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    path.push(format!("symm-{kind}-{pid}-{tick}.locks"));
    path
}
