use crate::adapters::db::{query::LinkQuery, repository};
use crate::domain::error::SymmError;
use crate::domain::model::{LinkKind, LinkRecord};
use rusqlite::Connection;
#[cfg(feature = "gui")]
use std::collections::HashSet;
#[cfg(feature = "gui")]
use std::path::Path;

pub fn open() -> Result<Connection, SymmError> {
    repository::open_db()
}

#[cfg(feature = "gui")]
pub fn open_at(data_dir: &Path) -> Result<Connection, SymmError> {
    repository::open_db_at(data_dir)
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

pub fn find_by_name_optional(
    conn: &Connection,
    name: &str,
) -> Result<Option<LinkRecord>, SymmError> {
    repository::find_optional(conn, &LinkQuery::name_exact(name))
}

pub fn find_by_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<LinkRecord>, SymmError> {
    repository::find_many_by_ids(conn, ids)
}

#[cfg(test)]
pub fn find_by_names(conn: &Connection, names: &[String]) -> Result<Vec<LinkRecord>, SymmError> {
    repository::find_many_by_names(conn, names)
}

pub fn find_existing_by_names(
    conn: &Connection,
    names: &[String],
) -> Result<Vec<LinkRecord>, SymmError> {
    repository::find_existing_by_names(conn, names)
}

#[cfg(feature = "gui")]
pub fn existing_ids(conn: &Connection, ids: &[i64]) -> Result<HashSet<i64>, SymmError> {
    repository::existing_ids(conn, ids)
}

pub fn delete_known_id(conn: &Connection, id: i64) -> Result<(), SymmError> {
    repository::delete_id(conn, id)
}

pub fn count(conn: &Connection) -> Result<usize, SymmError> {
    repository::count_links(conn)
}

#[cfg(feature = "gui")]
pub fn count_kinds(conn: &Connection) -> Result<(usize, usize), SymmError> {
    repository::count_link_kinds(conn)
}

#[cfg(feature = "gui")]
pub fn count_matching(conn: &Connection, search: &str) -> Result<usize, SymmError> {
    repository::count_links_matching(conn, search)
}

pub fn list_paginated(
    conn: &Connection,
    limit: Option<u32>,
    offset: u32,
) -> Result<Vec<LinkRecord>, SymmError> {
    repository::list_links_paginated(conn, limit, offset)
}

#[cfg(feature = "gui")]
pub fn list_matching_paginated(
    conn: &Connection,
    search: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<(u32, LinkRecord)>, SymmError> {
    repository::list_links_matching_paginated(conn, search, limit, offset)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::db::schema;

    #[test]
    fn find_by_ids_preserves_requested_order() {
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate");
        upsert_link(&conn, "a", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");
        upsert_link(&conn, "b", "/tmp/b", "/tmp/t2", LinkKind::Symlink).expect("insert");
        upsert_link(&conn, "c", "/tmp/c", "/tmp/t3", LinkKind::Symlink).expect("insert");

        let records = find_by_ids(&conn, &[3, 1]).expect("find many");

        assert_eq!(
            records.iter().map(|record| record.id).collect::<Vec<_>>(),
            vec![3, 1]
        );
    }

    #[test]
    fn find_by_ids_reports_missing_id() {
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate");
        upsert_link(&conn, "a", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");

        let err = find_by_ids(&conn, &[1, 99]).expect_err("missing id should fail");

        assert!(matches!(err, SymmError::NotFound { selector } if selector == "#99"));
    }

    #[test]
    fn find_by_names_preserves_requested_order_and_duplicates() {
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate");
        upsert_link(&conn, "a", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");
        upsert_link(&conn, "b", "/tmp/b", "/tmp/t2", LinkKind::Symlink).expect("insert");

        let names = vec!["b".to_string(), "a".to_string(), "b".to_string()];
        let records = find_by_names(&conn, &names).expect("find many");

        assert_eq!(
            records
                .iter()
                .map(|record| record.name.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "a", "b"]
        );
    }

    #[test]
    fn find_by_ids_handles_more_than_one_sql_parameter_chunk() {
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate");
        for n in 1..=905 {
            upsert_link(
                &conn,
                &format!("r{n}"),
                &format!("/tmp/link-{n}"),
                &format!("/tmp/target-{n}"),
                LinkKind::Symlink,
            )
            .expect("insert");
        }

        let ids = (1..=905).rev().collect::<Vec<_>>();
        let records = find_by_ids(&conn, &ids).expect("find many");

        assert_eq!(
            records.iter().map(|record| record.id).collect::<Vec<_>>(),
            ids
        );
    }
}
