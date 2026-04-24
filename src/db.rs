use crate::error::SymmError;
use crate::model::{LinkKind, LinkRecord};
use rusqlite::{Connection, Error as SqlError, ErrorCode, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DB_FILE_NAME: &str = "symm.db";

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn symm_home() -> Result<PathBuf, SymmError> {
    if let Ok(v) = std::env::var("SYMM_HOME") {
        let p = PathBuf::from(v);
        fs::create_dir_all(&p).map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
        return Ok(p);
    }

    let home = dirs::home_dir().ok_or_else(|| SymmError::InvalidArgument {
        message: "Cannot resolve home directory".to_string(),
    })?;
    let p = home.join(".symm");
    fs::create_dir_all(&p).map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })?;
    Ok(p)
}

pub fn db_path() -> Result<PathBuf, SymmError> {
    Ok(symm_home()?.join(DB_FILE_NAME))
}

pub fn open_db() -> Result<Connection, SymmError> {
    let path = db_path()?;
    let conn = Connection::open(path).map_err(|e| SymmError::DbError {
        message: e.to_string(),
    })?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), SymmError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS links (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            link_path TEXT NOT NULL UNIQUE,
            target_path TEXT NOT NULL,
            link_kind TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )
    .map_err(|e| SymmError::DbError {
        message: e.to_string(),
    })?;
    Ok(())
}

fn canonicalish(path: &Path) -> String {
    match fs::canonicalize(path) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

pub fn normalize_target(path: &Path) -> Result<String, SymmError> {
    if !path.exists() {
        return Err(SymmError::TargetNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }
    Ok(canonicalish(path))
}

pub fn normalize_link(path: &Path) -> String {
    canonicalish(path)
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
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
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
        Err(e) => Err(map_sql_error(e, name, link_path)),
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

fn map_sql_error(err: SqlError, name: &str, link_path: &str) -> SymmError {
    match err {
        SqlError::SqliteFailure(e, _)
            if e.code == ErrorCode::ConstraintViolation
                || e.code == ErrorCode::ConstraintPrimaryKey
                || e.code == ErrorCode::ConstraintUnique =>
        {
            let conn = match open_db() {
                Ok(c) => c,
                Err(_) => {
                    return SymmError::DbError {
                        message: format!("constraint violation for name={name} link={link_path}"),
                    };
                }
            };
            if get_by_selector(&conn, name).is_ok() {
                return SymmError::NameConflict {
                    name: name.to_string(),
                };
            }
            if get_by_selector(&conn, link_path).is_ok() {
                return SymmError::PathConflict {
                    path: link_path.to_string(),
                };
            }
            SymmError::DbError {
                message: format!("constraint violation for name={name} link={link_path}"),
            }
        }
        _ => SymmError::DbError {
            message: err.to_string(),
        },
    }
}
