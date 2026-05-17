//! 软链创建/写入/删除（唯一对外入口；Windows 提权策略在 `windows` 子模块）。

mod create;
mod remove;
#[cfg(windows)]
mod windows;

pub use create::{create_link, write_symlink};
pub use remove::remove_link;
