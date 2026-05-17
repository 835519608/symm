use crate::domain::error::SymmError;
use crate::ui::output;
use crate::workflows::list_views;
use crate::workflows::perf;
use crate::workflows::select;
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
    let view = list_views::view_from_selector(conn, &selector)?;
    if json {
        let text = output::render_json(&view)?;
        writeln!(writer, "{text}").map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    } else {
        writer
            .write_all(output::render_show_detail(&view).as_bytes())
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
    select::pick_one_selector(conn)
}
