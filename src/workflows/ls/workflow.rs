use crate::domain::error::SymmError;
use crate::domain::model::LinkStatus;
use crate::ui::output;
use crate::workflows::list_views;
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
        let stats = stream_json(conn, wanted, limit, offset, writer)?;
        (stats.scanned, stats.emitted)
    } else {
        let stats = stream_table(conn, wanted, limit, offset, writer)?;
        (stats.scanned, stats.emitted)
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

fn stream_table<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
    writer: &mut W,
) -> Result<list_views::ViewStreamStats, SymmError> {
    output::write_list_table_header(writer)?;
    list_views::for_each_view(conn, wanted, limit, offset, |view| {
        output::write_list_table_item(writer, &view)
    })
}

fn stream_json<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
    writer: &mut W,
) -> Result<list_views::ViewStreamStats, SymmError> {
    output::write_json_array_start(writer)?;
    let mut first = true;
    let stats = list_views::for_each_view(conn, wanted, limit, offset, |view| {
        output::write_json_item(writer, &view, first)?;
        first = false;
        Ok(())
    })?;
    output::write_json_array_end(writer)?;
    Ok(stats)
}
