use crate::error::SymmError;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static MOCK_LOCK_RELEASED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum LockProbeProgress {
    Scanning {
        scanned_files: usize,
        current: PathBuf,
    },
    Querying {
        batch: usize,
        total_batches: usize,
    },
}

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub display: String,
}

impl Display for ProcInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

pub fn list_locking_processes_with_progress<F>(
    path: &Path,
    mut progress: F,
) -> Result<Vec<ProcInfo>, SymmError>
where
    F: FnMut(LockProbeProgress),
{
    if let Some(mocked) = mock_locking_processes(path) {
        return Ok(mocked);
    }
    #[cfg(windows)]
    {
        windows_list_locking_processes(path, &mut progress)
    }
    #[cfg(not(windows))]
    {
        let _ = &mut progress;
        unix_list_locking_processes(path)
    }
}

#[cfg(windows)]
pub fn kill_processes(pids: &[u32]) -> Result<(), SymmError> {
    if should_mock_kill_processes() {
        if mock_locks_clear_on_kill() {
            MOCK_LOCK_RELEASED.store(true, Ordering::SeqCst);
        }
        return Ok(());
    }
    for pid in pids {
        let status = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .map_err(|e| SymmError::IoError {
                message: format!("执行 taskkill 失败：{e}"),
            })?;
        if !status.success() {
            return Err(SymmError::PermissionDenied {
                message: format!("无法结束进程 PID={pid}（可能无权限）"),
            });
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn kill_processes(pids: &[u32]) -> Result<(), SymmError> {
    if should_mock_kill_processes() {
        if mock_locks_clear_on_kill() {
            MOCK_LOCK_RELEASED.store(true, Ordering::SeqCst);
        }
        return Ok(());
    }
    for pid in pids {
        let status = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .map_err(|e| SymmError::IoError {
                message: format!("执行 kill 失败：{e}"),
            })?;
        if !status.success() {
            return Err(SymmError::PermissionDenied {
                message: format!("无法结束进程 PID={pid}（可能无权限）"),
            });
        }
    }
    Ok(())
}

fn mock_locking_processes(path: &Path) -> Option<Vec<ProcInfo>> {
    let raw_paths = std::env::var("SYMM_TEST_LOCK_PATHS").ok()?;
    if mock_locks_clear_on_kill() && MOCK_LOCK_RELEASED.load(Ordering::SeqCst) {
        return Some(vec![]);
    }
    let mocked_paths = raw_paths
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    let current = path.to_string_lossy().to_string();
    if !mocked_paths
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&current))
    {
        return Some(vec![]);
    }

    let display = std::env::var("SYMM_TEST_LOCK_DISPLAY")
        .unwrap_or_else(|_| "PID 4242  mock-lock-holder".to_string());
    Some(vec![ProcInfo { pid: 4242, display }])
}

fn should_mock_kill_processes() -> bool {
    std::env::var("SYMM_TEST_LOCK_PATHS").is_ok()
}

fn mock_locks_clear_on_kill() -> bool {
    std::env::var("SYMM_TEST_LOCK_CLEAR_ON_KILL")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no"
            )
        })
        .unwrap_or(true)
}

#[cfg(not(windows))]
fn unix_list_locking_processes(path: &Path) -> Result<Vec<ProcInfo>, SymmError> {
    // 尽量使用系统常见自带工具：优先 fuser，其次 lsof；若都不可用则返回空列表。
    let p = path.to_string_lossy().to_string();

    let out = std::process::Command::new("fuser")
        .args(["-a", &p])
        .output();
    if let Ok(out) = out
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        let pids = text
            .split_whitespace()
            .filter_map(|t| t.parse::<u32>().ok())
            .collect::<Vec<_>>();
        return Ok(pids
            .into_iter()
            .map(|pid| ProcInfo {
                pid,
                display: format!("PID {pid}"),
            })
            .collect());
    }

    let out = std::process::Command::new("lsof")
        .args(["-t", "--", &p])
        .output();
    if let Ok(out) = out
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        let pids = text
            .lines()
            .filter_map(|l| l.trim().parse::<u32>().ok())
            .collect::<Vec<_>>();
        return Ok(pids
            .into_iter()
            .map(|pid| ProcInfo {
                pid,
                display: format!("PID {pid}"),
            })
            .collect());
    }

    Ok(vec![])
}

#[cfg(windows)]
fn windows_list_locking_processes<F>(
    path: &Path,
    progress: &mut F,
) -> Result<Vec<ProcInfo>, SymmError>
where
    F: FnMut(LockProbeProgress),
{
    use std::collections::BTreeMap;

    let probe_paths = collect_lock_probe_paths(path, progress)?;
    let mut dedup = BTreeMap::<u32, ProcInfo>::new();
    let total_batches = probe_paths.len().div_ceil(256).max(1);
    for (index, chunk) in probe_paths.chunks(256).enumerate() {
        progress(LockProbeProgress::Querying {
            batch: index + 1,
            total_batches,
        });
        for proc in windows_list_locking_process_batch(chunk)? {
            dedup.entry(proc.pid).or_insert(proc);
        }
    }
    Ok(dedup.into_values().collect())
}

