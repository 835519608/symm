//! 占用检测相关的用户提示文案（平台差异集中于此）。

/// 开始扫描占用前向用户展示的说明（可为空）。
pub fn pre_scan_notices() -> Vec<&'static str> {
    #[cfg(windows)]
    {
        let mut lines = Vec::new();
        if super::lock_probe_requests_uac() {
            lines.push(
                "将弹出 UAC 以扫描文件占用（只用于查/结束占用进程；迁移和建链仍在当前用户下执行）",
            );
            lines.push("若出现 UAC 对话框，请点击「是」");
        }
        if crate::adapters::platform::privilege::is_privileged() {
            lines.push(
                "提示：当前终端已是管理员；新建文件可能归管理员所有。建议用普通终端运行 symm，仅在 UAC 时授权",
            );
        }
        lines
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// 未发现占用进程时的补充说明；无则返回 `None`。
pub fn empty_lock_list_notice() -> Option<&'static str> {
    #[cfg(windows)]
    {
        Some("未发现占用进程；若仍提示文件被锁，请完全退出相关程序后重试")
    }
    #[cfg(not(windows))]
    {
        None
    }
}
