//! 读盘链状态（`ls` / `show` 用；不扫全库）。

mod probe;

pub use probe::{for_record, probe_record, to_view, try_for_record, view_from_probe};
