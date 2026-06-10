//! GUI 入口：Windows 使用 windows 子系统，避免启动时弹出控制台窗口。
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::PathBuf;

fn main() {
    let visible_startup = !is_elevated_child_invocation();
    install_panic_reporter(visible_startup);
    if let Err(err) = run() {
        eprintln!("{err}");
        if visible_startup {
            report_startup_failure(&err);
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match symm::app::dispatch::try_execute_elevated_from_args() {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(err) => {
            return Err(symm::ui::output::render_error_json(&err));
        }
    }
    symm::gui::run().map_err(|err| err.to_string())
}

fn is_elevated_child_invocation() -> bool {
    std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg.to_string_lossy().starts_with("__elevated-"))
}

fn install_panic_reporter(visible_startup: bool) {
    std::panic::set_hook(Box::new(move |info| {
        let message = format!("symm GUI 崩溃：{info}");
        eprintln!("{message}");
        if visible_startup {
            report_startup_failure(&message);
        }
    }));
}

fn report_startup_failure(message: &str) {
    let log_path = write_startup_log(message);
    show_startup_failure(message, log_path.as_deref());
}

fn write_startup_log(message: &str) -> Option<PathBuf> {
    let log_path = match symm::adapters::paths::home::data_home() {
        Ok(dir) => dir.join("symm-gui-startup.log"),
        Err(_) => std::env::temp_dir().join("symm-gui-startup.log"),
    };
    let body = format!("{message}\n");
    std::fs::write(&log_path, body).ok()?;
    Some(log_path)
}

#[cfg(windows)]
fn show_startup_failure(message: &str, log_path: Option<&std::path::Path>) {
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    use windows::core::PCWSTR;

    let text = match log_path {
        Some(path) => format!(
            "symm GUI 启动失败。\n\n{message}\n\n日志：{}",
            path.display()
        ),
        None => format!("symm GUI 启动失败。\n\n{message}"),
    };
    let title = "symm";
    let text = widestr(&text);
    let title = widestr(title);
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_startup_failure(_message: &str, _log_path: Option<&std::path::Path>) {}

#[cfg(windows)]
fn widestr(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
