use super::{LockProbeProgress, PlatformProcess, ProcInfo};
use crate::domain::error::SymmError;
use std::path::Path;

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
        use filelocksmith::{
            find_processes_locking_path, pid_to_process_path, set_debug_privilege,
        };

        progress(LockProbeProgress::Querying {
            batch: 1,
            total_batches: 1,
        });
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

    fn kill_processes(&self, pids: &[u32]) -> Result<(), SymmError> {
        use filelocksmith::quit_processes;

        let pids = pids.iter().map(|pid| *pid as usize).collect::<Vec<_>>();
        if quit_processes(pids) {
            Ok(())
        } else {
            Err(SymmError::PermissionDenied {
                message: "无法结束占用进程（可能无权限）".to_string(),
            })
        }
    }
}
