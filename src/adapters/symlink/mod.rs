//! 软链创建/写入/删除（唯一对外入口；Windows 提权策略在 `windows` 子模块）。

mod link;
mod unlink;
#[cfg(windows)]
mod windows;

pub use link::{create_link, write_symlink};
pub use unlink::unlink;
