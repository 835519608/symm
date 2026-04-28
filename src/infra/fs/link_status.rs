use crate::domain::model::{LinkRecord, LinkStatus, LinkView};
use std::fs;
use std::path::Path;

pub fn status_for(record: &LinkRecord) -> LinkStatus {
    let link = Path::new(&record.link_path);
    if fs::symlink_metadata(link).is_err() {
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
