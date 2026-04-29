use crate::adapters::db::repository;
use crate::adapters::fs::link_status;
use crate::domain::error::SymmError;
use crate::domain::model::LinkStatus;
use crate::ui::output;
use std::io::Write;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    json: bool,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
    writer: &mut W,
) -> Result<(), SymmError> {
    if json {
        stream_ls_json(conn, wanted, limit, offset, writer)
    } else {
        stream_ls_table(conn, wanted, limit, offset, writer)
    }
}

fn stream_ls_table<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
    writer: &mut W,
) -> Result<(), SymmError> {
    let records = repository::list_links_paginated(conn, limit, offset)?;
    output::write_list_header(writer)?;
    for record in records {
        let view = link_status::as_view(record);
        if wanted.is_none_or(|s| view.status == s) {
            output::write_list_row(writer, &view)?;
        }
    }
    Ok(())
}

fn stream_ls_json<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    limit: Option<u32>,
    offset: u32,
    writer: &mut W,
) -> Result<(), SymmError> {
    let records = repository::list_links_paginated(conn, limit, offset)?;
    output::write_json_array_start(writer)?;
    let mut first = true;
    for record in records {
        let view = link_status::as_view(record);
        if wanted.is_none_or(|s| view.status == s) {
            output::write_json_item(writer, &view, first)?;
            first = false;
        }
    }
    output::write_json_array_end(writer)
}
