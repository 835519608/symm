use crate::domain::error::SymmError;
use crate::domain::model::LinkStatus;
use crate::infra::db::repository;
use crate::infra::fs::link_ops;
use crate::interface::output;
use std::io::Write;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    json: bool,
    wanted: Option<LinkStatus>,
    writer: &mut W,
) -> Result<(), SymmError> {
    if json {
        stream_ls_json(conn, wanted, writer)
    } else {
        stream_ls_table(conn, wanted, writer)
    }
}

fn stream_ls_table<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let records = repository::list_links(conn)?;
    output::write_list_header(writer)?;
    for record in records {
        let view = link_ops::as_view(record);
        if wanted.is_none_or(|s| view.status == s) {
            output::write_list_row(writer, &view)?;
        }
    }
    Ok(())
}

fn stream_ls_json<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let records = repository::list_links(conn)?;
    output::write_json_array_start(writer)?;
    let mut first = true;
    for record in records {
        let view = link_ops::as_view(record);
        if wanted.is_none_or(|s| view.status == s) {
            output::write_json_item(writer, &view, first)?;
            first = false;
        }
    }
    output::write_json_array_end(writer)
}
