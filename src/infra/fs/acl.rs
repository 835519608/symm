use crate::domain::error::SymmError;
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;

#[cfg(windows)]
pub struct AclSnapshot {
    pub file: PathBuf,
}

#[cfg(not(windows))]
pub struct AclSnapshot;

#[cfg(windows)]
pub fn snapshot_dir_acl(src_dir: &Path) -> Result<Option<AclSnapshot>, SymmError> {
    let meta = std::fs::symlink_metadata(src_dir).map_err(|e| SymmError::IoError {
        message: format!("无法读取 ACL 源路径元数据：{e}"),
    })?;
    if !meta.is_dir() {
        return Ok(None);
    }

    let mut file = std::env::temp_dir();
    let pid = std::process::id();
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    file.push(format!("symm-acl-{pid}-{tick}.txt"));

    let status = std::process::Command::new("icacls")
        .arg(src_dir.as_os_str())
        .args(["/save"])
        .arg(&file)
        .args(["/t", "/c", "/q"])
        .status()
        .map_err(|e| SymmError::IoError {
            message: format!("执行 icacls /save 失败：{e}"),
        })?;
    if !status.success() {
        return Ok(None);
    }

    Ok(Some(AclSnapshot { file }))
}

#[cfg(not(windows))]
pub fn snapshot_dir_acl(_src_dir: &Path) -> Result<Option<AclSnapshot>, SymmError> {
    Ok(None)
}

#[cfg(windows)]
pub fn restore_dir_acl(dst_dir: &Path, snapshot: &AclSnapshot) -> Result<(), SymmError> {
    let status = std::process::Command::new("icacls")
        .arg(dst_dir.as_os_str())
        .args(["/restore"])
        .arg(&snapshot.file)
        .args(["/c", "/q"])
        .status()
        .map_err(|e| SymmError::IoError {
            message: format!("执行 icacls /restore 失败：{e}"),
        })?;
    if !status.success() {
        return Err(SymmError::PermissionDenied {
            message: format!("恢复 ACL 失败：{}", dst_dir.display()),
        });
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn restore_dir_acl(_dst_dir: &Path, _snapshot: &AclSnapshot) -> Result<(), SymmError> {
    Ok(())
}
