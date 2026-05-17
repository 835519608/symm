//! 查锁/杀进程：通过提权子进程执行（`runas`）；调用方已在 `lock::mod` 完成分流。

use super::ProcInfo;
use super::snapshot::{read_snapshot, write_snapshot};
use crate::adapters::platform::elevate;
use crate::adapters::platform::process::{PlatformProcess, platform};
use crate::domain::error::SymmError;
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn list_locking_processes(path: &Path) -> Result<Vec<ProcInfo>, SymmError> {
    let snapshot = temp_snapshot_path("list");
    let log = temp_snapshot_path("elev-log");
    if snapshot.exists() {
        let _ = std::fs::remove_file(&snapshot);
    }
    if log.exists() {
        let _ = std::fs::remove_file(&log);
    }
    if let Err(err) = run_privileged_subcommand([
        OsStr::new("__elevated-list-locks"),
        OsStr::new("--out"),
        snapshot.as_os_str(),
        OsStr::new("--elevated-log"),
        log.as_os_str(),
        path.as_os_str(),
    ]) {
        return Err(enrich_elevated_error(err, &log));
    }
    if !snapshot.is_file() {
        return Err(enrich_elevated_error(
            SymmError::PermissionDenied {
                message: "提权扫锁未生成结果：可能未弹出或未在 UAC 对话框中点击「是」。请检查系统 UAC 是否开启后重试".to_string(),
            },
            &log,
        ));
    }
    let procs = read_snapshot(&snapshot).map_err(|e| {
        enrich_elevated_error(
            SymmError::PermissionDenied {
                message: format!(
                    "提权扫锁结果无效（{}）。若未看到 UAC 对话框，请检查 UAC 设置；若已取消授权请重试",
                    e
                ),
            },
            &log,
        )
    })?;
    let _ = std::fs::remove_file(&snapshot);
    let _ = std::fs::remove_file(&log);
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

fn enrich_elevated_error(err: SymmError, log: &Path) -> SymmError {
    let detail = std::fs::read_to_string(log).unwrap_or_default();
    let detail = detail.trim();
    if detail.is_empty() {
        return err;
    }
    match err {
        SymmError::PermissionDenied { message } => SymmError::PermissionDenied {
            message: format!("{message}（提权子进程：{detail}）"),
        },
        other => other,
    }
}

fn run_privileged_subcommand<I, S>(args: I) -> Result<(), SymmError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    elevate::run_elevated(args)
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
