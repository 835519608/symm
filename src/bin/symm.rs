//! GUI 入口：Windows 使用 windows 子系统，避免启动时弹出控制台窗口。
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() -> eframe::Result<()> {
    match symm::app::dispatch::try_execute_elevated_from_args() {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(err) => {
            eprintln!("{}", symm::ui::output::render_error_json(&err));
            std::process::exit(1);
        }
    }
    symm::gui::run()
}
