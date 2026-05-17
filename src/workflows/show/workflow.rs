use crate::adapters::db::resolve;
use crate::adapters::status;
use crate::domain::error::SymmError;
use crate::ui::interaction::pick_record;
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
    let record = resolve::record_from_token(conn, &selector)?;
    let mut view = status::to_view(record.clone());
    view.index = resolve::index_in_list(conn, &record)?;
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
    if let Some(selector) = selector.filter(|s| !s.is_empty()) {
        return Ok(selector.to_string());
    }
    pick_record::pick_one(conn)
}
