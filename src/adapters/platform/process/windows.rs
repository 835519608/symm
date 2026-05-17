use super::{LockProbeProgress, PlatformProcess, ProcInfo};
use crate::domain::error::SymmError;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct Platform;

impl PlatformProcess for Platform {
    fn list_locking_processes_with_progress<F>(
        &self,
        path: &Path,
        progress: &mut F,
    ) -> Result<Vec<ProcInfo>, SymmError>
    where
        F: FnMut(LockProbeProgress),
    {
        list_locking_processes_direct(path, progress)
    }

    fn kill_processes(&self, pids: &[u32]) -> Result<(), SymmError> {
        kill_processes_direct(pids)
    }
}

pub(crate) fn list_locking_processes_direct<F>(
    path: &Path,
    progress: &mut F,
) -> Result<Vec<ProcInfo>, SymmError>
where
    F: FnMut(LockProbeProgress),
{
    use filelocksmith::{find_processes_locking_path, pid_to_process_path, set_debug_privilege};

    progress(LockProbeProgress::Querying {
        batch: 1,
        total_batches: 1,
    });
    let _ = set_debug_privilege();
    let path_string = path.to_string_lossy().to_string();
    let pids = find_processes_locking_path(&path_string);
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
