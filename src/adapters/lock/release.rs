//! 杀进程后的等待与面向 Windows 资源管理器的提示文案。

use super::ProcInfo;
use super::test_hooks;
use crate::domain::error::SymmError;
use std::path::Path;
use std::time::Duration;

/// 结束占用进程后：测试走 mock；真实环境仅短暂等待句柄释放，不再重复扫锁/UAC。
const AFTER_KILL_SETTLE_MS: u64 = 800;

pub fn is_explorer_process(proc: &ProcInfo) -> bool {
    proc.display.to_ascii_lowercase().contains("explorer.exe")
}

pub fn wait_after_kill(path: &Path) -> Result<Vec<ProcInfo>, SymmError> {
    if let Some(remaining) = test_hooks::mock_locking_processes(path) {
        return Ok(remaining);
    }
    std::thread::sleep(Duration::from_millis(AFTER_KILL_SETTLE_MS));
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
