//! 文件占用检测与结束进程（编排层；底层 OS 调用在 `platform::process`）。

mod messages;
mod privileged;
mod release;
mod snapshot;
mod test_hooks;

pub use messages::{empty_lock_list_notice, pre_scan_notices};
pub use release::{format_still_locked_message, poll_until_unlocked};

pub use crate::adapters::platform::process::{LockProbeProgress, ProcInfo};

use crate::adapters::platform::privilege;
use crate::adapters::platform::process::{PlatformProcess, platform};
use crate::domain::error::SymmError;
use std::path::Path;

fn use_direct_platform_ops() -> bool {
    privilege::is_privileged() || test_hooks::skip_privileged_lock_probe()
}

#[cfg(windows)]
fn uac_cancelled_by_user(err: &SymmError) -> bool {
    matches!(
        err,
        SymmError::PermissionDenied { message }
            if message.contains("取消 UAC")
    )
}

/// Windows：当前进程非管理员时，占用检测会弹出 UAC 提权子进程（仅扫描/结束占用，不提升主进程）。
#[cfg(windows)]
pub fn lock_probe_requests_uac() -> bool {
    !use_direct_platform_ops()
}

#[cfg(not(windows))]
pub fn lock_probe_requests_uac() -> bool {
    false
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

    if test_hooks::skip_real_lock_probe_in_tests() {
        return Ok(vec![]);
    }

    if use_direct_platform_ops() {
        return platform().list_locking_processes_with_progress(path, &mut progress);
    }

    #[cfg(windows)]
    {
        // 非管理员：filelocksmith 需 SeDebug 才能枚举部分句柄；与结束占用一致走 UAC 提权子进程。
        progress(LockProbeProgress::Querying {
            batch: 1,
            total_batches: 1,
        });
        match privileged::list_locking_processes(path) {
            Ok(procs) => Ok(procs),
            Err(err) if uac_cancelled_by_user(&err) => Err(err),
            Err(elevated_err) => {
                if let Ok(procs) =
                    platform().list_locking_processes_with_progress(path, &mut progress)
                    && !procs.is_empty()
                {
                    return Ok(procs);
                }
                Err(elevated_err)
            }
        }
    }

    #[cfg(unix)]
    {
        if let Ok(procs) = platform().list_locking_processes_with_progress(path, &mut progress) {
            return Ok(procs);
        }
        progress(LockProbeProgress::Querying {
            batch: 1,
            total_batches: 1,
        });
        privileged::list_locking_processes(path)
    }
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
