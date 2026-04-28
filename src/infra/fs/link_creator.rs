use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::path::Path;
#[cfg(windows)]
use std::process::Command;

pub fn create_link(target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).map_err(|e| {
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

    #[cfg(windows)]
    {
        if target.is_dir() {
            match std::os::windows::fs::symlink_dir(target, link) {
                Ok(_) => return Ok(LinkKind::Symlink),
                Err(_) => {
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
