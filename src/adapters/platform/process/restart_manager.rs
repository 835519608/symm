//! Windows Restart Manager：按迁移资源清单查询占用进程。

use super::ProcInfo;
use crate::domain::error::SymmError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use windows::Win32::Foundation::{ERROR_MORE_DATA, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::RestartManager::{
    CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources,
    RmStartSession,
};
use windows::core::PCWSTR;

/// 单次 RM 会话注册的资源上限（路径过多时分批）。
const RM_REGISTER_CHUNK: usize = 512;

struct RmSession(u32);

impl Drop for RmSession {
    fn drop(&mut self) {
        unsafe {
            let _ = RmEndSession(self.0);
        }
    }
}

pub fn list_locking_processes_for_path(
    root: &Path,
    mut progress: impl FnMut(super::LockProbeProgress),
) -> Result<Vec<ProcInfo>, SymmError> {
    let resources = collect_resource_paths(root, &mut progress)?;
    progress(super::LockProbeProgress::Querying {
        batch: 1,
        total_batches: 1,
    });
    list_processes_for_resources(&resources)
}

fn collect_resource_paths(
    root: &Path,
    progress: &mut impl FnMut(super::LockProbeProgress),
) -> Result<Vec<PathBuf>, SymmError> {
    let root = dunce::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    if root.is_file() {
        return Ok(vec![root]);
    }

    let mut paths = vec![root.clone()];
    let mut files_seen = 0usize;
    for entry in WalkDir::new(&root).follow_links(false) {
        let entry = entry.map_err(|e| SymmError::IoError {
            message: format!("收集占用检测路径失败：{e}"),
        })?;
        let path = entry.path();
        if path == root {
            continue;
        }
        if entry.file_type().is_symlink() {
            continue;
        }
        if entry.file_type().is_file() {
            paths.push(path.to_path_buf());
            files_seen += 1;
            progress(super::LockProbeProgress::Scanning {
                scanned_files: files_seen,
                current: path.to_path_buf(),
            });
        }
    }
    Ok(paths)
}

fn list_processes_for_resources(paths: &[PathBuf]) -> Result<Vec<ProcInfo>, SymmError> {
    if paths.is_empty() {
        return Ok(vec![]);
    }

    let self_pid = std::process::id();
    let mut by_pid: HashMap<u32, ProcInfo> = HashMap::new();

    for chunk in paths.chunks(RM_REGISTER_CHUNK) {
        for info in query_rm_chunk(chunk)? {
            let pid = info.Process.dwProcessId;
            if pid == 0 || pid == self_pid {
                continue;
            }
            by_pid.entry(pid).or_insert_with(|| ProcInfo {
                pid,
                display: format_rm_process(&info, pid),
            });
        }
    }

    let mut out: Vec<_> = by_pid.into_values().collect();
    out.sort_by_key(|p| p.pid);
    Ok(out)
}

fn query_rm_chunk(paths: &[PathBuf]) -> Result<Vec<RM_PROCESS_INFO>, SymmError> {
    let wide_paths = paths
        .iter()
        .map(|p| path_to_wide_null(p))
        .collect::<Result<Vec<_>, _>>()?;
    let pcwstrs: Vec<PCWSTR> = wide_paths.iter().map(|w| PCWSTR(w.as_ptr())).collect();

    unsafe {
        let session = start_session()?;
        let register = RmRegisterResources(session.0, Some(pcwstrs.as_slice()), None, None);
        win32_ok(register, "RmRegisterResources")?;
        let list = get_process_list(session.0)?;
        Ok(list)
    }
}

unsafe fn start_session() -> Result<RmSession, SymmError> {
    let mut handle = 0u32;
    let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
    let result = RmStartSession(&mut handle, 0, windows::core::PWSTR(key.as_mut_ptr()));
    win32_ok(result, "RmStartSession")?;
    Ok(RmSession(handle))
}

unsafe fn get_process_list(session: u32) -> Result<Vec<RM_PROCESS_INFO>, SymmError> {
    let mut needed = 0u32;
    let mut count = 0u32;
    let mut reboot = 0u32;
    let first = RmGetList(session, &mut needed, &mut count, None, &mut reboot);
    if first != ERROR_SUCCESS && first != ERROR_MORE_DATA {
        return Err(rm_error(first, "RmGetList（查询大小）"));
    }
    if needed == 0 {
        return Ok(vec![]);
    }

    let mut buffer = vec![RM_PROCESS_INFO::default(); needed as usize];
    count = needed;
    let second = RmGetList(
        session,
        &mut needed,
        &mut count,
        Some(buffer.as_mut_ptr()),
        &mut reboot,
    );
    if second != ERROR_SUCCESS && second != ERROR_MORE_DATA {
        return Err(rm_error(second, "RmGetList"));
    }
    buffer.truncate(count as usize);
    Ok(buffer)
}

fn format_rm_process(info: &RM_PROCESS_INFO, pid: u32) -> String {
    let app = wide_null_to_string(&info.strAppName);
    if let Some(image) = super::process_image_path(pid) {
        if app.is_empty() {
            format!("PID {pid}  {}", image.display())
        } else {
            format!("PID {pid}  {} ({app})", image.display())
        }
    } else if app.is_empty() {
        format!("PID {pid}")
    } else {
        format!("PID {pid}  {app}")
    }
}

fn path_to_wide_null(path: &Path) -> Result<Vec<u16>, SymmError> {
    use std::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    if wide.len() <= 1 {
        return Err(SymmError::IoError {
            message: "占用检测路径为空".to_string(),
        });
    }
    Ok(wide)
}

fn wide_null_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn win32_ok(code: WIN32_ERROR, api: &str) -> Result<(), SymmError> {
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(rm_error(code, api))
    }
}

fn rm_error(code: WIN32_ERROR, api: &str) -> SymmError {
    SymmError::IoError {
        message: format!("Restart Manager {api} 失败（Win32 错误 {code}）"),
    }
}
