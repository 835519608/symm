use crate::domain::model::LinkKind;
use std::fs::{self, Metadata};
use std::path::Path;

pub fn existing_link_kind(path: &Path) -> Option<LinkKind> {
    let meta = fs::symlink_metadata(path).ok()?;
    kind_from_metadata(&meta)
}

pub fn kind_from_metadata(meta: &Metadata) -> Option<LinkKind> {
    #[cfg(windows)]
    {
        windows_kind_from_metadata(meta)
    }
    #[cfg(not(windows))]
    {
        if meta.file_type().is_symlink() {
            Some(LinkKind::Symlink)
        } else {
            None
        }
    }
}

#[cfg(windows)]
fn windows_kind_from_metadata(meta: &Metadata) -> Option<LinkKind> {
    use std::os::windows::fs::{FileTypeExt, MetadataExt};

    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

    let file_type = meta.file_type();
    if file_type.is_symlink_dir() || file_type.is_symlink_file() {
        return Some(LinkKind::Symlink);
    }

    let attrs = meta.file_attributes();
    if (attrs & FILE_ATTRIBUTE_DIRECTORY) != 0 && (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        return Some(LinkKind::Junction);
    }

    None
}

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::existing_link_kind;
    use crate::domain::model::LinkKind;
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
