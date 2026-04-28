use crate::domain::error::SymmError;
#[cfg(not(windows))]
use crate::infra::errors::io_map::io_ctx;
use crate::infra::processes::lock_probe::{
    mark_mock_released, mock_locks_clear_on_kill, should_mock_kill_processes,
};

#[cfg(windows)]
pub fn kill_processes(pids: &[u32]) -> Result<(), SymmError> {
    use filelocksmith::quit_processes;

    if should_mock_kill_processes() {
        if mock_locks_clear_on_kill() {
            mark_mock_released();
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
            mark_mock_released();
        }
        return Ok(());
    }
    for pid in pids {
        let status = std::process::Command::new("kill")
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
