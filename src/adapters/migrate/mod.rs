//! 路径迁移编排（同盘移动、跨盘复制、树内 rebase）；写软链统一经 `symlink`。

mod copy_dir;
mod copy_file;
mod path;
mod rebase;
mod relocate_symlink;

pub use path::{MigrationEvent, migrate_path};
