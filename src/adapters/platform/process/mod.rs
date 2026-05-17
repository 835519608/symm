//! 平台进程能力：仅 OS API 封装（静态分发）。

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
#[cfg(windows)]
pub use windows::Platform;

pub fn platform() -> &'static Platform {
    static INSTANCE: Platform = Platform;
    &INSTANCE
}
