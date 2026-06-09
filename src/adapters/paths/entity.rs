use crate::adapters::symlink;
use crate::domain::error::SymmError;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntityFingerprint {
    inner: EntityFingerprintInner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EntityFingerprintInner {
    #[cfg(unix)]
    Unix {
        dev: u64,
        ino: u64,
        ctime: i64,
        ctime_nsec: i64,
        len: u64,
    },
    #[cfg(windows)]
    Windows {
        volume: u32,
        index: u64,
        creation_time: u64,
        last_write_time: u64,
        len: u64,
    },
}

impl EntityFingerprint {
    pub(crate) fn for_non_link_entity(path: &Path) -> Result<Self, SymmError> {
        let meta = fs::symlink_metadata(path).map_err(|e| SymmError::IoError {
            message: format!("无法确认 link 路径实体身份：{e}"),
        })?;
        if symlink::kind_from_path_and_metadata(path, &meta).is_some() {
            return Err(entity_changed(path));
        }
        Self::from_metadata(path, &meta)
    }

    #[cfg(unix)]
    fn from_metadata(_path: &Path, meta: &fs::Metadata) -> Result<Self, SymmError> {
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            inner: EntityFingerprintInner::Unix {
                dev: meta.dev(),
                ino: meta.ino(),
                ctime: meta.ctime(),
                ctime_nsec: meta.ctime_nsec(),
                len: meta.len(),
            },
        })
    }

    #[cfg(windows)]
    fn from_metadata(path: &Path, _meta: &fs::Metadata) -> Result<Self, SymmError> {
        let identity = windows_file_identity(path)?;
        Ok(Self {
            inner: EntityFingerprintInner::Windows {
                volume: identity.volume,
                index: identity.index,
                creation_time: identity.creation_time,
                last_write_time: identity.last_write_time,
                len: identity.len,
            },
        })
    }

    #[cfg(not(any(unix, windows)))]
    fn from_metadata(path: &Path, _meta: &fs::Metadata) -> Result<Self, SymmError> {
        Err(SymmError::InvalidArgument {
            message: format!(
                "当前平台无法可靠确认 link 路径实体身份，请重新执行本次操作：{}",
                path.display()
            ),
        })
    }
}

pub(crate) fn entity_changed(path: &Path) -> SymmError {
    SymmError::InvalidArgument {
        message: format!(
            "link 路径状态已变化，请重新执行本次操作：{}",
            path.display()
        ),
    }
}

#[cfg(windows)]
struct WindowsFileIdentity {
    volume: u32,
    index: u64,
    creation_time: u64,
    last_write_time: u64,
    len: u64,
}

#[cfg(windows)]
fn windows_file_identity(path: &Path) -> Result<WindowsFileIdentity, SymmError> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, GetFileInformationByHandle, OPEN_EXISTING,
    };
    use windows::core::PCWSTR;

    let wide_path =
        super::windows::verbatim_wide_path(path).ok_or_else(|| SymmError::InvalidArgument {
            message: format!(
                "无法可靠确认 link 路径实体身份，请重新执行本次操作：{}",
                path.display()
            ),
        })?;
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_path.as_ptr()),
            FILE_READ_ATTRIBUTES.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    }
    .map_err(|e| SymmError::IoError {
        message: format!("无法确认 link 路径实体身份：{e}"),
    })?;

    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    let result = unsafe { GetFileInformationByHandle(handle, &mut info) };
    let _ = unsafe { CloseHandle(handle) };
    result.map_err(|e| SymmError::IoError {
        message: format!("无法确认 link 路径实体身份：{e}"),
    })?;

    Ok(WindowsFileIdentity {
        volume: info.dwVolumeSerialNumber,
        index: ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
        creation_time: filetime_to_u64(info.ftCreationTime),
        last_write_time: filetime_to_u64(info.ftLastWriteTime),
        len: ((info.nFileSizeHigh as u64) << 32) | info.nFileSizeLow as u64,
    })
}

#[cfg(windows)]
fn filetime_to_u64(filetime: windows::Win32::Foundation::FILETIME) -> u64 {
    ((filetime.dwHighDateTime as u64) << 32) | filetime.dwLowDateTime as u64
}
