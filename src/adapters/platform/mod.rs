//! 平台相关能力：编译期静态分发（OS API + 提权检测），编排见 `adapters::symlink` / `migrate` / `lock`。
//!
//! 非业务逻辑优先用成熟库：`runas`（提权子进程）、Restart Manager（Windows 占用检测）、
//! `dunce`（路径规范化）；其余为薄封装或 symm 自有编排/协议。

#[cfg(all(not(unix), not(windows)))]
compile_error!("symm 仅支持 Linux、macOS 与 Windows");

pub mod host;
pub mod privilege;
pub mod process;

pub use host::{HostFs, format_relocate_failure, host as host_platform};
