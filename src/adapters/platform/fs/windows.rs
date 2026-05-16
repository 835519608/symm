use super::PlatformFs;
use super::error::{RelocateFailure, map_link_io_error};
use crate::adapters::errors::io_map::ioe;
use crate::adapters::fs::path_ops;
use crate::adapters::fs::rebase;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::fs;
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Platform;

impl PlatformFs for Platform {
    fn create_link(&self, target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
        if target.is_dir() {
            match symlink_dir(target, link) {
                Ok(()) => return Ok(LinkKind::Symlink),
                Err(_) => {
                    create_junction(target, link)?;
                    return Ok(LinkKind::Junction);
                }
            }
        }

        symlink_file(target, link).map_err(map_link_io_error)?;
        Ok(LinkKind::Symlink)
    }

    fn write_symlink(&self, link: &Path, target: &Path) -> Result<(), SymmError> {
        let is_dir_link = match fs::metadata(target) {
            Ok(m) => m.is_dir(),
            Err(_) => true,
        };
        if is_dir_link {
            symlink_dir(target, link).map_err(ioe)?;
        } else {
            symlink_file(target, link).map_err(ioe)?;
        }
        Ok(())
    }

    fn same_volume(&self, a: &Path, b: &Path) -> Result<bool, SymmError> {
        Ok(path_prefix(a) == path_prefix(b))
    }

    fn relocate_path(&self, src: &Path, dst: &Path) -> Result<(), RelocateFailure> {
        match fs::rename(src, dst) {
            Ok(()) => Ok(()),
            Err(e)
                if e.raw_os_error() == Some(5)
                    && fs::symlink_metadata(src)
                        .map(|m| m.file_type().is_symlink())
                        .unwrap_or(false) =>
            {
                relocate_symlink_by_recreate(src, dst).map_err(|inner| RelocateFailure {
                    inner,
                    access_denied: false,
                })
            }
            Err(e) => Err(RelocateFailure::from_io(e)),
        }
    }

    fn snapshot_dir_acl(&self, src_dir: &Path) -> Result<Option<PathBuf>, SymmError> {
        let meta = fs::symlink_metadata(src_dir).map_err(|e| SymmError::IoError {
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

        let status = Command::new("icacls")
            .arg(src_dir.as_os_str())
            .args(["/save"])
            .arg(&file)
            .args(["/t", "/c", "/q"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| SymmError::IoError {
                message: format!("执行 icacls /save 失败：{e}"),
            })?;
        if !status.success() {
            return Ok(None);
        }

        Ok(Some(file))
    }

    fn restore_dir_acl(&self, dst_dir: &Path, snapshot: &Path) -> Result<(), SymmError> {
        let status = Command::new("icacls")
            .arg(dst_dir.as_os_str())
            .args(["/restore"])
            .arg(snapshot)
            .args(["/c", "/q"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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
}

fn path_prefix(path: &Path) -> Option<String> {
    path.components().find_map(|component| match component {
        Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().to_string()),
        _ => None,
    })
}

fn create_junction(target: &Path, link: &Path) -> Result<(), SymmError> {
    let target_s = target.to_string_lossy().to_string();
    let link_s = link.to_string_lossy().to_string();
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J", &link_s, &target_s])
        .output()
        .map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(SymmError::IoError {
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

fn relocate_symlink_by_recreate(src: &Path, dst: &Path) -> Result<(), SymmError> {
    if dst.exists() {
        path_ops::remove_path_any(dst)?;
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(ioe)?;
    }
    rebase::recreate_symlink(src, dst, Some((src, dst)))?;
    path_ops::remove_path_any(src)?;
    Ok(())
}
