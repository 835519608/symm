use super::HostFs;
use super::error::{RelocateFailure, map_link_io_error};
use crate::adapters::errors::io::ioe;
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::fs;
use std::fs::Metadata;
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Host;

impl HostFs for Host {
    fn create_link(&self, target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
        create_link_direct(target, link)
    }

    fn write_symlink(&self, link: &Path, target: &Path) -> Result<(), SymmError> {
        write_symlink_direct(link, target)
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
                Err(RelocateFailure::symlink_rename_denied(e))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LinkWriteKind {
    FileSymlink,
    DirSymlink,
    Junction,
}

impl LinkWriteKind {
    pub(crate) fn as_arg(self) -> &'static str {
        match self {
            Self::FileSymlink => "file-symlink",
            Self::DirSymlink => "dir-symlink",
            Self::Junction => "junction",
        }
    }

    fn from_arg(value: &str) -> Result<Self, SymmError> {
        match value {
            "file-symlink" => Ok(Self::FileSymlink),
            "dir-symlink" => Ok(Self::DirSymlink),
            "junction" => Ok(Self::Junction),
            _ => Err(SymmError::InvalidArgument {
                message: format!("未知链接写入类型：{value}"),
            }),
        }
    }
}

/// 提权子进程入口：仅创建链接（直接 OS API，不再递归提权）。
pub fn elevated_create_link_entry(
    target: &Path,
    link: &Path,
    link_kind: Option<&str>,
) -> Result<(), SymmError> {
    if let Some(link_kind) = link_kind {
        let kind = LinkWriteKind::from_arg(link_kind)?;
        return write_link_kind_direct(kind, target, link);
    }
    create_link_direct(target, link).map(|_| ())
}

pub fn create_link_direct(target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
    if target.is_dir() {
        match symlink_dir(target, link) {
            Ok(()) => return Ok(LinkKind::Symlink),
            Err(e) => {
                let mapped = map_link_io_error(e);
                match create_junction(target, link) {
                    Ok(()) => return Ok(LinkKind::Junction),
                    Err(_) if needs_link_elevation(&mapped) => {
                        return Err(mapped);
                    }
                    Err(junction_err) => return Err(junction_err),
                }
            }
        }
    }

    symlink_file(target, link).map_err(map_link_io_error)?;
    Ok(LinkKind::Symlink)
}

pub fn write_symlink_direct(link: &Path, target: &Path) -> Result<(), SymmError> {
    let is_dir_link = match fs::metadata(target) {
        Ok(m) => m.is_dir(),
        Err(_) => false,
    };
    let kind = if is_dir_link {
        LinkWriteKind::DirSymlink
    } else {
        LinkWriteKind::FileSymlink
    };
    write_link_kind_direct(kind, target, link)
}

pub(crate) fn infer_link_write_kind(src_link: &Path) -> Result<LinkWriteKind, SymmError> {
    let meta = fs::symlink_metadata(src_link).map_err(ioe)?;
    match symlink::kind_from_path_and_metadata(src_link, &meta)? {
        Some(LinkKind::Junction) => Ok(LinkWriteKind::Junction),
        Some(LinkKind::Symlink) if is_directory_reparse_point(&meta) => {
            Ok(LinkWriteKind::DirSymlink)
        }
        Some(LinkKind::Symlink) => Ok(LinkWriteKind::FileSymlink),
        None => Err(SymmError::InvalidArgument {
            message: format!("不是可重建的链接：{}", src_link.display()),
        }),
    }
}

pub(crate) fn infer_repoint_write_kind(
    src_link: &Path,
    target: &Path,
) -> Result<LinkWriteKind, SymmError> {
    let original = infer_link_write_kind(src_link)?;
    if original == LinkWriteKind::Junction {
        return Ok(LinkWriteKind::Junction);
    }
    Ok(link_write_kind_for_symlink_target(target))
}

fn link_write_kind_for_symlink_target(target: &Path) -> LinkWriteKind {
    if fs::metadata(target)
        .map(|meta| meta.is_dir())
        .unwrap_or(false)
    {
        LinkWriteKind::DirSymlink
    } else {
        LinkWriteKind::FileSymlink
    }
}

pub(crate) fn write_link_kind_direct(
    kind: LinkWriteKind,
    target: &Path,
    link: &Path,
) -> Result<(), SymmError> {
    match kind {
        LinkWriteKind::FileSymlink => symlink_file(target, link).map_err(ioe),
        LinkWriteKind::DirSymlink => symlink_dir(target, link).map_err(ioe),
        LinkWriteKind::Junction => create_junction(target, link),
    }
}

pub fn needs_link_elevation(err: &SymmError) -> bool {
    match err {
        SymmError::PermissionDenied { .. } => true,
        SymmError::IoError { message } => {
            message.contains("1314") || message.to_ascii_lowercase().contains("privilege")
        }
        _ => false,
    }
}

pub fn infer_link_kind_after_elevated(target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
    infer_existing_link_kind(link)?.ok_or_else(|| SymmError::IoError {
        message: format!(
            "提权创建链接后无法识别链接类型：{} -> {}",
            link.display(),
            target.display()
        ),
    })
}

fn infer_existing_link_kind(link: &Path) -> Result<Option<LinkKind>, SymmError> {
    let meta = fs::symlink_metadata(link).map_err(ioe)?;
    symlink::kind_from_path_and_metadata(link, &meta)
}

fn is_directory_reparse_point(meta: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    let attrs = meta.file_attributes();
    (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0 && (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0
}

fn path_prefix(path: &Path) -> Option<String> {
    path.components().find_map(|component| match component {
        Component::Prefix(prefix) => Some(prefix.as_os_str().to_string_lossy().to_string()),
        _ => None,
    })
}

fn create_junction(target: &Path, link: &Path) -> Result<(), SymmError> {
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.to_ascii_lowercase().contains("privilege")
            || stderr.contains("1314")
            || stderr.to_ascii_lowercase().contains("access is denied")
        {
            return Err(SymmError::PermissionDenied {
                message: stderr.into_owned(),
            });
        }
        Err(SymmError::IoError {
            message: stderr.into_owned(),
        })
    }
}
