use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use std::fs;
use std::fs::Metadata;
use std::path::Path;

pub fn unlink(link: &Path) -> Result<(), SymmError> {
    match fs::symlink_metadata(link) {
        Ok(meta) => {
            let Some(kind) = super::kind_from_path_and_metadata(link, &meta) else {
                return Ok(());
            };
            if should_remove_with_remove_dir(&meta, link, kind) {
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

#[cfg(windows)]
fn should_remove_with_remove_dir(meta: &Metadata, link: &Path, kind: LinkKind) -> bool {
    kind == LinkKind::Junction
        || is_directory_reparse_point(meta)
        || fs::metadata(link).is_ok_and(|m| m.is_dir())
}

#[cfg(not(windows))]
fn should_remove_with_remove_dir(_meta: &Metadata, _link: &Path, _kind: LinkKind) -> bool {
    false
}

#[cfg(windows)]
fn is_directory_reparse_point(meta: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    let attrs = meta.file_attributes();
    (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0 && (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0
}

#[cfg(test)]
#[cfg(unix)]
mod unix_tests {
    use super::unlink;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn unlink_removes_directory_symlink_without_removing_target() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target-dir");
        let link = dir.path().join("link-dir");
        fs::create_dir(&target).expect("target");
        fs::write(target.join("data.txt"), "x").expect("write target file");
        std::os::unix::fs::symlink(&target, &link).expect("symlink dir");

        unlink(&link).expect("unlink directory symlink");

        assert!(
            fs::symlink_metadata(&link).is_err(),
            "symlink path should be removed"
        );
        assert!(
            target.join("data.txt").exists(),
            "target contents should remain"
        );
    }
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::unlink;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

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
    fn unlink_removes_junction_without_removing_target() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target-dir");
        let link = dir.path().join("junction");
        fs::create_dir(&target).expect("target");
        fs::write(target.join("data.txt"), "x").expect("write target file");
        create_junction(&target, &link);

        unlink(&link).expect("unlink junction");

        assert!(
            fs::symlink_metadata(&link).is_err(),
            "junction path should be removed"
        );
        assert!(
            target.join("data.txt").exists(),
            "target contents should remain"
        );
    }

    #[test]
    fn unlink_removes_broken_junction() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target-dir");
        let link = dir.path().join("junction");
        fs::create_dir(&target).expect("target");
        create_junction(&target, &link);
        fs::remove_dir(&target).expect("remove target");

        unlink(&link).expect("unlink broken junction");

        assert!(
            fs::symlink_metadata(&link).is_err(),
            "broken junction path should be removed"
        );
    }
}
