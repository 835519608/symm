use crate::error::SymmError;
use crate::model::{LinkKind, LinkRecord, LinkStatus, LinkView};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn create_link(target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
        return Ok(LinkKind::Symlink);
    }

    #[cfg(windows)]
    {
        if target.is_dir() {
            match std::os::windows::fs::symlink_dir(target, link) {
                Ok(_) => return Ok(LinkKind::Symlink),
                Err(_) => {
                    // Windows 下软链接可能因权限策略失败，目录场景回退为 junction。
                    create_junction(target, link)?;
                    return Ok(LinkKind::Junction);
                }
            }
        }

        std::os::windows::fs::symlink_file(target, link).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                SymmError::PermissionDenied {
                    message: e.to_string(),
                }
            } else {
                SymmError::IoError {
                    message: e.to_string(),
                }
            }
        })?;
        return Ok(LinkKind::Symlink);
    }

    #[allow(unreachable_code)]
    Err(SymmError::InvalidArgument {
        message: "不支持的平台".to_string(),
    })
}

#[cfg(windows)]
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

pub fn remove_link(link: &Path) -> Result<(), SymmError> {
    match fs::symlink_metadata(link) {
        Ok(meta) => {
            let file_type = meta.file_type();
            if file_type.is_dir() {
                fs::remove_dir(link).map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
            } else {
                fs::remove_file(link).map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(SymmError::IoError {
            message: e.to_string(),
        }),
    }
}

pub fn status_for(record: &LinkRecord) -> LinkStatus {
    let link = Path::new(&record.link_path);
    if !link.exists() {
        return LinkStatus::Missing;
    }
    let target = Path::new(&record.target_path);
    if target.exists() {
        LinkStatus::Ok
    } else {
        LinkStatus::Broken
    }
}

pub fn as_view(record: LinkRecord) -> LinkView {
    let status = status_for(&record);
    LinkView {
        name: record.name,
        link_path: record.link_path,
        target_path: record.target_path,
        link_kind: record.link_kind,
        status,
    }
}
