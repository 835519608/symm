//! `ls` / `show` 共用：从库记录构建带盘态的 [`LinkView`]。

use crate::adapters::db::repository;
use crate::adapters::status;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkRecord, LinkStatus, LinkView};
use crate::workflows::selector;

pub struct CollectedViews {
    pub items: Vec<LinkView>,
    pub scanned: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewStreamStats {
    pub scanned: usize,
    pub emitted: usize,
}

pub fn collect_all(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
) -> Result<CollectedViews, SymmError> {
    if wanted.is_none() {
        let records = repository::list_links_paginated(conn, limit, offset)?;
        let items: Vec<LinkView> = records
            .into_iter()
            .enumerate()
            .map(|(i, record)| {
                let mut view = status::to_view(record);
                view.index = offset + i as u32 + 1;
                view
            })
            .collect();
        let scanned = offset as usize + items.len();
        return Ok(CollectedViews { items, scanned });
    }

    let Some(wanted) = wanted else {
        unreachable!("wanted was checked above");
    };
    collect_matching(conn, wanted, limit, offset)
}

fn collect_matching(
    conn: &rusqlite::Connection,
    wanted: LinkStatus,
    limit: Option<u32>,
    offset: u32,
) -> Result<CollectedViews, SymmError> {
    let start = offset as usize;
    let take = limit.map(|lim| lim as usize).unwrap_or(usize::MAX);
    let mut scanned = 0usize;
    let mut matched = 0usize;
    let mut items = Vec::new();

    repository::for_each_link(conn, |record| {
        scanned += 1;
        let mut view = status::to_view(record);
        view.index = scanned as u32;
        if view.status != wanted {
            return Ok(true);
        }
        if matched < start {
            matched += 1;
            return Ok(true);
        }
        if items.len() >= take {
            return Ok(false);
        }
        matched += 1;
        items.push(view);
        Ok(items.len() < take)
    })?;

    Ok(CollectedViews { items, scanned })
}

pub fn view_for_record(
    conn: &rusqlite::Connection,
    record: LinkRecord,
) -> Result<LinkView, SymmError> {
    let mut view = status::to_view(record.clone());
    view.index = selector::index_in_list(conn, &record)?;
    Ok(view)
}

pub fn view_from_selector(
    conn: &rusqlite::Connection,
    selector: &str,
) -> Result<LinkView, SymmError> {
    let record = selector::record_from_token(conn, selector)?;
    view_for_record(conn, record)
}

pub fn for_each_view<F>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
    mut f: F,
) -> Result<ViewStreamStats, SymmError>
where
    F: FnMut(LinkView) -> Result<(), SymmError>,
{
    if wanted.is_none() {
        let mut emitted = 0usize;
        repository::for_each_link_paginated(conn, limit, offset, |record| {
            let mut view = status::to_view(record);
            view.index = offset + emitted as u32 + 1;
            emitted += 1;
            f(view)?;
            Ok(true)
        })?;
        let scanned = offset as usize + emitted;
        return Ok(ViewStreamStats { scanned, emitted });
    }

    let Some(wanted) = wanted else {
        unreachable!("wanted was checked above");
    };
    let start = offset as usize;
    let take = limit.map(|lim| lim as usize).unwrap_or(usize::MAX);
    let mut scanned = 0usize;
    let mut matched = 0usize;
    let mut emitted = 0usize;
    repository::for_each_link(conn, |record| {
        scanned += 1;
        let mut view = status::to_view(record);
        view.index = scanned as u32;
        if view.status != wanted {
            return Ok(true);
        }
        if matched < start {
            matched += 1;
            return Ok(true);
        }
        if emitted >= take {
            return Ok(false);
        }
        matched += 1;
        emitted += 1;
        f(view)?;
        Ok(emitted < take)
    })?;
    Ok(ViewStreamStats { scanned, emitted })
}
