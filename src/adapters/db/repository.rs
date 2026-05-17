use crate::adapters::db::link_query::{LinkQuery, ListOptions, StringMatch};
use crate::adapters::paths::runtime_paths;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkKind, LinkRecord, prepare_link_name_for_storage};
use rusqlite::{Connection, Error as SqlError, ErrorCode, ToSql, params};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn open_db() -> Result<Connection, SymmError> {
    let path = runtime_paths::db_path()?;
    let conn = Connection::open(path).map_err(|e| SymmError::DbError {
        message: e.to_string(),
    })?;
    tune_connection(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

fn tune_connection(conn: &Connection) -> Result<(), SymmError> {
    conn.execute_batch(
        "PRAGMA busy_timeout = 5000;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;",
    )
    .map_err(|e| SymmError::DbError {
        message: format!("数据库连接调优失败：{e}"),
    })?;
    Ok(())
}

fn migrate(conn: &Connection) -> Result<(), SymmError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS links (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL DEFAULT '',
            link_path TEXT NOT NULL UNIQUE,
            target_path TEXT NOT NULL,
            link_kind TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )
    .map_err(db_err)?;
    if links_table_needs_autoincrement_upgrade(conn)? {
        migrate_links_to_autoincrement(conn)?;
    } else {
        create_link_indexes(conn)?;
    }
    Ok(())
}

fn links_table_needs_autoincrement_upgrade(conn: &Connection) -> Result<bool, SymmError> {
    let sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'links'",
            [],
            |row| row.get(0),
        )
        .ok();
    Ok(sql.is_some_and(|ddl| !ddl.to_ascii_uppercase().contains("AUTOINCREMENT")))
}

fn migrate_links_to_autoincrement(conn: &Connection) -> Result<(), SymmError> {
    conn.execute_batch(
        "CREATE TABLE links__autoinc (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL DEFAULT '',
            link_path TEXT NOT NULL UNIQUE,
            target_path TEXT NOT NULL,
            link_kind TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        INSERT INTO links__autoinc(
            id, name, link_path, target_path, link_kind, created_at, updated_at
        )
        SELECT id, name, link_path, target_path, link_kind, created_at, updated_at
        FROM links;
        DROP TABLE links;
        ALTER TABLE links__autoinc RENAME TO links;",
    )
    .map_err(db_err)?;
    create_link_indexes(conn)
}

fn create_link_indexes(conn: &Connection) -> Result<(), SymmError> {
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS ux_links_link_path ON links(link_path);
         CREATE UNIQUE INDEX IF NOT EXISTS ux_links_name_nonempty ON links(name) WHERE name <> '';",
    )
    .map_err(db_err)?;
    Ok(())
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
        link_kind.to_string(),
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

struct BuiltQuery {
    sql: String,
    params: Vec<Box<dyn ToSql>>,
}

fn build_select(query: &LinkQuery, options: ListOptions) -> BuiltQuery {
    let mut clauses: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(id) = query.id {
        clauses.push("id = ?".to_string());
        params.push(Box::new(id));
    }
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
        StringMatch::Contains => {
            clauses.push(format!("{column} LIKE ?"));
            params.push(Box::new(format!("%{value}%")));
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
        1 => Ok(rows.into_iter().next().expect("len checked")),
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

pub fn delete_one(conn: &Connection, query: &LinkQuery) -> Result<LinkRecord, SymmError> {
    let record = find_one(conn, query)?;
    let deleted = conn
        .execute("DELETE FROM links WHERE id = ?1", params![record.id])
        .map_err(db_err)?;
    if deleted == 0 {
        return Err(SymmError::NotFound {
            selector: query.describe(),
        });
    }
    Ok(record)
}

pub fn list_links(conn: &Connection) -> Result<Vec<LinkRecord>, SymmError> {
    find_all(conn, &LinkQuery::default(), ListOptions::default())
}

pub fn list_links_paginated(
    conn: &Connection,
    limit: Option<u32>,
    offset: u32,
) -> Result<Vec<LinkRecord>, SymmError> {
    find_all(conn, &LinkQuery::default(), ListOptions { limit, offset })
}

fn map_link_row(row: &rusqlite::Row<'_>) -> Result<LinkRecord, SqlError> {
    let kind_str: String = row.get(4)?;
    Ok(LinkRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        link_path: row.get(2)?,
        target_path: row.get(3)?,
        link_kind: kind_str.parse().unwrap_or(LinkKind::Symlink),
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
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
    use crate::adapters::db::link_query::StringMatch;

    #[test]
    fn migrate_upgrades_legacy_table_without_autoincrement() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            "CREATE TABLE links (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL DEFAULT '',
                link_path TEXT NOT NULL UNIQUE,
                target_path TEXT NOT NULL,
                link_kind TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )
        .expect("legacy schema");
        conn.execute(
            "INSERT INTO links(id, name, link_path, target_path, link_kind, created_at, updated_at)
             VALUES(99, 'legacy', '/tmp/legacy', '/tmp/t', 'symlink', 1, 1)",
            [],
        )
        .expect("seed legacy row");
        migrate(&conn).expect("migrate");
        let record = find_one(&conn, &LinkQuery::link_path_exact("/tmp/legacy")).expect("by path");
        assert_eq!(record.id, 99);
    }

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
    fn find_by_name_contains() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "my-demo", "/tmp/a", "/tmp/t", LinkKind::Symlink).expect("insert");
        let hit = find_one(
            &conn,
            &LinkQuery {
                name: Some("demo".to_string()),
                name_match: StringMatch::Contains,
                ..LinkQuery::default()
            },
        )
        .expect("like");
        assert_eq!(hit.name, "my-demo");
    }

    #[test]
    fn list_index_resolves_second_row_after_delete() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "a", "/tmp/a", "/tmp/t1", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "b", "/tmp/b", "/tmp/t2", LinkKind::Symlink).expect("insert");
        insert_link(&conn, "c", "/tmp/c", "/tmp/t3", LinkKind::Symlink).expect("insert");
        delete_one(&conn, &LinkQuery::id(2)).expect("delete middle");

        let rows = list_links(&conn).expect("list");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].name, "c");
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
    fn insert_normalizes_pure_digit_name() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        let stored =
            insert_link(&conn, "42", "/tmp/link42", "/tmp/t", LinkKind::Symlink).expect("insert");
        assert_eq!(stored, "link-42");
        let r = find_one(&conn, &LinkQuery::name_exact("link-42")).expect("by name");
        assert_eq!(r.name, "link-42");
    }
}
