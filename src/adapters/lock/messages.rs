//! 占用检测相关的用户提示文案（平台差异集中于此）。

/// 开始扫描占用前向用户展示的说明（可为空）。
pub fn pre_scan_notices() -> Vec<&'static str> {
    #[cfg(windows)]
    {
        let mut lines = Vec::new();
        if super::lock_probe_requests_uac() {
            lines.push(
                "将通过 UAC 提权扫描占用（Restart Manager，按迁移目录文件清单；仅检测/结束占用进程；迁移与建链仍在当前用户下执行，请勿对整个终端「以管理员身份运行」）",
            );
            lines.push(
                "若未出现 UAC 对话框，请检查系统「用户账户控制」是否开启；出现后请点击「是」",
            );
        }
        if crate::adapters::platform::privilege::is_privileged() {
            lines.push(
                "提示：当前 symm 以管理员身份运行，新建目录可能归属管理员；建议使用普通终端运行 symm，仅在 UAC 提示时授权占用扫描",
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
        Some(
            "未发现占用进程；若迁移时报文件被锁定，请关闭正在使用该路径的程序（如编辑器、资源管理器窗口）后重试",
        )
    }
    #[cfg(not(windows))]
    {
        None
    }
}
