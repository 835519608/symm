//! egui 桌面界面（`symm` 可执行文件，feature `gui`）。

mod app;
mod data;
mod env;
mod icon;
mod panels;
mod settings_store;
mod state;
mod theme;
mod util;
mod widgets;

pub use app::run;
