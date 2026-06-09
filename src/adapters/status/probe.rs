use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkRecord, LinkStatus, LinkView};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusProbe {
    pub status: LinkStatus,
    pub status_error: Option<String>,
}

pub fn for_record(record: &LinkRecord) -> LinkStatus {
    probe_record(record).status
}

pub fn try_for_record(record: &LinkRecord) -> Result<LinkStatus, SymmError> {
    let link = Path::new(&record.link_path);
    let state = symlink::inspect_link_path(link)?;
    Ok(match state {
        symlink::LinkPathState::Missing => LinkStatus::Missing,
        symlink::LinkPathState::Entity => LinkStatus::Stale,
        symlink::LinkPathState::Link { kind } if kind != record.link_kind => LinkStatus::Stale,
        symlink::LinkPathState::Link { .. } => {
            let expected = Path::new(&record.target_path);
            if !symlink::link_points_to(link, expected)? {
                return Ok(LinkStatus::Drift);
            }
            if !crate::adapters::paths::presence::target_exists(expected)? {
                return Ok(LinkStatus::Broken);
            }
            LinkStatus::Ok
        }
    })
}

pub fn to_view(record: LinkRecord) -> LinkView {
    let probe = probe_record(&record);
    view_from_probe(record, 0, probe)
}

pub fn view_from_probe(record: LinkRecord, index: u32, probe: StatusProbe) -> LinkView {
    LinkView {
        record,
        index,
        status: probe.status,
        status_error: probe.status_error,
    }
}

pub fn probe_record(record: &LinkRecord) -> StatusProbe {
    match try_for_record(record) {
        Ok(status) => StatusProbe {
            status,
            status_error: None,
        },
        Err(err) => StatusProbe {
            status: LinkStatus::Unknown,
            status_error: Some(err.to_string()),
        },
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

    fn symlink_file(target: &Path, link: &Path) {
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, link).expect("symlink");

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(target, link).expect("symlink");
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
        symlink_file(&target, &link);
        let status = for_record(&record(&link.to_string_lossy(), &target.to_string_lossy()));
        assert_eq!(status, LinkStatus::Ok);
    }

    #[test]
    fn drift_when_symlink_points_elsewhere() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("target.txt");
        let other = dir.path().join("other.txt");
        let link = dir.path().join("link.txt");
        fs::write(&target, "x").expect("target");
        fs::write(&other, "y").expect("other");
        symlink_file(&other, &link);
        let status = for_record(&record(&link.to_string_lossy(), &target.to_string_lossy()));
        assert_eq!(status, LinkStatus::Drift);
    }

    #[test]
    fn drift_takes_priority_when_recorded_target_is_missing() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("missing-target.txt");
        let other = dir.path().join("other.txt");
        let link = dir.path().join("link.txt");
        fs::write(&other, "y").expect("other");
        symlink_file(&other, &link);
        let status = for_record(&record(&link.to_string_lossy(), &target.to_string_lossy()));
        assert_eq!(status, LinkStatus::Drift);
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
