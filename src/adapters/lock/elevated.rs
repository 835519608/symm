//! 查锁/杀进程：通过提权子进程执行（`runas`）；调用方已在 `lock::mod` 完成分流。

#[cfg(windows)]
use super::ProcInfo;
use super::elevated_progress::ProgressAppender;
#[cfg(windows)]
use super::elevated_progress::spawn_progress_relay;
#[cfg(windows)]
use super::snapshot::read_snapshot;
use super::snapshot::write_snapshot;
use crate::adapters::platform::privilege;
use crate::adapters::platform::process::{LockProbeProgress, PlatformProcess, platform};
use crate::domain::error::SymmError;
#[cfg(windows)]
use std::env;
use std::ffi::OsStr;
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::sync::Arc;
#[cfg(windows)]
use std::sync::atomic::AtomicU64;
#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::mpsc;
#[cfg(windows)]
use std::thread;
#[cfg(windows)]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(windows)]
pub fn list_locking_processes(
    path: &Path,
    mut progress: impl FnMut(LockProbeProgress),
) -> Result<Vec<ProcInfo>, SymmError> {
    let session = ElevatedLockProbeSession::new();
    let snapshot = session.snapshot().to_path_buf();
    let log = session.log().to_path_buf();
    let progress_file = session.progress().to_path_buf();

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
    let mut progress_appender = progress_path.map(ProgressAppender::new).transpose()?;
    let mut report = |event: LockProbeProgress| {
        if let Some(appender) = progress_appender.as_mut() {
            let _ = appender.append(&event);
        }
    };
    let result = platform().list_locking_processes_with_progress(path, &mut report);
    if let Some(appender) = progress_appender {
        appender.finish()?;
    }
    let procs = result?;
    write_snapshot(output, &procs)
}

#[cfg(windows)]
fn enrich_elevated_error(err: SymmError, log: &Path) -> SymmError {
    let detail = std::fs::read_to_string(log).unwrap_or_default();
    let detail = detail.trim();
    if detail.is_empty() {
        return err;
    }
    if elevated_log_indicates_io_failure(detail) {
        return SymmError::IoError {
            message: format!("占用扫描失败：{detail}"),
        };
    }
    match err {
        SymmError::PermissionDenied { message } => SymmError::PermissionDenied {
            message: format!("{message}（子进程日志：{detail}）"),
        },
        other => other,
    }
}

#[cfg(windows)]
fn elevated_log_indicates_io_failure(detail: &str) -> bool {
    detail.contains("收集占用检测路径失败")
        || detail.contains("IoError")
        || detail.contains("IO 错误")
        || detail.contains("IO error")
}

fn run_privileged_subcommand<I, S>(args: I) -> Result<(), SymmError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    privilege::spawn_elevated_subcommand(args)
}

#[cfg(windows)]
struct ElevatedLockProbeSession {
    snapshot: PathBuf,
    log: PathBuf,
    progress: PathBuf,
}

#[cfg(windows)]
impl ElevatedLockProbeSession {
    fn new() -> Self {
        let stem = unique_temp_stem();
        Self {
            snapshot: temp_session_path(&stem, "snapshot"),
            log: temp_session_path(&stem, "log"),
            progress: temp_session_path(&stem, "progress"),
        }
    }

    fn snapshot(&self) -> &Path {
        &self.snapshot
    }

    fn log(&self) -> &Path {
        &self.log
    }

    fn progress(&self) -> &Path {
        &self.progress
    }
}

#[cfg(windows)]
impl Drop for ElevatedLockProbeSession {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.snapshot);
        let _ = std::fs::remove_file(&self.log);
        let _ = std::fs::remove_file(&self.progress);
    }
}

#[cfg(windows)]
fn unique_temp_stem() -> String {
    let pid = std::process::id();
    let tick = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("symm-lock-probe-{pid}-{tick}-{sequence}")
}

#[cfg(windows)]
fn temp_session_path(stem: &str, kind: &str) -> PathBuf {
    env::temp_dir().join(format!("{stem}-{kind}.tmp"))
}
