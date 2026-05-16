//! 平台进程能力：占用检测与结束进程。

mod test_hooks;
mod unsupported;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use crate::domain::error::SymmError;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
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

pub trait PlatformProcess {
    fn list_locking_processes_with_progress<F>(
        &self,
        path: &Path,
        progress: &mut F,
    ) -> Result<Vec<ProcInfo>, SymmError>
    where
        F: FnMut(LockProbeProgress);

    fn kill_processes(&self, pids: &[u32]) -> Result<(), SymmError>;
}

#[cfg(unix)]
pub use unix::Platform;
#[cfg(not(any(unix, windows)))]
pub use unsupported::Platform;
#[cfg(windows)]
pub use windows::Platform;

pub fn platform() -> &'static Platform {
    static INSTANCE: Platform = Platform;
    &INSTANCE
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
    platform().list_locking_processes_with_progress(path, &mut progress)
}

pub fn kill_processes(pids: &[u32]) -> Result<(), SymmError> {
    if test_hooks::should_mock_kill_processes() {
        test_hooks::mark_mock_released_if_configured();
        return Ok(());
    }
    platform().kill_processes(pids)
}
