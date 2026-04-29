use crate::adapters::db::repository;
use crate::adapters::fs::link_status;
use crate::domain::error::SymmError;
use crate::domain::model::LinkStatus;
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
    let (scanned, emitted) = if json {
        stream_ls_json(conn, wanted, limit, offset, writer)?
    } else {
        stream_ls_table(conn, wanted, limit, offset, writer)?
    };
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

fn stream_ls_table<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
    writer: &mut W,
) -> Result<(usize, usize), SymmError> {
    let records = repository::list_links_paginated(conn, limit, offset)?;
    output::write_list_header(writer)?;
    let mut scanned = 0usize;
    let mut emitted = 0usize;
    for record in records {
        scanned += 1;
        let view = link_status::as_view(record);
        if wanted.is_none_or(|s| view.status == s) {
            output::write_list_row(writer, &view)?;
            emitted += 1;
        }
    }
    Ok((scanned, emitted))
}

fn stream_ls_json<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
    writer: &mut W,
) -> Result<(usize, usize), SymmError> {
    let records = repository::list_links_paginated(conn, limit, offset)?;
    output::write_json_array_start(writer)?;
    let mut first = true;
    let mut scanned = 0usize;
    let mut emitted = 0usize;
    for record in records {
        scanned += 1;
        let view = link_status::as_view(record);
        if wanted.is_none_or(|s| view.status == s) {
            output::write_json_item(writer, &view, first)?;
            first = false;
            emitted += 1;
        }
    }
    output::write_json_array_end(writer)?;
    Ok((scanned, emitted))
}
