use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::fs;
use std::fs::Metadata;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkPathState {
    Missing,
    Link { kind: LinkKind },
    Entity,
}

pub fn inspect_link_path(path: &Path) -> Result<LinkPathState, SymmError> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LinkPathState::Missing);
        }
        Err(err) => {
            return Err(SymmError::IoError {
                message: format!("无法读取 link 路径：{err}"),
            });
        }
    };
    Ok(match kind_from_path_and_metadata(path, &meta) {
        Some(kind) => LinkPathState::Link { kind },
        None => LinkPathState::Entity,
    })
}

pub fn link_points_to(link: &Path, expected: &Path) -> Result<bool, SymmError> {
    let actual = fs::read_link(link).map_err(|e| SymmError::IoError {
        message: format!("无法读取 link 指向：{e}"),
    })?;
    Ok(paths_match(resolve_link_target(link, actual)?, expected))
}

fn resolve_link_target(link: &Path, target: PathBuf) -> Result<PathBuf, SymmError> {
    if target.is_absolute() {
        return Ok(target);
    }
    let parent = link.parent().ok_or_else(|| SymmError::InvalidArgument {
        message: "无法解析 link 父目录".to_string(),
    })?;
    Ok(parent.join(target))
}

fn paths_match(actual: PathBuf, expected: &Path) -> bool {
    let expected = if expected.is_absolute() {
        expected.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(expected))
            .unwrap_or_else(|_| expected.to_path_buf())
    };
    if crate::adapters::paths::lexical::clean(&actual)
        == crate::adapters::paths::lexical::clean(&expected)
    {
        return true;
    }
    match (dunce::canonicalize(actual), dunce::canonicalize(expected)) {
        (Ok(a), Ok(e)) => a == e,
        _ => false,
    }
}

pub fn kind_from_path_and_metadata(path: &Path, meta: &Metadata) -> Option<LinkKind> {
    #[cfg(windows)]
    {
        windows_kind_from_path_and_metadata(path, meta)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        kind_from_metadata(meta)
    }
}

#[cfg(not(windows))]
fn kind_from_metadata(meta: &Metadata) -> Option<LinkKind> {
    if meta.file_type().is_symlink() {
        Some(LinkKind::Symlink)
    } else {
        None
    }
}

#[cfg(windows)]
fn windows_kind_from_path_and_metadata(path: &Path, meta: &Metadata) -> Option<LinkKind> {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

    let attrs = meta.file_attributes();
    if (attrs & FILE_ATTRIBUTE_REPARSE_POINT) == 0 {
        return None;
    }

    match reparse_tag_from_path(path) {
        Some(IO_REPARSE_TAG_SYMLINK) => Some(LinkKind::Symlink),
        Some(IO_REPARSE_TAG_MOUNT_POINT) if (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0 => {
            Some(LinkKind::Junction)
        }
        _ => None,
    }
}

#[cfg(windows)]
use windows::Win32::System::SystemServices::{IO_REPARSE_TAG_MOUNT_POINT, IO_REPARSE_TAG_SYMLINK};

#[cfg(windows)]
fn reparse_tag_from_path(path: &Path) -> Option<u32> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, MAXIMUM_REPARSE_DATA_BUFFER_SIZE, OPEN_EXISTING,
    };
    use windows::Win32::System::IO::DeviceIoControl;
    use windows::Win32::System::Ioctl::FSCTL_GET_REPARSE_POINT;
    use windows::core::PCWSTR;

    let wide_path = verbatim_wide_path(path)?;
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_path.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    }
    .ok()?;

    let mut buffer = vec![0u8; MAXIMUM_REPARSE_DATA_BUFFER_SIZE as usize];
    let mut bytes_returned = 0u32;
    let result = unsafe {
        DeviceIoControl(
            handle,
            FSCTL_GET_REPARSE_POINT,
            None,
            0,
            Some(buffer.as_mut_ptr().cast()),
            buffer.len() as u32,
            Some(&mut bytes_returned),
            None,
        )
    };
    let _ = unsafe { CloseHandle(handle) };

    result.ok()?;
    if bytes_returned < 4 {
        return None;
    }

    Some(u32::from_le_bytes(buffer[0..4].try_into().ok()?))
}

#[cfg(windows)]
fn verbatim_wide_path(path: &Path) -> Option<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().collect()
    }

    let absolute;
    let path = if path.is_absolute() {
        path
    } else {
        absolute = std::env::current_dir().ok()?.join(path);
        absolute.as_path()
    };

    let raw: Vec<u16> = path.as_os_str().encode_wide().collect();
    let verbatim = wide(r"\\?\");
    let nt_verbatim = wide(r"\??\");
    if raw.starts_with(&verbatim) || raw.starts_with(&nt_verbatim) {
        let mut out = raw;
        out.push(0);
        return Some(out);
    }

    let unc = wide(r"\\");
    let mut out = if raw.starts_with(&unc) {
        let mut prefixed = wide(r"\\?\UNC\");
        prefixed.extend_from_slice(&raw[unc.len()..]);
        prefixed
    } else {
        let mut prefixed = verbatim;
        prefixed.extend_from_slice(&raw);
        prefixed
    };
    out.push(0);
    Some(out)
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::kind_from_path_and_metadata;
    use crate::domain::model::LinkKind;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn existing_link_kind(path: &Path) -> Option<LinkKind> {
        let meta = fs::symlink_metadata(path).ok()?;
        kind_from_path_and_metadata(path, &meta)
    }

    fn create_junction(target: &Path, link: &Path) {
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .expect("run mklink");
        assert!(status.success(), "mklink /J should succeed");
    }

    #[test]
    fn detects_directory_symlink_as_symlink() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target-dir");
        let link = dir.path().join("link-dir");
        fs::create_dir(&target).expect("target");
        std::os::windows::fs::symlink_dir(&target, &link).expect("symlink dir");

        assert_eq!(existing_link_kind(&link), Some(LinkKind::Symlink));
    }

    #[test]
    fn detects_junction_as_junction() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target-dir");
        let link = dir.path().join("junction");
        fs::create_dir(&target).expect("target");
        create_junction(&target, &link);

        assert_eq!(existing_link_kind(&link), Some(LinkKind::Junction));
    }

    #[test]
    fn detects_broken_junction_as_junction() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target-dir");
        let link = dir.path().join("junction");
        fs::create_dir(&target).expect("target");
        create_junction(&target, &link);
        fs::remove_dir(&target).expect("remove target");

        assert_eq!(existing_link_kind(&link), Some(LinkKind::Junction));
    }
}
