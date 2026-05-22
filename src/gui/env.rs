use crate::gui::state::{AddConflictPolicy, AddLockPolicy};
use std::env;
use std::path::Path;

/// 在闭包内临时设置环境变量，供 workflow 非交互分支读取。
pub fn with_env_vars<T>(pairs: &[(&str, &str)], f: impl FnOnce() -> T) -> T {
    let previous: Vec<_> = pairs.iter().map(|(k, _)| (*k, env::var(k).ok())).collect();
    for (key, value) in pairs {
        unsafe {
            env::set_var(key, value);
        }
    }
    let out = f();
    for (key, old) in previous {
        match old {
            Some(v) => unsafe {
                env::set_var(key, v);
            },
            None => unsafe {
                env::remove_var(key);
            },
        }
    }
    out
}

pub fn with_add_policies<T>(
    name: &str,
    lock: AddLockPolicy,
    conflict: AddConflictPolicy,
    f: impl FnOnce() -> T,
) -> T {
    let lock_v = match lock {
        AddLockPolicy::Unlock => "unlock",
        AddLockPolicy::Cancel => "cancel",
    };
    let conflict_v = match conflict {
        AddConflictPolicy::KeepLink => "link",
        AddConflictPolicy::KeepTarget => "target",
    };
    with_env_vars(
        &[
            ("SYMM_ADD_NAME", name),
            ("SYMM_ADD_LOCK_CHOICE", lock_v),
            ("SYMM_ADD_CONFLICT_CHOICE", conflict_v),
            ("SYMM_ADD_SYMLINK_CONFLICT_CHOICE", "retarget"),
        ],
        f,
    )
}

/// 将 `settings.json` 中的数据目录同步到 `SYMM_HOME`（空字符串表示恢复默认发现规则）。
pub fn sync_symm_home(data_dir: &str) {
    let trimmed = data_dir.trim();
    unsafe {
        if trimmed.is_empty() {
            let _ = env::remove_var("SYMM_HOME");
        } else {
            env::set_var("SYMM_HOME", trimmed);
        }
    }
}

/// 确保目录存在（设置页应用前校验）。
pub fn ensure_data_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}
