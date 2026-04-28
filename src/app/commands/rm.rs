use crate::domain::error::SymmError;
use crate::infra::db::repository;
use crate::infra::fs::link_ops;
use std::io::Write;
use std::path::Path;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    selector: &str,
    writer: &mut W,
) -> Result<(), SymmError> {
    let record = repository::get_by_selector(conn, selector)?;
    link_ops::remove_link(Path::new(&record.link_path))?;
    repository::delete_by_selector(conn, selector)?;
    writeln!(writer, "删除成功：{}", record.name).map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })?;
    Ok(())
}
