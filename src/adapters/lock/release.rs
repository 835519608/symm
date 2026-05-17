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
        "链接位置仍被占用：{}（还剩 {} 个进程，例如 {}）",
        link.display(),
        remaining.len(),
        remaining[0]
    );
    if remaining.iter().all(is_explorer_process) {
        format!(
            "{base}。请关闭正在浏览该文件夹的资源管理器窗口，并退出其它可能占用该路径的程序后重试"
        )
    } else if remaining.iter().any(is_explorer_process) {
        format!("{base}。请关闭相关文件夹窗口后重试")
    } else if remaining.len() == 1 {
        format!("{base}。请完全退出占用程序（含托盘/后台）后重试")
    } else {
        format!("{base}。请完全退出上述占用程序（含托盘/后台）后重试")
    }
}
