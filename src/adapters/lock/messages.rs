//! 占用检测相关的用户提示文案（平台差异集中于此）。

/// 开始扫描占用前向用户展示的说明（可为空）。
pub fn pre_scan_notices() -> Vec<&'static str> {
    let mut lines = Vec::new();
    #[cfg(windows)]
    {
        if super::lock_probe_requests_uac() {
            lines.push(
                "即将通过 UAC 提权扫描占用（仅检测/结束占用进程；迁移与建链仍在当前用户下执行，请勿对整个终端「以管理员身份运行」）",
            );
        }
        if crate::adapters::platform::privilege::is_privileged() {
            lines.push(
                "提示：当前 symm 以管理员身份运行，新建目录可能归属管理员；建议使用普通终端运行 symm，仅在 UAC 提示时授权占用扫描",
            );
        }
    }
    lines
}

/// 未发现占用进程时的补充说明；无则返回 `None`。
pub fn empty_lock_list_notice() -> Option<&'static str> {
    #[cfg(windows)]
    {
        return Some("未发现占用进程；若迁移时报文件锁定，请先完全退出 Cursor 后再试");
    }
    #[cfg(not(windows))]
    {
        None
    }
}
