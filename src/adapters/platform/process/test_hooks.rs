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

pub fn mark_mock_released_if_configured() {
    if mock_locks_clear_on_kill() {
        MOCK_LOCK_RELEASED.store(true, Ordering::SeqCst);
    }
}

fn mock_locks_clear_on_kill() -> bool {
    std::env::var("SYMM_TEST_LOCK_CLEAR_ON_KILL")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no"
            )
        })
        .unwrap_or(true)
}
