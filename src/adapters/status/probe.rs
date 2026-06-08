use crate::adapters::symlink;
use crate::domain::model::{LinkKind, LinkRecord, LinkStatus, LinkView};
use std::fs;
use std::fs::Metadata;
use std::path::Path;

pub fn for_record(record: &LinkRecord) -> LinkStatus {
    let link = Path::new(&record.link_path);
    let meta = match fs::symlink_metadata(link) {
        Err(_) => return LinkStatus::Missing,
        Ok(meta) => meta,
    };
    if !is_expected_link_kind(&meta, record.link_kind) {
        return LinkStatus::Stale;
    }
    let expected = Path::new(&record.target_path);
    if !expected.exists() {
        return LinkStatus::Broken;
    }
    if !symlink_target_matches(link, expected) {
        return LinkStatus::Drift;
    }
    LinkStatus::Ok
}

fn is_expected_link_kind(meta: &Metadata, kind: LinkKind) -> bool {
    symlink::kind_from_metadata(meta) == Some(kind)
}

pub fn to_view(record: LinkRecord) -> LinkView {
    let status = for_record(&record);
    LinkView {
        record,
        index: 0,
        status,
    }
}

fn symlink_target_matches(link: &Path, expected: &Path) -> bool {
    let Ok(actual) = fs::read_link(link) else {
        return false;
    };
    if actual == expected {
        return true;
    }
    let actual = if actual.is_absolute() {
        actual
    } else {
        link.parent().unwrap_or_else(|| Path::new("")).join(actual)
    };
    match (dunce::canonicalize(actual), dunce::canonicalize(expected)) {
        (Ok(a), Ok(e)) => a == e,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::LinkKind;
    use std::fs;
    use tempfile::tempdir;

    fn record(link_path: &str, target_path: &str) -> LinkRecord {
        record_with_kind(link_path, target_path, LinkKind::Symlink)
    }

    fn record_with_kind(link_path: &str, target_path: &str, link_kind: LinkKind) -> LinkRecord {
        LinkRecord {
            id: 1,
            name: "t".to_string(),
            link_path: link_path.to_string(),
            target_path: target_path.to_string(),
            link_kind,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[cfg(windows)]
    fn create_junction(target: &std::path::Path, link: &std::path::Path) {
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .expect("run mklink");
        assert!(status.success(), "mklink /J should succeed");
    }

    #[test]
    fn stale_when_path_exists_but_not_symlink() {
        let dir = tempdir().expect("tempdir");
        let link = dir.path().join("link.txt");
        let target = dir.path().join("target.txt");
        fs::write(&target, "x").expect("target");
        fs::write(&link, "plain file").expect("link");
        let status = for_record(&record(&link.to_string_lossy(), &target.to_string_lossy()));
        assert_eq!(status, LinkStatus::Stale);
    }

    #[test]
    fn ok_when_symlink_points_at_target() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        fs::write(&target, "x").expect("target");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, &link).expect("symlink");
        let status = for_record(&record(&link.to_string_lossy(), &target.to_string_lossy()));
        assert_eq!(status, LinkStatus::Ok);
    }

    #[cfg(windows)]
    #[test]
    fn ok_when_junction_points_at_target() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target-dir");
        let link = dir.path().join("junction");
        fs::create_dir(&target).expect("target");
        create_junction(&target, &link);

        let status = for_record(&record_with_kind(
            &link.to_string_lossy(),
            &target.to_string_lossy(),
            LinkKind::Junction,
        ));
        assert_eq!(status, LinkStatus::Ok);
    }

    #[cfg(windows)]
    #[test]
    fn broken_when_junction_target_is_missing() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target-dir");
        let link = dir.path().join("junction");
        fs::create_dir(&target).expect("target");
        create_junction(&target, &link);
        fs::remove_dir(&target).expect("remove target");

        let status = for_record(&record_with_kind(
            &link.to_string_lossy(),
            &target.to_string_lossy(),
            LinkKind::Junction,
        ));
        assert_eq!(status, LinkStatus::Broken);
    }

    #[cfg(windows)]
    #[test]
    fn stale_when_recorded_symlink_is_junction() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target-dir");
        let link = dir.path().join("junction");
        fs::create_dir(&target).expect("target");
        create_junction(&target, &link);

        let status = for_record(&record_with_kind(
            &link.to_string_lossy(),
            &target.to_string_lossy(),
            LinkKind::Symlink,
        ));
        assert_eq!(status, LinkStatus::Stale);
    }
}
