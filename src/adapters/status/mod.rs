//! 读盘链状态（`ls` / `show` 用；不扫全库）。

mod probe;

pub use probe::{for_record, to_view};
