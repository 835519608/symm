use crate::adapters::db::{query::LinkQuery, repository};
use crate::domain::error::SymmError;
use crate::domain::model::{LinkKind, LinkRecord};
use rusqlite::Connection;

pub fn open() -> Result<Connection, SymmError> {
    repository::open_db()
}

pub fn upsert_link(
    conn: &Connection,
    name: &str,
    link_path: &str,
    target_path: &str,
    link_kind: LinkKind,
) -> Result<String, SymmError> {
    repository::insert_link(conn, name, link_path, target_path, link_kind)
}

pub fn find_by_link_path(
    conn: &Connection,
    link_path: &str,
) -> Result<Option<LinkRecord>, SymmError> {
    repository::find_optional(conn, &LinkQuery::link_path_exact(link_path))
}

pub fn find_by_name(conn: &Connection, name: &str) -> Result<LinkRecord, SymmError> {
    repository::find_one(conn, &LinkQuery::name_exact(name))
}

pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<LinkRecord>, SymmError> {
    repository::find_optional(conn, &LinkQuery::id(id))
}

pub fn delete_by_id(conn: &Connection, id: i64) -> Result<LinkRecord, SymmError> {
    repository::delete_one(conn, &LinkQuery::id(id))
}

pub fn count(conn: &Connection) -> Result<usize, SymmError> {
    repository::count_links(conn)
}

pub fn list_paginated(
    conn: &Connection,
    limit: Option<u32>,
    offset: u32,
) -> Result<Vec<LinkRecord>, SymmError> {
    repository::list_links_paginated(conn, limit, offset)
}

pub fn index_for_id(conn: &Connection, id: i64) -> Result<Option<u32>, SymmError> {
    repository::list_index_for_id(conn, id)
}

pub fn for_each<F>(conn: &Connection, f: F) -> Result<(), SymmError>
where
    F: FnMut(LinkRecord) -> Result<bool, SymmError>,
{
    repository::for_each_link(conn, f)
}

pub fn for_each_paginated<F>(
    conn: &Connection,
    limit: Option<u32>,
    offset: u32,
    f: F,
) -> Result<(), SymmError>
where
    F: FnMut(LinkRecord) -> Result<bool, SymmError>,
{
    repository::for_each_link_paginated(conn, limit, offset, f)
}
