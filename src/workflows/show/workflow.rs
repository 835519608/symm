use crate::adapters::db::selector;
use crate::adapters::fs::link_status;
use crate::domain::error::SymmError;
use crate::ui::interaction::record_picker;
use crate::ui::output;
use crate::workflows::perf;
use std::io::Write;
use std::time::Instant;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    selector: Option<&str>,
    json: bool,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let selector = resolve_selector(conn, selector)?;
    let record = selector::resolve_cli_record(conn, &selector)?;
    let mut view = link_status::as_view(record.clone());
    view.index = selector::list_index_for_record(conn, &record)?;
    if json {
        let text = output::render_json(&view)?;
        writeln!(writer, "{text}").map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    } else {
        writer
            .write_all(output::render_show_table(&view).as_bytes())
            .map_err(|e| SymmError::IoError {
                message: e.to_string(),
            })?;
    }
    perf::log_perf(
        "show",
        started.elapsed(),
        &[("selector", selector), ("json", json.to_string())],
    );
    Ok(())
}

fn resolve_selector(
    conn: &rusqlite::Connection,
    selector: Option<&str>,
) -> Result<String, SymmError> {
    if let Some(selector) = selector {
        if !selector.is_empty() {
            return Ok(selector.to_string());
        }
    }
    record_picker::pick_one(conn)
}
