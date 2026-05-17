//! `ls` / `show` 共用：从库记录构建带盘态的 [`LinkView`]。

use crate::adapters::db::{repository, resolve};
use crate::adapters::status;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkRecord, LinkStatus, LinkView};

pub struct CollectedViews {
    pub items: Vec<LinkView>,
    pub scanned: usize,
}

pub fn collect_all(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
) -> Result<CollectedViews, SymmError> {
    let records = repository::list_links(conn)?;
    Ok(collect_from_records(records, wanted, limit, offset))
}

pub fn collect_from_records(
    records: Vec<LinkRecord>,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
) -> CollectedViews {
    let scanned = records.len();
    let filtered: Vec<LinkView> = records
        .into_iter()
        .enumerate()
        .map(|(i, record)| {
            let mut view = status::to_view(record);
            view.index = i as u32 + 1;
            view
        })
        .filter(|view| wanted.is_none_or(|status| view.status == status))
        .collect();

    let start = offset as usize;
    let end = limit
        .map(|lim| start.saturating_add(lim as usize))
        .unwrap_or(filtered.len());
    let items = filtered
        .into_iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();

    CollectedViews { items, scanned }
}

pub fn view_for_record(
    conn: &rusqlite::Connection,
    record: LinkRecord,
) -> Result<LinkView, SymmError> {
    let mut view = status::to_view(record.clone());
    view.index = resolve::index_in_list(conn, &record)?;
    Ok(view)
}

pub fn view_from_selector(
    conn: &rusqlite::Connection,
    selector: &str,
) -> Result<LinkView, SymmError> {
    let record = resolve::record_from_token(conn, selector)?;
    view_for_record(conn, record)
}
