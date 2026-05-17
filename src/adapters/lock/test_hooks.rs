use super::ProcInfo;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static MOCK_LOCK_RELEASED: AtomicBool = AtomicBool::new(false);

pub fn mock_locking_processes(path: &Path) -> Option<Vec<ProcInfo>> {
    let raw_paths = std::env::var("SYMM_TEST_LOCK_PATHS").ok()?;
    if mock_locks_clear_on_kill() && MOCK_LOCK_RELEASED.load(Ordering::SeqCst) {
        return Some(vec![]);
    }
    let mocked_paths = raw_paths
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    let current = path.to_string_lossy().to_string();
    if !mocked_paths
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&current))
    {
        return Some(vec![]);
    }

    let display = std::env::var("SYMM_TEST_LOCK_DISPLAY")
        .unwrap_or_else(|_| "PID 4242  mock-lock-holder".to_string());
    Some(vec![ProcInfo { pid: 4242, display }])
}

pub fn should_mock_kill_processes() -> bool {
    std::env::var("SYMM_TEST_LOCK_PATHS").is_ok()
}

/// 集成测试无交互 sudo 时，回退为当前用户直接查锁/杀进程。
pub fn skip_privileged_lock_probe() -> bool {
    std::env::var("SYMM_TEST_SKIP_PRIVILEGED_LOCK")
        .is_ok_and(|v| !matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "false" | "no"))
}

/// 集成测试默认跳过真实 OS 扫锁（Windows CI 上 RM 遍历大目录较慢）。
/// 显式设置 `SYMM_TEST_LOCK_PATHS` 时仍走 mock，不跳过。
pub fn skip_real_lock_probe_in_tests() -> bool {
    skip_privileged_lock_probe() && std::env::var("SYMM_TEST_LOCK_PATHS").is_err()
}

pub fn mark_mock_released_if_configured() {
    if mock_locks_clear_on_kill() {
        MOCK_LOCK_RELEASED.store(true, Ordering::SeqCst);
    }
}

pub fn mock_locks_clear_on_kill() -> bool {
    std::env::var("SYMM_TEST_LOCK_CLEAR_ON_KILL")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no"
            )
        })
        .unwrap_or(true)
}
