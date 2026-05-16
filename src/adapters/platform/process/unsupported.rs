use super::{LockProbeProgress, PlatformProcess, ProcInfo};
use crate::domain::error::SymmError;
use std::path::Path;

#[allow(dead_code)]
pub struct Platform;

impl PlatformProcess for Platform {
    fn list_locking_processes_with_progress<F>(
        &self,
        _path: &Path,
        _progress: &mut F,
    ) -> Result<Vec<ProcInfo>, SymmError>
    where
        F: FnMut(LockProbeProgress),
    {
        Ok(vec![])
    }

    fn kill_processes(&self, _pids: &[u32]) -> Result<(), SymmError> {
        Ok(())
    }
}
