use crate::domain::error::SymmError;
use crate::domain::model::{LinkKind, LinkRecord, LinkStatus, LinkView};
use std::path::Path;

pub fn create_link(target: &Path, link: &Path) -> Result<LinkKind, SymmError> {
    super::link_creator::create_link(target, link)
}

pub fn remove_link(link: &Path) -> Result<(), SymmError> {
    super::link_remover::remove_link(link)
}

pub fn status_for(record: &LinkRecord) -> LinkStatus {
    super::link_status::status_for(record)
}

pub fn as_view(record: LinkRecord) -> LinkView {
    super::link_status::as_view(record)
}
