//! 文件占用检测与结束进程（编排层；底层 OS 调用在 `platform::process`）。

mod privileged;
mod snapshot;
mod test_hooks;

pub use crate::adapters::platform::process::{LockProbeProgress, ProcInfo};

use crate::adapters::platform::privilege;
use crate::adapters::platform::process::{PlatformProcess, platform};
use crate::domain::error::SymmError;
use std::path::Path;

fn use_direct_platform_ops() -> bool {
    privilege::is_privileged() || test_hooks::skip_privileged_lock_probe()
}

pub fn list_locking_processes_with_progress<F>(
    path: &Path,
    mut progress: F,
) -> Result<Vec<ProcInfo>, SymmError>
where
    F: FnMut(LockProbeProgress),
{
    if let Some(mocked) = test_hooks::mock_locking_processes(path) {
        return Ok(mocked);
    }

    if use_direct_platform_ops() {
        return platform().list_locking_processes_with_progress(path, &mut progress);
    }

    // Windows：同用户占用进程通常无需 UAC；仅失败时再走提权子进程 + 快照文件。
    #[cfg(windows)]
    if let Ok(procs) = platform().list_locking_processes_with_progress(path, &mut progress) {
        return Ok(procs);
    }

    progress(LockProbeProgress::Querying {
        batch: 1,
        total_batches: 1,
    });
    privileged::list_locking_processes(path)
}

pub fn kill_processes(pids: &[u32]) -> Result<(), SymmError> {
    if test_hooks::should_mock_kill_processes() {
        test_hooks::mark_mock_released_if_configured();
        return Ok(());
    }
    if pids.is_empty() {
        return Ok(());
    }

    if privilege::is_privileged() {
        return platform().kill_processes(pids);
    }

    privileged::kill_processes(pids)
}

pub fn elevated_list_locks_entry(path: &Path, output: &Path) -> Result<(), SymmError> {
    privileged::elevated_list_locks_entry(path, output)
}

pub fn elevated_kill_entry(pids: &[u32]) -> Result<(), SymmError> {
    platform().kill_processes(pids)
}
