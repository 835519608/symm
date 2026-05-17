//! SQLite 建表、索引与 schema 迁移（`open_db` 时调用）。

use crate::domain::error::SymmError;
use rusqlite::{Connection, Error as SqlError};

pub(super) fn tune_connection(conn: &Connection) -> Result<(), SymmError> {
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

pub fn migrate(conn: &Connection) -> Result<(), SymmError> {
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

fn db_err(e: SqlError) -> SymmError {
    SymmError::DbError {
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::db::query::LinkQuery;
    use crate::adapters::db::repository;
    use rusqlite::Connection;

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
        let record = repository::find_one(&conn, &LinkQuery::link_path_exact("/tmp/legacy"))
            .expect("by path");
        assert_eq!(record.id, 99);
    }
}
