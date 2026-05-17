//! 查锁/杀进程：通过提权子进程执行（`runas`）；调用方已在 `lock::mod` 完成分流。

use super::ProcInfo;
use super::elevated_progress::{append_progress, spawn_progress_relay};
use super::snapshot::{read_snapshot, write_snapshot};
use crate::adapters::platform::privilege;
use crate::adapters::platform::process::{LockProbeProgress, PlatformProcess, platform};
use crate::domain::error::SymmError;
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn list_locking_processes(
    path: &Path,
    mut progress: impl FnMut(LockProbeProgress),
) -> Result<Vec<ProcInfo>, SymmError> {
    let snapshot = temp_snapshot_path("list");
    let log = temp_snapshot_path("elev-log");
    let progress_file = temp_snapshot_path("elev-progress");
    if snapshot.exists() {
        let _ = std::fs::remove_file(&snapshot);
    }
    if log.exists() {
        let _ = std::fs::remove_file(&log);
    }
    if progress_file.exists() {
        let _ = std::fs::remove_file(&progress_file);
    }

    let (tx, rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let relay = spawn_progress_relay(progress_file.clone(), tx, stop.clone());

    let child = thread::spawn({
        let snapshot = snapshot.clone();
        let log = log.clone();
        let progress_file = progress_file.clone();
        let path = path.to_path_buf();
        move || {
            run_privileged_subcommand([
                OsStr::new("__elevated-list-locks"),
                OsStr::new("--out"),
                snapshot.as_os_str(),
                OsStr::new("--elevated-log"),
                log.as_os_str(),
                OsStr::new("--elevated-progress"),
                progress_file.as_os_str(),
                path.as_os_str(),
            ])
        }
    });

    loop {
        while let Ok(event) = rx.try_recv() {
            progress(event);
        }
        if child.is_finished() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    stop.store(true, Ordering::Relaxed);
    let _ = relay.join();
    while let Ok(event) = rx.try_recv() {
        progress(event);
    }

    let run_result = child.join().map_err(|_| SymmError::IoError {
        message: "占用扫描子进程异常退出".to_string(),
    })?;

    if let Err(err) = run_result {
        return Err(enrich_elevated_error(err, &log));
    }
    if !snapshot.is_file() {
        return Err(enrich_elevated_error(
            SymmError::PermissionDenied {
                message:
                    "占用扫描无结果：可能未弹出 UAC，或未点「是」。请确认系统 UAC 已开启后重试"
                        .to_string(),
            },
            &log,
        ));
    }
    let procs = read_snapshot(&snapshot).map_err(|e| {
        enrich_elevated_error(
            SymmError::PermissionDenied {
                message: format!(
                    "占用扫描结果无效（{}）。未看到 UAC 请检查设置；若已取消授权请重试",
                    e
                ),
            },
            &log,
        )
    })?;
    let _ = std::fs::remove_file(&snapshot);
    let _ = std::fs::remove_file(&log);
    let _ = std::fs::remove_file(&progress_file);
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

pub fn elevated_list_locks_entry(
    path: &Path,
    output: &Path,
    progress_path: Option<&Path>,
) -> Result<(), SymmError> {
    let mut report = |event: LockProbeProgress| {
        if let Some(progress_path) = progress_path {
            let _ = append_progress(progress_path, &event);
        }
    };
    let procs = platform().list_locking_processes_with_progress(path, &mut report)?;
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
            message: format!("{message}（子进程日志：{detail}）"),
        },
        other => other,
    }
}

fn run_privileged_subcommand<I, S>(args: I) -> Result<(), SymmError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    privilege::spawn_elevated_subcommand(args)
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
