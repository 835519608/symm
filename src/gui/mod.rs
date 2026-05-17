//! egui 桌面界面（`symm-gui`，feature `gui`）。
//!
//! 布局参考常见数据库管理工具：顶栏工具、左侧树、中央仪表盘、底栏状态。
//! 数据经 `adapters::db` 与 `workflows::list_views` 读取，不重复业务逻辑。

mod app;
mod data;
mod panels;
mod state;
mod theme;
mod widgets;

pub use app::run;
