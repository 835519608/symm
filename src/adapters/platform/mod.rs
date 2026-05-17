//! 平台相关能力：编译期静态分发（OS API + 提权检测），编排见 `adapters::fs` / `adapters::lock`。

#[cfg(all(not(unix), not(windows)))]
compile_error!("symm 仅支持 Linux、macOS 与 Windows");

#[cfg(windows)]
pub mod elevate;
pub mod fs;
pub mod privilege;
pub mod process;

pub use fs::{PlatformFs, format_relocate_failure, fs as fs_platform};
