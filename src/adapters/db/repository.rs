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

fn now_ns() -> i128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i128)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStatus {
    Pending,
    Done,
    Failed,
}

impl OperationStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    fn parse(raw: &str) -> Result<Self, SymmError> {
        match raw {
            "pending" => Ok(Self::Pending),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            _ => Err(SymmError::DbError {
                message: format!("未知 operation status：{raw}"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationStep {
    Staging,
    Migrate,
    LinkChange,
    DbWrite,
    Finalize,
}

impl OperationStep {
    fn as_str(self) -> &'static str {
        match self {
            Self::Staging => "staging",
            Self::Migrate => "migrate",
            Self::LinkChange => "link_change",
            Self::DbWrite => "db_write",
            Self::Finalize => "finalize",
        }
    }

    fn parse(raw: &str) -> Result<Self, SymmError> {
        match raw {
            "staging" => Ok(Self::Staging),
            "migrate" => Ok(Self::Migrate),
            "link_change" => Ok(Self::LinkChange),
            "db_write" => Ok(Self::DbWrite),
            "finalize" => Ok(Self::Finalize),
            _ => Err(SymmError::DbError {
                message: format!("未知 operation step：{raw}"),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OperationRecord {
    pub operation_id: String,
    pub command: String,
    pub step: OperationStep,
    pub status: OperationStatus,
    pub payload: String,
    pub detail: String,
    pub updated_at: i64,
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
        CREATE UNIQUE INDEX IF NOT EXISTS ux_links_name_nonempty ON links(name) WHERE name <> '';

        CREATE TABLE IF NOT EXISTS operations (
            operation_id TEXT PRIMARY KEY,
            command TEXT NOT NULL,
            step TEXT NOT NULL,
            status TEXT NOT NULL,
            payload TEXT NOT NULL,
            detail TEXT NOT NULL DEFAULT '',
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_operations_status_updated_at
            ON operations(status, updated_at);",
    )
    .map_err(|e| SymmError::DbError {
        message: e.to_string(),
    })?;
    Ok(())
}

pub fn begin_operation(
    conn: &Connection,
    command: &str,
    payload: &str,
) -> Result<String, SymmError> {
    let operation_id = format!("op-{}-{command}", now_ns());
    conn.execute(
        "INSERT INTO operations(operation_id, command, step, status, payload, detail, updated_at)
         VALUES(?1, ?2, ?3, ?4, ?5, '', ?6)",
        params![
            operation_id,
            command,
            OperationStep::Staging.as_str(),
            OperationStatus::Pending.as_str(),
            payload,
            now_ts()
        ],
    )
    .map_err(|e| SymmError::DbError {
        message: format!("写入 operation 失败：{e}"),
    })?;
    Ok(operation_id)
}

pub fn advance_operation_step(
    conn: &Connection,
    operation_id: &str,
    step: OperationStep,
    status: OperationStatus,
    detail: &str,
) -> Result<(), SymmError> {
    conn.execute(
        "UPDATE operations
         SET step = ?2, status = ?3, detail = ?4, updated_at = ?5
         WHERE operation_id = ?1",
        params![
            operation_id,
            step.as_str(),
            status.as_str(),
            detail,
            now_ts()
        ],
    )
    .map_err(|e| SymmError::DbError {
        message: format!("更新 operation 失败：{e}"),
    })?;
    Ok(())
}

pub fn mark_operation_done(conn: &Connection, operation_id: &str) -> Result<(), SymmError> {
    advance_operation_step(
        conn,
        operation_id,
        OperationStep::Finalize,
        OperationStatus::Done,
        "",
    )
}

pub fn mark_operation_failed(
    conn: &Connection,
    operation_id: &str,
    detail: &str,
) -> Result<(), SymmError> {
    advance_operation_step(
        conn,
        operation_id,
        OperationStep::Finalize,
        OperationStatus::Failed,
        detail,
    )
}

pub fn list_pending_operations(conn: &Connection) -> Result<Vec<OperationRecord>, SymmError> {
    let mut stmt = conn
        .prepare(
            "SELECT operation_id, command, step, status, payload, detail, updated_at
             FROM operations
             WHERE status = 'pending'
             ORDER BY updated_at ASC",
        )
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;
    let mapped = stmt
        .query_map([], map_operation_row)
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })?;
    mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| SymmError::DbError {
            message: e.to_string(),
        })
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

    let rec = stmt.query_row(params![selector], map_link_row);

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
        .query_map([], map_link_row)
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
    let kind_str: String = row.get(3)?;
    Ok(LinkRecord {
        name: row.get(0)?,
        link_path: row.get(1)?,
        target_path: row.get(2)?,
        link_kind: kind_str.parse().unwrap_or(LinkKind::Symlink),
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn map_operation_row(row: &rusqlite::Row<'_>) -> Result<OperationRecord, SqlError> {
    let step: String = row.get(2)?;
    let status: String = row.get(3)?;
    Ok(OperationRecord {
        operation_id: row.get(0)?,
        command: row.get(1)?,
        step: OperationStep::parse(&step).map_err(to_sql_from_domain)?,
        status: OperationStatus::parse(&status).map_err(to_sql_from_domain)?,
        payload: row.get(4)?,
        detail: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn to_sql_from_domain(err: SymmError) -> SqlError {
    SqlError::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
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
    fn migrate_creates_operations_table() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        conn.execute(
            "INSERT INTO operations(operation_id, command, step, status, payload, detail, updated_at)
             VALUES('op-test', 'add', 'staging', 'pending', '{}', '', 1)",
            [],
        )
        .expect("insert operations row");
    }

    #[test]
    fn operation_status_flow_can_be_read_back() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        let operation_id = begin_operation(&conn, "add", r#"{"k":"v"}"#).expect("begin op");
        advance_operation_step(
            &conn,
            &operation_id,
            OperationStep::DbWrite,
            OperationStatus::Pending,
            "writing",
        )
        .expect("advance");
        let pending = list_pending_operations(&conn).expect("list pending");
        assert!(
            pending.iter().any(|record| {
                record.operation_id == operation_id
                    && record.step == OperationStep::DbWrite
                    && record.status == OperationStatus::Pending
            }),
            "expected operation in pending list with advanced step"
        );
        mark_operation_done(&conn, &operation_id).expect("mark done");
        let pending_after_done = list_pending_operations(&conn).expect("list pending after done");
        assert!(
            pending_after_done
                .iter()
                .all(|record| record.operation_id != operation_id),
            "done operation should not remain pending"
        );
    }
}
