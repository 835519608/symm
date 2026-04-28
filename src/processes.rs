use crate::error::SymmError;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

static MOCK_LOCK_RELEASED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    use filelocksmith::quit_processes;

    if should_mock_kill_processes() {
        if mock_locks_clear_on_kill() {
            MOCK_LOCK_RELEASED.store(true, Ordering::SeqCst);
        }
        return Ok(());
    }
    let pids = pids.iter().map(|pid| *pid as usize).collect::<Vec<_>>();
    if quit_processes(pids) {
        Ok(())
    } else {
        Err(SymmError::PermissionDenied {
            message: "无法结束占用进程（可能无权限）".to_string(),
        })
    }
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
    use filelocksmith::{find_processes_locking_path, pid_to_process_path, set_debug_privilege};

    progress(LockProbeProgress::Querying {
        batch: 1,
        total_batches: 1,
    });
    // 提升 SeDebugPrivilege 可提高跨进程句柄枚举的可见性（在有权限时生效）。
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
