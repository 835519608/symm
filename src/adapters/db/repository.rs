use crate::adapters::db::query::{LinkQuery, ListOptions, StringMatch};
use crate::adapters::db::schema;
use crate::adapters::paths::runtime_paths;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkKind, LinkRecord, prepare_link_name_for_storage};
use rusqlite::{
    Connection, Error as SqlError, ErrorCode, ToSql, params, params_from_iter, types::Type,
};
use std::collections::HashMap;
#[cfg(any(feature = "gui", test))]
use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn open_db() -> Result<Connection, SymmError> {
    let path = runtime_paths::db_path()?;
    open_db_file(&path)
}

#[cfg(feature = "gui")]
pub fn open_db_at(data_dir: &Path) -> Result<Connection, SymmError> {
    let path = if data_dir.as_os_str().is_empty() {
        runtime_paths::default_data_home()?
    } else {
        std::fs::create_dir_all(data_dir).map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
        data_dir.to_path_buf()
    }
    .join(runtime_paths::DB_FILE_NAME);
    open_db_file(&path)
}

fn open_db_file(path: &Path) -> Result<Connection, SymmError> {
    let conn = Connection::open(path).map_err(|e| SymmError::DbError {
        message: e.to_string(),
    })?;
    schema::tune_connection(&conn)?;
    schema::migrate_file(&conn, path)?;
    Ok(conn)
}

pub fn insert_link(
    conn: &Connection,
    name: &str,
    link_path: &str,
    target_path: &str,
    link_kind: LinkKind,
) -> Result<String, SymmError> {
    let prepared = prepare_link_name_for_storage(name);
    let ts = now_ts();
    let mut stmt = conn
        .prepare(
            "INSERT INTO links(name, link_path, target_path, link_kind, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(link_path) DO UPDATE SET
               name = excluded.name,
               target_path = excluded.target_path,
               link_kind = excluded.link_kind,
               updated_at = excluded.updated_at",
        )
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;

    let res = stmt.execute(params![
        prepared.stored.as_str(),
        link_path,
        target_path,
        link_kind.as_db_str(),
        ts,
        ts
    ]);
    match res {
        Ok(_) => Ok(prepared.stored),
        Err(e) => Err(map_sql_error(e, &prepared.stored)),
    }
}

const SELECT_ROW: &str =
    "SELECT id, name, link_path, target_path, link_kind, created_at, updated_at FROM links";
pub(super) const MAX_QUERY_PARAMS: usize = 900;

struct BuiltQuery {
    sql: String,
    params: Vec<Box<dyn ToSql>>,
}

fn build_select(query: &LinkQuery, options: ListOptions) -> BuiltQuery {
    let mut clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    push_string_predicate(
        &mut clauses,
        &mut params,
        "name",
        query.name.as_deref(),
        query.name_match,
    );
    push_string_predicate(
        &mut clauses,
        &mut params,
        "link_path",
        query.link_path.as_deref(),
        query.link_path_match,
    );
    push_string_predicate(
        &mut clauses,
        &mut params,
        "target_path",
        query.target_path.as_deref(),
        query.target_path_match,
    );

    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };

    let limit = options.limit.unwrap_or(u32::MAX);
    let sql = format!("{SELECT_ROW}{where_sql} ORDER BY id ASC LIMIT ? OFFSET ?",);
    params.push(Box::new(limit as i64));
    params.push(Box::new(options.offset as i64));
    BuiltQuery { sql, params }
}

fn push_string_predicate(
    clauses: &mut Vec<String>,
    params: &mut Vec<Box<dyn ToSql>>,
    column: &str,
    value: Option<&str>,
    mode: StringMatch,
) {
    let Some(value) = value else {
        return;
    };
    match mode {
        StringMatch::Exact => {
            clauses.push(format!("{column} = ?"));
            params.push(Box::new(value.to_string()));
        }
    }
}

fn query_params(built: &BuiltQuery) -> Vec<&dyn ToSql> {
    built.params.iter().map(|p| p.as_ref()).collect()
}

