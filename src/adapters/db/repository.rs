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

pub fn get_by_link_path(
    conn: &Connection,
    link_path: &str,
) -> Result<Option<LinkRecord>, SymmError> {
    let mut stmt = conn
        .prepare(
            "SELECT name, link_path, target_path, link_kind, created_at, updated_at
             FROM links WHERE link_path = ?1 LIMIT 1",
        )
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;

    let rec = stmt.query_row(params![link_path], |row| {
        let kind_str: String = row.get(3)?;
        Ok(LinkRecord {
            name: row.get(0)?,
            link_path: row.get(1)?,
            target_path: row.get(2)?,
            link_kind: kind_str.parse().unwrap_or(LinkKind::Symlink),
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    });

    match rec {
        Ok(v) => Ok(Some(v)),
        Err(SqlError::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(SymmError::DbError {
            message: e.to_string(),
        }),
    }
}

pub fn delete_by_selector(conn: &Connection, selector: &str) -> Result<(), SymmError> {
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
    let mut stmt = conn
        .prepare(
            "SELECT name, link_path, target_path, link_kind, created_at, updated_at
             FROM links WHERE name = ?1 OR link_path = ?1 LIMIT 1",
        )
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;

    let rec = stmt.query_row(params![selector], |row| {
        let kind_str: String = row.get(3)?;
        Ok(LinkRecord {
            name: row.get(0)?,
            link_path: row.get(1)?,
            target_path: row.get(2)?,
            link_kind: kind_str.parse().unwrap_or(LinkKind::Symlink),
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    });

    rec.map_err(|e| match e {
        SqlError::QueryReturnedNoRows => SymmError::NotFound {
            selector: selector.to_string(),
        },
        _ => SymmError::DbError {
            message: e.to_string(),
        },
    })
}

pub fn list_links(conn: &Connection) -> Result<Vec<LinkRecord>, SymmError> {
    let mut stmt = conn
        .prepare(
            "SELECT name, link_path, target_path, link_kind, created_at, updated_at
             FROM links ORDER BY name ASC",
        )
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;

    let mapped = stmt
        .query_map([], |row| {
            let kind_str: String = row.get(3)?;
            Ok(LinkRecord {
                name: row.get(0)?,
                link_path: row.get(1)?,
                target_path: row.get(2)?,
                link_kind: kind_str.parse().unwrap_or(LinkKind::Symlink),
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;

    mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
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
