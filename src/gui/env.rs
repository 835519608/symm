use std::env;
use std::path::Path;

/// 将 `settings.json` 中的数据目录同步到 `SYMM_HOME`（空字符串表示恢复默认发现规则）。
pub fn sync_symm_home(data_dir: &str) {
    let trimmed = data_dir.trim();
    unsafe {
        if trimmed.is_empty() {
            env::remove_var("SYMM_HOME");
        } else {
            env::set_var("SYMM_HOME", trimmed);
        }
    }
}

/// 确保目录存在（设置页应用前校验）。
pub fn ensure_data_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}