pub fn find_all(
    conn: &Connection,
    query: &LinkQuery,
    options: ListOptions,
) -> Result<Vec<LinkRecord>, SymmError> {
    let built = build_select(query, options);
    let mut stmt = conn.prepare(&built.sql).map_err(db_err)?;
    let mapped = stmt
        .query_map(query_params(&built).as_slice(), map_link_row)
        .map_err(db_err)?;
    mapped.collect::<Result<Vec<_>, _>>().map_err(db_err)
}

pub fn find_one(conn: &Connection, query: &LinkQuery) -> Result<LinkRecord, SymmError> {
    let rows = find_all(
        conn,
        query,
        ListOptions {
            limit: Some(2),
            offset: 0,
        },
    )?;
    match rows.len() {
        0 => Err(SymmError::NotFound {
            selector: query.describe(),
        }),
        1 => rows.into_iter().next().ok_or_else(|| SymmError::NotFound {
            selector: query.describe(),
        }),
        n => Err(SymmError::InvalidArgument {
            message: format!(
                "查询条件匹配到 {n} 条记录，请缩小范围：{}",
                query.describe()
            ),
        }),
    }
}

pub fn find_optional(
    conn: &Connection,
    query: &LinkQuery,
) -> Result<Option<LinkRecord>, SymmError> {
    Ok(find_all(
        conn,
        query,
        ListOptions {
            limit: Some(1),
            offset: 0,
        },
    )?
    .into_iter()
    .next())
}

pub fn find_many_by_ids(conn: &Connection, ids: &[i64]) -> Result<Vec<LinkRecord>, SymmError> {
    let mut ordered = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(MAX_QUERY_PARAMS) {
        let by_id: HashMap<i64, LinkRecord> = find_many_by_id_chunk(conn, chunk)?
            .into_iter()
            .map(|record| (record.id, record))
            .collect();
        for id in chunk {
            let record = by_id.get(id).cloned().ok_or_else(|| SymmError::NotFound {
                selector: format!("#{id}"),
            })?;
            ordered.push(record);
        }
    }
    Ok(ordered)
}

#[cfg(test)]
pub fn find_many_by_names(
    conn: &Connection,
    names: &[String],
) -> Result<Vec<LinkRecord>, SymmError> {
    let mut ordered = Vec::with_capacity(names.len());
    for chunk in names.chunks(MAX_QUERY_PARAMS) {
        let by_name: HashMap<String, LinkRecord> = find_many_by_name_chunk(conn, chunk)?
            .into_iter()
            .map(|record| (record.name.clone(), record))
            .collect();
        for name in chunk {
            let record = by_name
                .get(name)
                .cloned()
                .ok_or_else(|| SymmError::NotFound {
                    selector: name.clone(),
                })?;
            ordered.push(record);
        }
    }
    Ok(ordered)
}

pub fn find_existing_by_names(
    conn: &Connection,
    names: &[String],
) -> Result<Vec<LinkRecord>, SymmError> {
    let mut records = Vec::new();
    for chunk in names.chunks(MAX_QUERY_PARAMS) {
        records.extend(find_many_by_name_chunk(conn, chunk)?);
    }
    Ok(records)
}

#[cfg(any(feature = "gui", test))]
pub fn existing_ids(conn: &Connection, ids: &[i64]) -> Result<HashSet<i64>, SymmError> {
    let mut existing = HashSet::with_capacity(ids.len());
    for chunk in ids.chunks(MAX_QUERY_PARAMS) {
        for record in find_many_by_id_chunk(conn, chunk)? {
            existing.insert(record.id);
        }
    }
    Ok(existing)
}

pub(super) fn find_many_by_id_chunk(
    conn: &Connection,
    ids: &[i64],
) -> Result<Vec<LinkRecord>, SymmError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat_n("?", ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("{SELECT_ROW} WHERE id IN ({placeholders}) ORDER BY id ASC");
    let mut stmt = conn.prepare(&sql).map_err(db_err)?;
    let mapped = stmt
        .query_map(params_from_iter(ids.iter()), map_link_row)
        .map_err(db_err)?;
    mapped.collect::<Result<Vec<_>, _>>().map_err(db_err)
}

