use super::{LockProbeDepth, LockProbeProgress, PlatformProcess, ProcInfo};
use crate::domain::error::SymmError;
use std::collections::HashSet;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// 目录句柄未命中时，抽样扫描子文件以发现占用（如编辑器锁定目录内文件）。
const DIR_LOCK_PROBE_MAX_FILES: usize = 48;
/// 深度抽样总时长上限，避免提权子进程长时间无输出被误认为卡死。
const DIR_LOCK_PROBE_MAX_DURATION: Duration = Duration::from_secs(12);

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct Platform;

impl PlatformProcess for Platform {
    fn list_locking_processes_with_progress<F>(
        &self,
        path: &Path,
        depth: LockProbeDepth,
        progress: &mut F,
    ) -> Result<Vec<ProcInfo>, SymmError>
    where
        F: FnMut(LockProbeProgress),
    {
        list_locking_processes_direct(path, depth, progress)
    }

    fn kill_processes(&self, pids: &[u32]) -> Result<(), SymmError> {
        kill_processes_direct(pids)
    }
}

pub(crate) fn list_locking_processes_direct<F>(
    path: &Path,
    depth: LockProbeDepth,
    progress: &mut F,
) -> Result<Vec<ProcInfo>, SymmError>
where
    F: FnMut(LockProbeProgress),
{
    use filelocksmith::{pid_to_process_path, set_debug_privilege};

    progress(LockProbeProgress::Querying {
        batch: 1,
        total_batches: 1,
    });
    let _ = set_debug_privilege();
    let pids = collect_locking_pids(path, depth, progress);
    let mut out = Vec::with_capacity(pids.len());
    for pid in pids {
        let pid_u32 = match u32::try_from(pid) {
            Ok(pid_u32) => pid_u32,
            Err(_) => continue,
        };
        let display = match pid_to_process_path(pid) {
            Some(proc_path) => format!("PID {pid_u32}  {proc_path}"),
            None => format!("PID {pid}"),
        };
        out.push(ProcInfo {
            pid: pid_u32,
            display,
        });
    }
    Ok(out)
}

pub(crate) fn kill_processes_direct(pids: &[u32]) -> Result<(), SymmError> {
    use filelocksmith::{quit_processes, set_debug_privilege};

    if pids.is_empty() {
        return Ok(());
    }

    let _ = set_debug_privilege();
    let targets_explorer = pids.iter().any(|pid| is_explorer_pid(*pid));
    if targets_explorer {
        dismiss_explorer_windows(CREATE_NO_WINDOW);
    }

    let ids = pids.iter().map(|pid| *pid as usize).collect::<Vec<_>>();
    if !quit_processes(ids) {
        return Err(SymmError::PermissionDenied {
            message: "无法结束占用进程（可能无权限）".to_string(),
        });
    }

    if targets_explorer {
        // explorer 被结束后 winlogon 会立即拉起新实例，需留出句柄释放时间。
        std::thread::sleep(Duration::from_millis(1500));
    }
    Ok(())
}

fn collect_locking_pids<F>(path: &Path, depth: LockProbeDepth, progress: &mut F) -> Vec<usize>
where
    F: FnMut(LockProbeProgress),
{
    use filelocksmith::find_processes_locking_path;

    let path_string = path.to_string_lossy();
    let mut pids: HashSet<usize> = find_processes_locking_path(path_string.as_ref())
        .into_iter()
        .collect();
    if !pids.is_empty() || !path.is_dir() || depth == LockProbeDepth::Shallow {
        return pids.into_iter().collect();
    }

    let started = Instant::now();
    let mut files_checked = 0usize;
    for entry in walkdir::WalkDir::new(path).max_depth(2) {
        if started.elapsed() >= DIR_LOCK_PROBE_MAX_DURATION {
            break;
        }
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        for pid in find_processes_locking_path(entry.path()) {
            pids.insert(pid);
        }
        files_checked += 1;
        progress(LockProbeProgress::Scanning {
            scanned_files: files_checked,
            current: entry.path().to_path_buf(),
        });
        if files_checked >= DIR_LOCK_PROBE_MAX_FILES || pids.len() >= 8 {
            break;
        }
    }
    pids.into_iter().collect()
}

fn is_explorer_pid(pid: u32) -> bool {
    filelocksmith::pid_to_process_path(pid as usize)
        .is_some_and(|path| path.to_ascii_lowercase().ends_with("explorer.exe"))
}

fn dismiss_explorer_windows(create_no_window: u32) {
    let _ = Command::new("powershell")
        .creation_flags(create_no_window)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "$shell = New-Object -ComObject Shell.Application; foreach ($w in @($shell.Windows())) { try { $w.Quit() } catch {} }",
        ])
        .status();
}
