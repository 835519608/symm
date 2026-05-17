//! Windows Restart Manager：按迁移资源清单查询占用进程。
//!
//! RM 只应注册**文件**路径；目录若不带尾部 `\` 会导致 `RmGetList` 返回 `ERROR_ACCESS_DENIED`。
//! 批次内若有受防护/过滤驱动拦截的路径，会对整批 `RmGetList` 失败，故对 `ACCESS_DENIED` 做二分拆分。

use super::ProcInfo;
use crate::domain::error::SymmError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use windows::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_MORE_DATA, ERROR_SUCCESS, WIN32_ERROR,
};
use windows::Win32::System::RestartManager::{
    CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources,
    RmStartSession,
};
use windows::core::PCWSTR;

/// 单次 RM 会话注册的资源上限（路径过多时分批）。
const RM_REGISTER_CHUNK: usize = 512;
const RM_GETLIST_MAX_RETRIES: u32 = 6;

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
    if resources.is_empty() {
        return Ok(vec![]);
    }
    let total_batches = resources.len().div_ceil(RM_REGISTER_CHUNK);
    list_processes_for_resources(&resources, total_batches, &mut progress)
}

fn collect_resource_paths(
    root: &Path,
    progress: &mut impl FnMut(super::LockProbeProgress),
) -> Result<Vec<PathBuf>, SymmError> {
    let root = dunce::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    if root.is_file() {
        return Ok(vec![root]);
    }

    let mut paths = Vec::new();
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

fn list_processes_for_resources(
    paths: &[PathBuf],
    total_batches: usize,
    progress: &mut impl FnMut(super::LockProbeProgress),
) -> Result<Vec<ProcInfo>, SymmError> {
    let self_pid = std::process::id();
    let mut by_pid: HashMap<u32, ProcInfo> = HashMap::new();

    for (batch_idx, chunk) in paths.chunks(RM_REGISTER_CHUNK).enumerate() {
        progress(super::LockProbeProgress::Querying {
            batch: batch_idx + 1,
            total_batches,
        });
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
    if paths.is_empty() {
        return Ok(vec![]);
    }
    match query_rm_chunk_once(paths) {
        Ok(list) => Ok(list),
        Err(code) if code == ERROR_ACCESS_DENIED && paths.len() > 1 => {
            let mid = paths.len() / 2;
            let (left, right) = paths.split_at(mid);
            let mut merged = query_rm_chunk(left)?;
            merged.extend(query_rm_chunk(right)?);
            Ok(merged)
        }
        Err(ERROR_ACCESS_DENIED) => Ok(vec![]),
        Err(code) => Err(rm_error(code, "RmGetList")),
    }
}

fn query_rm_chunk_once(paths: &[PathBuf]) -> Result<Vec<RM_PROCESS_INFO>, WIN32_ERROR> {
    let wide_paths = paths
        .iter()
        .map(|p| path_to_wide_null(p))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ERROR_ACCESS_DENIED)?;

    let pcwstrs: Vec<PCWSTR> = wide_paths.iter().map(|w| PCWSTR(w.as_ptr())).collect();

    unsafe {
        let session = start_session()?;
        let register = RmRegisterResources(session.0, Some(pcwstrs.as_slice()), None, None);
        if register != ERROR_SUCCESS {
            return Err(register);
        }
        get_process_list(session.0)
    }
}

unsafe fn start_session() -> Result<RmSession, WIN32_ERROR> {
    let mut handle = 0u32;
    let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];
    let result =
        unsafe { RmStartSession(&mut handle, Some(0), windows::core::PWSTR(key.as_mut_ptr())) };
    if result != ERROR_SUCCESS {
        return Err(result);
    }
    Ok(RmSession(handle))
}

unsafe fn get_process_list(session: u32) -> Result<Vec<RM_PROCESS_INFO>, WIN32_ERROR> {
    let mut count = 0u32;
    let mut buffer: Vec<RM_PROCESS_INFO> = Vec::new();
    let mut retry = 0u32;

    loop {
        let mut needed = 0u32;
        let mut count_inout = count;
        let mut reboot = 0u32;
        let ptr = if buffer.is_empty() {
            None
        } else {
            Some(buffer.as_mut_ptr())
        };
        let result = unsafe { RmGetList(session, &mut needed, &mut count_inout, ptr, &mut reboot) };

        if result == ERROR_SUCCESS {
            buffer.truncate(count_inout as usize);
            return Ok(buffer);
        }

        if result == ERROR_MORE_DATA && retry < RM_GETLIST_MAX_RETRIES {
            count = needed;
            buffer.resize(needed as usize, RM_PROCESS_INFO::default());
            retry += 1;
            continue;
        }

        return Err(result);
    }
}

fn format_rm_process(info: &RM_PROCESS_INFO, pid: u32) -> String {
    let app = wide_null_to_string(&info.strAppName);
    if let Some(image) = super::windows::process_image_path(pid) {
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
    use std::os::windows::ffi::OsStrExt;
    if path.is_dir() {
        return Err(SymmError::IoError {
            message: "Restart Manager 资源须为文件路径，不能注册目录".to_string(),
        });
    }
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

fn rm_error(code: WIN32_ERROR, api: &str) -> SymmError {
    SymmError::IoError {
        message: format!("Restart Manager {api} 失败（Win32 错误 {}）", code.0),
    }
}