fn find_many_by_name_chunk(
    conn: &Connection,
    names: &[String],
) -> Result<Vec<LinkRecord>, SymmError> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat_n("?", names.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("{SELECT_ROW} WHERE name IN ({placeholders}) ORDER BY id ASC");
    let mut stmt = conn.prepare(&sql).map_err(db_err)?;
    let mapped = stmt
        .query_map(params_from_iter(names.iter()), map_link_row)
        .map_err(db_err)?;
    mapped.collect::<Result<Vec<_>, _>>().map_err(db_err)
}

pub fn delete_id(conn: &Connection, id: i64) -> Result<(), SymmError> {
    let deleted = conn
        .execute("DELETE FROM links WHERE id = ?1", params![id])
        .map_err(db_err)?;
    if deleted == 0 {
        return Err(SymmError::NotFound {
            selector: format!("#{id}"),
        });
    }
    Ok(())
}

pub fn for_each_link<F>(conn: &Connection, mut f: F) -> Result<(), SymmError>
where
    F: FnMut(LinkRecord) -> Result<bool, SymmError>,
{
    let mut stmt = conn
        .prepare(&format!("{SELECT_ROW} ORDER BY id ASC"))
        .map_err(db_err)?;
    let mut rows = stmt.query([]).map_err(db_err)?;
    while let Some(row) = rows.next().map_err(db_err)? {
        let record = map_link_row(row).map_err(db_err)?;
        if !f(record)? {
            break;
        }
    }
    Ok(())
}

pub fn for_each_link_paginated<F>(
    conn: &Connection,
    limit: Option<u32>,
    offset: u32,
    mut f: F,
) -> Result<(), SymmError>
where
    F: FnMut(LinkRecord) -> Result<bool, SymmError>,
{
    let limit = limit.unwrap_or(u32::MAX);
    let sql = format!("{SELECT_ROW} ORDER BY id ASC LIMIT ?1 OFFSET ?2");
    let mut stmt = conn.prepare(&sql).map_err(db_err)?;
    let mut rows = stmt
        .query(params![limit as i64, offset as i64])
        .map_err(db_err)?;
    while let Some(row) = rows.next().map_err(db_err)? {
        let record = map_link_row(row).map_err(db_err)?;
        if !f(record)? {
            break;
        }
    }
    Ok(())
}

pub fn count_links(conn: &Connection) -> Result<usize, SymmError> {
    conn.query_row("SELECT COUNT(*) FROM links", [], |row| row.get::<_, i64>(0))
        .map(|count| count.max(0) as usize)
        .map_err(db_err)
}

#[cfg(any(feature = "gui", test))]
pub fn count_link_kinds(conn: &Connection) -> Result<(usize, usize), SymmError> {
    let mut stmt = conn
        .prepare("SELECT link_kind, COUNT(*) FROM links GROUP BY link_kind")
        .map_err(db_err)?;
    let mut rows = stmt.query([]).map_err(db_err)?;
    let mut counts = (0usize, 0usize);
    while let Some(row) = rows.next().map_err(db_err)? {
        let kind: String = row.get(0).map_err(db_err)?;
        let count = row.get::<_, i64>(1).map_err(db_err)?.max(0) as usize;
        match LinkKind::from_db_str(&kind) {
            Some(LinkKind::Symlink) => counts.0 = count,
            Some(LinkKind::Junction) => counts.1 = count,
            None => {
                return Err(SymmError::DbError {
                    message: format!("未知链接类型：{kind}"),
                });
            }
        }
    }
    Ok(counts)
}

#[cfg(any(feature = "gui", test))]
pub fn count_links_matching(conn: &Connection, search: &str) -> Result<usize, SymmError> {
    let Some(pattern) = search_pattern(search) else {
        return count_links(conn);
    };
    conn.query_row(
        &format!("SELECT COUNT(*) FROM links WHERE {}", search_where_sql()),
        params![pattern],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count.max(0) as usize)
    .map_err(db_err)
}

pub fn list_links_paginated(
    conn: &Connection,
    limit: Option<u32>,
    offset: u32,
) -> Result<Vec<LinkRecord>, SymmError> {
    find_all(conn, &LinkQuery::default(), ListOptions { limit, offset })
}

