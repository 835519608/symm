use crate::error::SymmError;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub display: String,
}

pub fn list_locking_processes(path: &Path) -> Result<Vec<ProcInfo>, SymmError> {
    #[cfg(windows)]
    {
        return windows_list_locking_processes(path);
    }

    #[cfg(not(windows))]
    {
        return unix_list_locking_processes(path);
    }
}

pub fn kill_processes(pids: &[u32]) -> Result<(), SymmError> {
    #[cfg(windows)]
    {
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
        return Ok(());
    }

    #[cfg(not(windows))]
    {
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
        return Ok(());
    }
}

#[cfg(not(windows))]
fn unix_list_locking_processes(path: &Path) -> Result<Vec<ProcInfo>, SymmError> {
    // 尽量使用系统常见自带工具：优先 fuser，其次 lsof；若都不可用则返回空列表。
    let p = path.to_string_lossy().to_string();

    let out = std::process::Command::new("fuser")
        .args(["-a", &p])
        .output();
    if let Ok(out) = out {
        if out.status.success() {
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
    }

    let out = std::process::Command::new("lsof")
        .args(["-t", "--", &p])
        .output();
    if let Ok(out) = out {
        if out.status.success() {
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
    }

    Ok(vec![])
}

#[cfg(windows)]
fn windows_list_locking_processes(path: &Path) -> Result<Vec<ProcInfo>, SymmError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::ERROR_MORE_DATA;
    use windows_sys::Win32::System::RestartManager::{
        CCH_RM_SESSION_KEY, RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources,
        RmStartSession,
    };

    let mut session: u32 = 0;
    let mut key = [0u16; CCH_RM_SESSION_KEY as usize + 1];

    let start = unsafe { RmStartSession(&mut session, 0, key.as_mut_ptr()) };
    if start != 0 {
        return Ok(vec![]);
    }

    let res = (|| {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let resources = [wide.as_ptr()];

        let reg = unsafe {
            RmRegisterResources(
                session,
                1,
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

        let mut buf: Vec<RM_PROCESS_INFO> = Vec::with_capacity(needed as usize);
        unsafe { buf.set_len(needed as usize) };
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
