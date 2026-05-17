//! 当前进程是否已具备管理员 / root 权限（供 lock、fs/link 等策略层共用）。

#[cfg(windows)]
pub fn is_privileged() -> bool {
    filelocksmith::is_process_elevated()
}

#[cfg(unix)]
pub fn is_privileged() -> bool {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|uid| uid.trim() == "0")
        .unwrap_or(false)
}
