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
            total_batches: Some(1),
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
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let pids = parse_fuser_pids(&text);
        if !text.trim().is_empty() && pids.is_empty() {
            // `fuser -a` can print path headers or PID access flags; let lsof try if no PID survived.
        } else {
            return Ok(pids);
        }
    }

    if let Ok(out) = Command::new("lsof").args(["-t", "--", &p]).output()
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        return Ok(parse_plain_pids(&text));
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

fn parse_plain_pids(text: &str) -> Vec<ProcInfo> {
    text.split_whitespace()
        .filter_map(|t| t.parse::<u32>().ok())
        .filter(|pid| *pid != std::process::id())
        .map(proc_info)
        .collect()
}

fn parse_fuser_pids(text: &str) -> Vec<ProcInfo> {
    text.split_whitespace()
        .filter_map(parse_leading_pid)
        .filter(|pid| *pid != std::process::id())
        .map(proc_info)
        .collect()
}

fn parse_leading_pid(token: &str) -> Option<u32> {
    let digits: String = token.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u32>().ok()
}

fn proc_info(pid: u32) -> ProcInfo {
    ProcInfo {
        pid,
        display: format!("PID {pid}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_fuser_pids, parse_plain_pids};

    #[test]
    fn fuser_parser_accepts_access_flags_after_pid() {
        let pids = parse_fuser_pids("/tmp/demo: 1234c 5678f 9012");

        assert_eq!(
            pids.into_iter().map(|proc| proc.pid).collect::<Vec<_>>(),
            vec![1234, 5678, 9012]
        );
    }

    #[test]
    fn fuser_parser_ignores_path_headers_without_pid() {
        let pids = parse_fuser_pids("/tmp/demo:");

        assert!(pids.is_empty());
    }

    #[test]
    fn plain_pid_parser_rejects_fuser_access_flags() {
        let pids = parse_plain_pids("1234c\n5678\n");

        assert_eq!(
            pids.into_iter().map(|proc| proc.pid).collect::<Vec<_>>(),
            vec![5678]
        );
    }
}