#[cfg(any(feature = "gui", test))]
pub fn list_links_matching_paginated(
    conn: &Connection,
    search: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<(u32, LinkRecord)>, SymmError> {
    let limit = limit as i64;
    let offset = offset as i64;
    let Some(pattern) = search_pattern(search) else {
        let rows = find_all(
            conn,
            &LinkQuery::default(),
            ListOptions {
                limit: Some(limit as u32),
                offset: offset as u32,
            },
        )?;
        return Ok(rows
            .into_iter()
            .enumerate()
            .map(|(i, record)| (offset as u32 + i as u32 + 1, record))
            .collect());
    };

    let sql = format!(
        "{SELECT_ROW}
         WHERE {}
         ORDER BY id ASC LIMIT ?2 OFFSET ?3",
        search_where_sql()
    );
    let params: Vec<Box<dyn ToSql>> = vec![Box::new(pattern), Box::new(limit), Box::new(offset)];
    let param_refs = params
        .iter()
        .map(|param| param.as_ref())
        .collect::<Vec<_>>();
    let mut stmt = conn.prepare(&sql).map_err(db_err)?;
    let mapped = stmt
        .query_map(param_refs.as_slice(), map_link_row)
        .map_err(db_err)?;
    mapped
        .enumerate()
        .map(|(i, row)| Ok((offset as u32 + i as u32 + 1, row?)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_err)
}

pub fn list_index_for_id(conn: &Connection, id: i64) -> Result<Option<u32>, SymmError> {
    let exists = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM links WHERE id = ?1)",
            params![id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(db_err)?
        != 0;
    if !exists {
        return Ok(None);
    }

    let count = conn
        .query_row(
            "SELECT COUNT(*) FROM links WHERE id <= ?1",
            params![id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(db_err)?;
    Ok(Some(count as u32))
}

fn map_link_row(row: &rusqlite::Row<'_>) -> Result<LinkRecord, SqlError> {
    let kind_str: String = row.get(4)?;
    let link_kind = LinkKind::from_db_str(&kind_str).ok_or_else(|| {
        SqlError::FromSqlConversionFailure(
            4,
            Type::Text,
            format!("未知链接类型：{kind_str}").into(),
        )
    })?;
    Ok(LinkRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        link_path: row.get(2)?,
        target_path: row.get(3)?,
        link_kind,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

#[cfg(any(feature = "gui", test))]
fn search_pattern(search: &str) -> Option<String> {
    let query = search.trim().to_lowercase();
    if query.is_empty() {
        None
    } else {
        Some(format!("%{}%", escape_like(&query)))
    }
}

#[cfg(any(feature = "gui", test))]
fn search_where_sql() -> &'static str {
    "(name <> '' AND lower(name) LIKE ?1 ESCAPE '\\')
     OR (name = '' AND lower(link_path) LIKE ?1 ESCAPE '\\')"
}

#[cfg(any(feature = "gui", test))]
fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' | '%' | '_' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn map_sql_error(err: SqlError, name: &str) -> SymmError {
    match err {
        SqlError::SqliteFailure(e, msg) if e.code == ErrorCode::ConstraintViolation => {
            let msg = msg.unwrap_or_default();
            if msg.contains("links.name") || msg.contains("ux_links_name_nonempty") {
                return SymmError::NameConflict {
                    name: name.to_string(),
                };
            }
            SymmError::DbError {
                message: format!("唯一约束冲突：name={name}"),
            }
        }
        _ => SymmError::DbError {
            message: err.to_string(),
        },
    }
}

fn db_err(e: SqlError) -> SymmError {
    SymmError::DbError {
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::db::schema::migrate;

    #[test]
    fn find_by_name_and_link_path_combined() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "demo", "/tmp/a", "/tmp/t", LinkKind::Symlink).expect("insert");
        let hit = find_one(
            &conn,
            &LinkQuery {
                name: Some("demo".to_string()),
                link_path: Some("/tmp/a".to_string()),
                ..LinkQuery::default()
            },
        )
        .expect("and query");
        assert_eq!(hit.name, "demo");
    }

    #[test]
    fn list_index_resolves_second_row_after_delete() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "a", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "b", "/tmp/b", "/tmp/t2", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "c", "/tmp/c", "/tmp/t3", LinkKind::Symlink).expect("insert");
        delete_id(&conn, 2).expect("delete middle");

        let rows = list_links_paginated(&conn, None, 0).expect("list");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].name, "c");
    }

    #[test]
    fn list_index_returns_none_for_missing_id() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "a", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "b", "/tmp/b", "/tmp/t2", LinkKind::Symlink).expect("insert");

        assert_eq!(list_index_for_id(&conn, 99).expect("index"), None);
    }

    #[test]
    fn upsert_same_link_path_keeps_id() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "v1", "/tmp/link", "/tmp/t1", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "v2", "/tmp/link", "/tmp/t2", LinkKind::Symlink).expect("insert");
        let record = find_one(&conn, &LinkQuery::link_path_exact("/tmp/link")).expect("get");
        assert_eq!(record.id, 1);
        assert_eq!(record.name, "v2");
    }

    #[test]
    fn find_many_by_ids_fetches_requested_records_in_one_query_shape() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "a", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "b", "/tmp/b", "/tmp/t2", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "c", "/tmp/c", "/tmp/t3", LinkKind::Symlink).expect("insert");

        let records = find_many_by_ids(&conn, &[3, 1]).expect("find many");
        assert_eq!(
            records.iter().map(|record| record.id).collect::<Vec<_>>(),
            vec![3, 1]
        );
    }

    #[test]
    fn find_many_by_ids_reports_missing_id() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "a", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");

        let err = find_many_by_ids(&conn, &[1, 99]).expect_err("missing id should fail");

        assert!(matches!(err, SymmError::NotFound { selector } if selector == "#99"));
    }

    #[test]
    fn delete_id_deletes_without_fetching_record() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "a", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");

        delete_id(&conn, 1).expect("delete");

        let err =
            find_one(&conn, &LinkQuery::link_path_exact("/tmp/a")).expect_err("deleted record");
        assert!(matches!(err, SymmError::NotFound { .. }));
    }

    #[test]
    fn list_links_matching_paginated_returns_page_and_match_index() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "alpha", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "zzz", "/tmp/zzz", "/tmp/t0", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "beta", "/tmp/b", "/tmp/t2", LinkKind::Junction).expect("insert");
        insert_link(&conn, "", "/tmp/gamma-link", "/tmp/t3", LinkKind::Symlink).expect("insert");

        let rows = list_links_matching_paginated(&conn, "a", 2, 1).expect("search matching page");

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, 2);
        assert_eq!(rows[0].1.name, "beta");
        assert_eq!(rows[1].0, 3);
        assert_eq!(rows[1].1.link_path, "/tmp/gamma-link");
    }

    #[test]
    fn counts_and_existing_ids_are_set_based() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "alpha", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "beta", "/tmp/b", "/tmp/t2", LinkKind::Junction).expect("insert");

        assert_eq!(count_links(&conn).expect("count"), 2);
        assert_eq!(count_link_kinds(&conn).expect("kind count"), (1, 1));
        assert_eq!(count_links_matching(&conn, "alp").expect("matching"), 1);
        assert_eq!(
            existing_ids(&conn, &[2, 99]).expect("existing ids"),
            HashSet::from([2])
        );
    }

    #[test]
    fn insert_normalizes_pure_digit_name() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        let stored =
            insert_link(&conn, "42", "/tmp/link42", "/tmp/t", LinkKind::Symlink).expect("insert");
        assert_eq!(stored, "link-42");
        let r = find_one(&conn, &LinkQuery::name_exact("link-42")).expect("by name");
        assert_eq!(r.name, "link-42");
    }

    #[test]
    fn junction_kind_round_trips_with_stable_db_value() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "j", "/tmp/j", "/tmp/t", LinkKind::Junction).expect("insert");

        let raw: String = conn
            .query_row("SELECT link_kind FROM links WHERE name = 'j'", [], |row| {
                row.get(0)
            })
            .expect("raw link kind");
        assert_eq!(raw, "junction");

        let record = find_one(&conn, &LinkQuery::link_path_exact("/tmp/j")).expect("get");
        assert_eq!(record.link_kind, LinkKind::Junction);
    }
}
