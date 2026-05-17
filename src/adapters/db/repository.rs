use crate::adapters::paths::runtime_paths;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkKind, LinkRecord};
use rusqlite::{Connection, Error as SqlError, ErrorCode, params};
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
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            link_path TEXT NOT NULL UNIQUE,
            target_path TEXT NOT NULL,
            link_kind TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS ux_links_link_path ON links(link_path);
        CREATE UNIQUE INDEX IF NOT EXISTS ux_links_name_nonempty ON links(name) WHERE name <> '';",
    )
    .map_err(|e| SymmError::DbError {
        message: e.to_string(),
    })?;
    Ok(())
}

pub fn insert_link(
    conn: &Connection,
    name: &str,
    link_path: &str,
    target_path: &str,
    link_kind: LinkKind,
) -> Result<(), SymmError> {
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
        name,
        link_path,
        target_path,
        link_kind.to_string(),
        ts,
        ts
    ]);
    match res {
        Ok(_) => Ok(()),
        Err(e) => Err(map_sql_error(e, name)),
    }
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<LinkRecord, SymmError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, link_path, target_path, link_kind, created_at, updated_at
             FROM links WHERE id = ?1 LIMIT 1",
        )
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;

    stmt.query_row(params![id], map_link_row)
        .map_err(|e| match e {
            SqlError::QueryReturnedNoRows => SymmError::NotFound {
                selector: id.to_string(),
            },
            _ => SymmError::DbError {
                message: e.to_string(),
            },
        })
}

pub fn get_by_link_path(
    conn: &Connection,
    link_path: &str,
) -> Result<Option<LinkRecord>, SymmError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, link_path, target_path, link_kind, created_at, updated_at
             FROM links WHERE link_path = ?1 LIMIT 1",
        )
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;

    let rec = stmt.query_row(params![link_path], map_link_row);

    match rec {
        Ok(v) => Ok(Some(v)),
        Err(SqlError::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(SymmError::DbError {
            message: e.to_string(),
        }),
    }
}

pub fn delete_by_selector(conn: &Connection, selector: &str) -> Result<(), SymmError> {
    if let Ok(id) = selector.parse::<i64>() {
        let deleted = conn
            .execute("DELETE FROM links WHERE id = ?1", params![id])
            .map_err(|e| SymmError::DbError {
                message: e.to_string(),
            })?;
        if deleted > 0 {
            return Ok(());
        }
    }
    conn.execute(
        "DELETE FROM links WHERE name = ?1 OR link_path = ?1",
        params![selector],
    )
    .map_err(|e| SymmError::DbError {
        message: e.to_string(),
    })?;
    Ok(())
}

pub fn get_by_selector(conn: &Connection, selector: &str) -> Result<LinkRecord, SymmError> {
    if let Ok(id) = selector.parse::<i64>() {
        if let Ok(record) = get_by_id(conn, id) {
            return Ok(record);
        }
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, name, link_path, target_path, link_kind, created_at, updated_at
             FROM links WHERE name = ?1 OR link_path = ?1 LIMIT 1",
        )
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;

    stmt.query_row(params![selector], map_link_row)
        .map_err(|e| match e {
            SqlError::QueryReturnedNoRows => SymmError::NotFound {
                selector: selector.to_string(),
            },
            _ => SymmError::DbError {
                message: e.to_string(),
            },
        })
}

pub fn list_links(conn: &Connection) -> Result<Vec<LinkRecord>, SymmError> {
    list_links_paginated(conn, None, 0)
}

pub fn list_links_paginated(
    conn: &Connection,
    limit: Option<u32>,
    offset: u32,
) -> Result<Vec<LinkRecord>, SymmError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, link_path, target_path, link_kind, created_at, updated_at
             FROM links
             ORDER BY id ASC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;
    let limit = limit.unwrap_or(u32::MAX);

    let mapped = stmt
        .query_map(params![limit as i64, offset as i64], map_link_row)
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;

    mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_creates_links_table() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        insert_link(&conn, "demo", "/tmp/link", "/tmp/target", LinkKind::Symlink).expect("insert");
        let record = get_by_link_path(&conn, "/tmp/link")
            .expect("get")
            .expect("exists");
        assert_eq!(record.name, "demo");
        assert_eq!(record.id, 1);
    }
}
