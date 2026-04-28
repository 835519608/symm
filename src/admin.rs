#[cfg(windows)]
pub fn is_elevated() -> bool {
    filelocksmith::is_process_elevated()
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    true
}
