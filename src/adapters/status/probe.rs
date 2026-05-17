use crate::domain::model::{LinkRecord, LinkStatus, LinkView};
use std::fs;
use std::path::Path;

pub fn for_record(record: &LinkRecord) -> LinkStatus {
    let link = Path::new(&record.link_path);
    let meta = match fs::symlink_metadata(link) {
        Err(_) => return LinkStatus::Missing,
        Ok(meta) => meta,
    };
    if !meta.file_type().is_symlink() {
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
    match (dunce::canonicalize(&actual), dunce::canonicalize(expected)) {
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
        LinkRecord {
            id: 1,
            name: "t".to_string(),
            link_path: link_path.to_string(),
            target_path: target_path.to_string(),
            link_kind: LinkKind::Symlink,
            created_at: 0,
            updated_at: 0,
        }
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
}
