//! 软链创建/写入/删除（唯一对外入口；Windows 提权策略在 `windows` 子模块）。

mod inspect;
mod link;
mod unlink;
#[cfg(windows)]
mod windows;

pub use inspect::{LinkPathState, inspect_link_path, kind_from_path_and_metadata, link_points_to};
pub(crate) use link::{
    LinkRecreateSpec, capture_recreate_spec, write_symlink_from_spec, write_symlink_like,
};
pub use link::{create_link, write_symlink};
pub use unlink::unlink;
