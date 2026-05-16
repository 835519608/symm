//! 平台相关能力：管理员检查、文件系统、进程占用（编译期静态分发）。

pub mod admin;
pub mod fs;
pub mod process;

pub use fs::{PlatformFs, format_relocate_failure, fs as fs_platform};
pub use process::{
    LockProbeProgress, ProcInfo, kill_processes, list_locking_processes_with_progress,
};
