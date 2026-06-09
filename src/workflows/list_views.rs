//! `ls` / `show` 共用：从库记录构建带盘态的 [`LinkView`]。

use crate::adapters::db::link_store;
use crate::adapters::status;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkRecord, LinkStatus, LinkView};
use crate::workflows::selector;

#[derive(Debug, Clone, Copy)]
pub struct ViewStreamStats {
    pub scanned: usize,
    pub emitted: usize,
    pub has_more: bool,
}

pub fn view_for_record(
    conn: &rusqlite::Connection,
    record: LinkRecord,
) -> Result<LinkView, SymmError> {
    let index = selector::index_in_list(conn, &record)?;
    let mut view = status::to_view(record);
    view.index = index;
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
        return Ok(ViewStreamStats {
            scanned,
            emitted,
            has_more: false,
        });
    }

    scan_status_filtered_views(
        conn,
        wanted.expect("wanted checked above"),
        limit,
        offset,
        false,
        f,
    )
}

pub fn for_each_view_page<F>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: u32,
    offset: u32,
    mut f: F,
) -> Result<ViewStreamStats, SymmError>
where
    F: FnMut(LinkView) -> Result<(), SymmError>,
{
    let take = limit.max(1) as usize;
    if wanted.is_none() {
        let mut seen = 0usize;
        let mut emitted = 0usize;
        let mut has_more = false;
        link_store::for_each_paginated(
            conn,
            Some(limit.max(1).saturating_add(1)),
            offset,
            |record| {
                let index = offset + seen as u32 + 1;
                seen += 1;
                if emitted >= take {
                    has_more = true;
                    return Ok(false);
                }
                let mut view = status::to_view(record);
                view.index = index;
                emitted += 1;
                f(view)?;
                Ok(true)
            },
        )?;
        return Ok(ViewStreamStats {
            scanned: offset as usize + seen,
            emitted,
            has_more,
        });
    }

    scan_status_filtered_views(
        conn,
        wanted.expect("wanted checked above"),
        Some(limit),
        offset,
        true,
        f,
    )
}

fn scan_status_filtered_views<F>(
    conn: &rusqlite::Connection,
    wanted: LinkStatus,
    limit: Option<u32>,
    offset: u32,
    detect_has_more: bool,
    mut f: F,
) -> Result<ViewStreamStats, SymmError>
where
    F: FnMut(LinkView) -> Result<(), SymmError>,
{
    let start = offset as usize;
    let take = limit.map(|lim| lim as usize).unwrap_or(usize::MAX);
    let mut scanned = 0usize;
    let mut matched = 0usize;
    let mut emitted = 0usize;
    let mut has_more = false;
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
            has_more = detect_has_more;
            return Ok(false);
        }
        matched += 1;
        emitted += 1;
        f(view)?;
        Ok(detect_has_more || emitted < take)
    })?;
    Ok(ViewStreamStats {
        scanned,
        emitted,
        has_more,
    })
}
