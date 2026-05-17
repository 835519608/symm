//! GUI 入口：Windows 使用 windows 子系统，避免启动时弹出控制台窗口。
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() -> eframe::Result<()> {
    symm::gui::run()
}
