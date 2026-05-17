use crate::domain::error::SymmError;
use crate::domain::model::LinkStatus;
use crate::domain::model::LinkView;
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
    let views = list_views::collect_all(conn, wanted, limit, offset)?;
    let scanned = views.scanned;
    let emitted = views.items.len();

    if json {
        stream_json(&views.items, writer)?;
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

fn stream_json<W: Write>(items: &[LinkView], writer: &mut W) -> Result<(), SymmError> {
    output::write_json_array_start(writer)?;
    let mut first = true;
    for view in items {
        output::write_json_item(writer, view, first)?;
        first = false;
    }
    output::write_json_array_end(writer)
}
