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
    match conn.execute_batch(
        "BEGIN IMMEDIATE;
         DROP TABLE IF EXISTS links__autoinc;
         CREATE TABLE links__autoinc (
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
         ALTER TABLE links__autoinc RENAME TO links;
         CREATE UNIQUE INDEX IF NOT EXISTS ux_links_link_path ON links(link_path);
         CREATE UNIQUE INDEX IF NOT EXISTS ux_links_name_nonempty ON links(name) WHERE name <> '';
         COMMIT;",
    ) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK; DROP TABLE IF EXISTS links__autoinc;");
            Err(db_err(err))
        }
    }
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

    #[test]
    fn autoincrement_upgrade_rolls_back_on_copy_failure() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            "CREATE TABLE links (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL DEFAULT '',
                link_path TEXT NOT NULL,
                target_path TEXT NOT NULL,
                link_kind TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            INSERT INTO links(id, name, link_path, target_path, link_kind, created_at, updated_at)
            VALUES
                (1, 'a', '/tmp/dup', '/tmp/t1', 'symlink', 1, 1),
                (2, 'b', '/tmp/dup', '/tmp/t2', 'symlink', 2, 2);",
        )
        .expect("legacy duplicate schema");

        migrate(&conn).expect_err("duplicate link_path should fail migration");

        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM links", [], |row| row.get(0))
            .expect("old links table should remain");
        assert_eq!(rows, 2);

        let temp_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'links__autoinc'",
                [],
                |row| row.get(0),
            )
            .expect("sqlite_master query");
        assert_eq!(temp_exists, 0);
    }
}
