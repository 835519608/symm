//! 路径迁移编排（同盘移动、跨盘复制、树内 rebase）；写软链统一经 `symlink`。

mod copy_dir;
mod copy_file;
mod path;
mod rebase;
mod relocate_symlink;

pub use copy_file::copy_path_with_progress;
pub use path::{
    MigrationEvent, can_use_fast_move, fs_extra_error, migrate_path, move_path_with_retry,
    move_path_without_progress,
};
pub use rebase::{rebase_symlinks_in_tree, recreate_symlink, tree_contains_symlink};
