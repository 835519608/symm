use super::{LockProbeProgress, PlatformProcess, ProcInfo};
use crate::adapters::errors::io::io_ctx;
use crate::domain::error::SymmError;
use std::path::Path;
use std::process::Command;

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
        progress(LockProbeProgress::Querying {
            batch: 1,
            total_batches: 1,
        });
        list_locking_processes_direct(path)
    }

    fn kill_processes(&self, pids: &[u32]) -> Result<(), SymmError> {
        kill_processes_direct(pids)
    }
}

pub(crate) fn list_locking_processes_direct(path: &Path) -> Result<Vec<ProcInfo>, SymmError> {
    let p = path.to_string_lossy().to_string();

    if let Ok(out) = Command::new("fuser").args(["-a", &p]).output()
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        return Ok(parse_pids(&text));
    }

    if let Ok(out) = Command::new("lsof").args(["-t", "--", &p]).output()
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        return Ok(parse_pids(&text));
    }

    Ok(vec![])
}

pub(crate) fn kill_processes_direct(pids: &[u32]) -> Result<(), SymmError> {
    for pid in pids {
        let status = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .map_err(|e| io_ctx("执行 kill 失败", e))?;
        if !status.success() {
            return Err(SymmError::PermissionDenied {
                message: format!("无法结束进程 PID={pid}（可能无权限）"),
            });
        }
    }
    Ok(())
}

fn parse_pids(text: &str) -> Vec<ProcInfo> {
    text.split_whitespace()
        .filter_map(|t| t.parse::<u32>().ok())
        .map(|pid| ProcInfo {
            pid,
            display: format!("PID {pid}"),
        })
        .collect()
}
