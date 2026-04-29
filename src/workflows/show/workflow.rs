use crate::adapters::db::repository;
use crate::adapters::fs::link_status;
use crate::domain::error::SymmError;
use crate::ui::output;
use std::io::Write;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    selector: &str,
    json: bool,
    writer: &mut W,
) -> Result<(), SymmError> {
    let view = link_status::as_view(repository::get_by_selector(conn, selector)?);
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
    Ok(())
}
