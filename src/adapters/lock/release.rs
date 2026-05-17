//! 解除占用后的轮询与面向 Windows 资源管理器的提示文案。

use super::test_hooks;
use super::{ProcInfo, list_locking_processes_shallow_for_poll};
use crate::domain::error::SymmError;
use std::path::Path;
use std::time::Duration;

const POLL_ATTEMPTS: usize = 24;
const POLL_INTERVAL: Duration = Duration::from_millis(250);

pub fn is_explorer_process(proc: &ProcInfo) -> bool {
    proc.display.to_ascii_lowercase().contains("explorer.exe")
}

pub fn poll_until_unlocked(path: &Path) -> Result<Vec<ProcInfo>, SymmError> {
    let attempts = poll_attempts();
    for attempt in 0..attempts {
        let remaining = list_locking_processes_shallow_for_poll(path)?;
        if remaining.is_empty() {
            return Ok(vec![]);
        }
        if attempt + 1 < attempts {
            std::thread::sleep(poll_interval());
        } else {
            return Ok(remaining);
        }
    }
    Ok(vec![])
}

pub fn format_still_locked_message(link: &Path, remaining: &[ProcInfo]) -> String {
    let base = format!(
        "link 路径仍被占用，未执行 add：{}（剩余 {} 个进程，示例：{}）",
        link.display(),
        remaining.len(),
        remaining[0]
    );
    if remaining.iter().all(is_explorer_process) {
        format!(
            "{base}。资源管理器（explorer.exe）结束后会自动重启并可能再次占用该路径；请先关闭正在浏览该路径或其父目录的文件夹窗口，并关闭其它可能占用该路径的程序后重试"
        )
    } else if remaining.iter().any(is_explorer_process) {
        format!("{base}。其中包含资源管理器（explorer.exe），请关闭相关文件夹窗口后重试")
    } else if remaining.len() == 1 {
        format!(
            "{base}。请结束或完全退出占用进程（{}，含托盘/后台实例）后重试",
            remaining[0]
        )
    } else {
        format!("{base}。请结束或完全退出上述占用进程（含托盘/后台实例）后重试")
    }
}

fn poll_attempts() -> usize {
    if test_hooks::should_mock_kill_processes() && !test_hooks::mock_locks_clear_on_kill() {
        1
    } else {
        POLL_ATTEMPTS
    }
}

fn poll_interval() -> Duration {
    if test_hooks::should_mock_kill_processes() {
        Duration::from_millis(0)
    } else {
        POLL_INTERVAL
    }
}
