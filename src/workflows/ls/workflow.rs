use crate::adapters::db::repository;
use crate::adapters::fs::link_status;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkStatus, LinkView};
use crate::ui::output;
use crate::workflows::perf;
use std::io::Write;
use std::time::Instant;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    json: bool,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let records = repository::list_links(conn)?;
    let views = collect_views(records, wanted, limit, offset);
    let scanned = views.scanned;
    let emitted = views.items.len();

    if json {
        stream_ls_json(&views.items, writer)?;
    } else {
        output::write_list_table(writer, &views.items)?;
    }

    perf::log_perf(
        "ls",
        started.elapsed(),
        &[
            ("json", json.to_string()),
            (
                "status_filter",
                wanted
                    .map(|status| status.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ),
            (
                "limit",
                limit
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ),
            ("offset", offset.to_string()),
            ("scanned", scanned.to_string()),
            ("emitted", emitted.to_string()),
        ],
    );
    Ok(())
}

struct CollectedViews {
    items: Vec<LinkView>,
    scanned: usize,
}

fn collect_views(
    records: Vec<crate::domain::model::LinkRecord>,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
) -> CollectedViews {
    let scanned = records.len();
    let filtered: Vec<LinkView> = records
        .into_iter()
        .enumerate()
        .map(|(i, record)| {
            let mut view = link_status::as_view(record);
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

fn stream_ls_json<W: Write>(items: &[LinkView], writer: &mut W) -> Result<(), SymmError> {
    output::write_json_array_start(writer)?;
    let mut first = true;
    for view in items {
        output::write_json_item(writer, view, first)?;
        first = false;
    }
    output::write_json_array_end(writer)
}
