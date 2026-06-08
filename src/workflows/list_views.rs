//! `ls` / `show` 共用：从库记录构建带盘态的 [`LinkView`]。

use crate::adapters::db::link_store;
use crate::adapters::status;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkRecord, LinkStatus, LinkView};
use crate::workflows::selector;

#[cfg(feature = "gui")]
pub struct CollectedViews {
    pub items: Vec<LinkView>,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewStreamStats {
    pub scanned: usize,
    pub emitted: usize,
}

#[cfg(feature = "gui")]
pub fn collect_all(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
) -> Result<CollectedViews, SymmError> {
    if wanted.is_none() {
        let records = link_store::list_paginated(conn, limit, offset)?;
        let items: Vec<LinkView> = records
            .into_iter()
            .enumerate()
            .map(|(i, record)| {
                let mut view = status::to_view(record);
                view.index = offset + i as u32 + 1;
                view
            })
            .collect();
        return Ok(CollectedViews { items });
    }

    let Some(wanted) = wanted else {
        unreachable!("wanted was checked above");
    };
    collect_matching(conn, wanted, limit, offset)
}

#[cfg(feature = "gui")]
fn collect_matching(
    conn: &rusqlite::Connection,
    wanted: LinkStatus,
    limit: Option<u32>,
    offset: u32,
) -> Result<CollectedViews, SymmError> {
    let start = offset as usize;
    let take = limit.map(|lim| lim as usize).unwrap_or(usize::MAX);
    let mut scanned = 0u32;
    let mut matched = 0usize;
    let mut items = Vec::new();

    link_store::for_each(conn, |record| {
        scanned = scanned.saturating_add(1);
        let mut view = status::to_view(record);
        view.index = scanned;
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

    Ok(CollectedViews { items })
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
        link_store::for_each_paginated(conn, limit, offset, |record| {
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
    link_store::for_each(conn, |record| {
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
