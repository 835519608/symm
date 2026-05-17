use super::restart_manager;
use super::{LockProbeProgress, PlatformProcess, ProcInfo};
use crate::domain::error::SymmError;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    QueryFullProcessImageNameW, TerminateProcess,
};

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
    restart_manager::list_locking_processes_for_path(path, progress)
}

pub(crate) fn kill_processes_direct(pids: &[u32]) -> Result<(), SymmError> {
    if pids.is_empty() {
        return Ok(());
    }

    let targets_explorer = pids.iter().any(|pid| is_explorer_pid(*pid));
    if targets_explorer {
        dismiss_explorer_windows(CREATE_NO_WINDOW);
    }

    for pid in pids {
        terminate_process(*pid)?;
    }

    if targets_explorer {
        std::thread::sleep(Duration::from_millis(1500));
    }
    Ok(())
}

pub(crate) fn process_image_path(pid: u32) -> Option<PathBuf> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let result = query_image_path(handle);
        let _ = CloseHandle(handle);
        result
    }
}

fn terminate_process(pid: u32) -> Result<(), SymmError> {
    if pid == 0 {
        return Ok(());
    }
    unsafe {
        let handle =
            OpenProcess(PROCESS_TERMINATE, false, pid).map_err(|e| SymmError::IoError {
                message: format!("无法打开进程 PID={pid}：{e}"),
            })?;
        let result = TerminateProcess(handle, 1);
        let _ = CloseHandle(handle);
        if result.is_err() {
            return Err(SymmError::PermissionDenied {
                message: format!("无法结束进程 PID={pid}（可能无权限）"),
            });
        }
    }
    Ok(())
}

unsafe fn query_image_path(process: HANDLE) -> Option<PathBuf> {
    let mut buffer = [0u16; 32_768];
    let mut size = buffer.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
        .ok()?;
    }
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(size as usize);
    Some(PathBuf::from(String::from_utf16_lossy(&buffer[..end])))
}

fn is_explorer_pid(pid: u32) -> bool {
    process_image_path(pid).is_some_and(|path| {
        path.to_string_lossy()
            .to_ascii_lowercase()
            .ends_with("explorer.exe")
    })
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