#[cfg(windows)]
fn windows_list_locking_process_batch(paths: &[PathBuf]) -> Result<Vec<ProcInfo>, SymmError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::ERROR_MORE_DATA;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::RestartManager::{
        CCH_RM_MAX_APP_NAME, CCH_RM_MAX_SVC_NAME, CCH_RM_SESSION_KEY, RM_PROCESS_INFO,
        RM_UNIQUE_PROCESS, RmEndSession, RmGetList, RmRegisterResources, RmStartSession,
    };

    let mut session: u32 = 0;
    let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];

    let start = unsafe { RmStartSession(&mut session, 0, key.as_mut_ptr()) };
    if start != 0 {
        return Ok(vec![]);
    }

    let res = (|| {
        let wide_paths = paths
            .iter()
            .map(|path| {
                path.as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect::<Vec<u16>>()
            })
            .collect::<Vec<_>>();
        let resources = wide_paths
            .iter()
            .map(|path| path.as_ptr())
            .collect::<Vec<_>>();

        let reg = unsafe {
            RmRegisterResources(
                session,
                resources.len() as u32,
                resources.as_ptr(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
            )
        };
        if reg != 0 {
            return Ok(vec![]);
        }

        let mut needed: u32 = 0;
        let mut count: u32 = 0;
        let mut reason: u32 = 0;
        let first = unsafe {
            RmGetList(
                session,
                &mut needed,
                &mut count,
                std::ptr::null_mut(),
                &mut reason,
            )
        };

        if first == 0 && needed == 0 {
            return Ok(vec![]);
        }
        if first != ERROR_MORE_DATA && first != 0 {
            return Ok(vec![]);
        }

        let empty = RM_PROCESS_INFO {
            Process: RM_UNIQUE_PROCESS {
                dwProcessId: 0,
                ProcessStartTime: FILETIME {
                    dwLowDateTime: 0,
                    dwHighDateTime: 0,
                },
            },
            strAppName: [0; (CCH_RM_MAX_APP_NAME as usize) + 1],
            strServiceShortName: [0; (CCH_RM_MAX_SVC_NAME as usize) + 1],
            ApplicationType: 0,
            AppStatus: 0,
            TSSessionId: 0,
            bRestartable: 0,
        };
        let mut buf: Vec<RM_PROCESS_INFO> = vec![empty; needed as usize];
        count = needed;

        let second = unsafe {
            RmGetList(
                session,
                &mut needed,
                &mut count,
                buf.as_mut_ptr(),
                &mut reason,
            )
        };
        if second != 0 {
            return Ok(vec![]);
        }

        let mut out = Vec::with_capacity(count as usize);
        for info in buf.into_iter().take(count as usize) {
            let pid = info.Process.dwProcessId;
            let app = utf16_to_string(&info.strAppName);
            let svc = utf16_to_string(&info.strServiceShortName);
            let display = if !app.is_empty() {
                format!("PID {pid}  {app}")
            } else if !svc.is_empty() {
                format!("PID {pid}  {svc}")
            } else {
                format!("PID {pid}")
            };
            out.push(ProcInfo { pid, display });
        }
        Ok(out)
    })();

    unsafe { RmEndSession(session) };
    res
}

#[cfg(windows)]
fn utf16_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[cfg(windows)]
fn collect_lock_probe_paths<F>(path: &Path, progress: &mut F) -> Result<Vec<PathBuf>, SymmError>
where
    F: FnMut(LockProbeProgress),
{
    if !path.is_dir() {
        return Ok(vec![path.to_path_buf()]);
    }

    let mut out = Vec::new();
    let mut scanned_files = 0usize;
    collect_files_recursively(path, &mut out, &mut scanned_files, progress)?;
    if out.is_empty() {
        out.push(path.to_path_buf());
    }
    Ok(out)
}

#[cfg(windows)]
fn collect_files_recursively<F>(
    root: &Path,
    out: &mut Vec<PathBuf>,
    scanned_files: &mut usize,
    progress: &mut F,
) -> Result<(), SymmError>
where
    F: FnMut(LockProbeProgress),
{
    use std::fs;

    let entries = fs::read_dir(root).map_err(|e| SymmError::IoError {
        message: format!("无法扫描占用检测路径 {}：{e}", root.display()),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| SymmError::IoError {
            message: format!("无法读取占用检测目录项 {}：{e}", root.display()),
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| SymmError::IoError {
            message: format!("无法读取占用检测目录项类型 {}：{e}", path.display()),
        })?;
        if file_type.is_dir() {
            collect_files_recursively(&path, out, scanned_files, progress)?;
        } else {
            out.push(path);
            *scanned_files += 1;
            if *scanned_files == 1 || (*scanned_files).is_multiple_of(200) {
                progress(LockProbeProgress::Scanning {
                    scanned_files: *scanned_files,
                    current: out.last().cloned().unwrap_or_else(|| root.to_path_buf()),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn collect_lock_probe_paths_includes_files_inside_directory() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("root");
        let nested = root.join("nested");
        fs::create_dir_all(&nested).expect("create nested dir");
        let file = nested.join("config.json");
        fs::write(&file, "{}").expect("write file");

        let mut events = Vec::new();
        let paths =
            collect_lock_probe_paths(&root, &mut |e| events.push(e)).expect("collect probe paths");

        assert!(
            paths.iter().any(|path| path == &file),
            "directory lock probing should include nested files"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, LockProbeProgress::Scanning { .. })),
            "directory lock probing should emit scanning progress"
        );
    }
}
