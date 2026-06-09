//! SQLite 建表、索引与 schema 迁移（`open_db` 时调用）。

use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use rusqlite::{Connection, DatabaseName, Error as SqlError, Row, params};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CURRENT_LINKS_TABLE_SQL: &str = "links (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL DEFAULT '',
            link_path TEXT NOT NULL,
            target_path TEXT NOT NULL,
            link_kind TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )";

const CURRENT_COLUMNS: &[ColumnSpec] = &[
    ColumnSpec {
        name: "id",
        column_type: "INTEGER",
        not_null: false,
        default_value: None,
        primary_key: true,
    },
    ColumnSpec {
        name: "name",
        column_type: "TEXT",
        not_null: true,
        default_value: Some("''"),
        primary_key: false,
    },
    ColumnSpec {
        name: "link_path",
        column_type: "TEXT",
        not_null: true,
        default_value: None,
        primary_key: false,
    },
    ColumnSpec {
        name: "target_path",
        column_type: "TEXT",
        not_null: true,
        default_value: None,
        primary_key: false,
    },
    ColumnSpec {
        name: "link_kind",
        column_type: "TEXT",
        not_null: true,
        default_value: None,
        primary_key: false,
    },
    ColumnSpec {
        name: "created_at",
        column_type: "INTEGER",
        not_null: true,
        default_value: None,
        primary_key: false,
    },
    ColumnSpec {
        name: "updated_at",
        column_type: "INTEGER",
        not_null: true,
        default_value: None,
        primary_key: false,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ColumnSpec {
    name: &'static str,
    column_type: &'static str,
    not_null: bool,
    default_value: Option<&'static str>,
    primary_key: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct ColumnInfo {
    name: String,
    column_type: String,
    not_null: bool,
    default_value: Option<String>,
    primary_key: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinksSchemaState {
    Current,
    KnownLegacy { has_link_kind: bool },
}

struct LegacyLinkRow {
    id: i64,
    name: String,
    link_path: String,
    target_path: String,
    link_kind: Option<String>,
    created_at: i64,
    updated_at: i64,
}

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

#[cfg(test)]
pub fn migrate(conn: &Connection) -> Result<(), SymmError> {
    migrate_inner(conn, None)
}

pub fn migrate_file(conn: &Connection, db_path: &Path) -> Result<(), SymmError> {
    migrate_inner(conn, Some(db_path))
}

fn migrate_inner(conn: &Connection, db_path: Option<&Path>) -> Result<(), SymmError> {
    if !links_table_exists(conn)? {
        create_current_schema(conn)?;
        return Ok(());
    }

    match classify_links_schema(conn)? {
        LinksSchemaState::Current => Ok(()),
        LinksSchemaState::KnownLegacy { has_link_kind } => {
            rebuild_links_table(conn, has_link_kind, db_path)?;
            Ok(())
        }
    }?;
    create_current_indexes(conn)?;
    ensure_current_links_schema(conn)?;
    Ok(())
}

fn create_current_schema(conn: &Connection) -> Result<(), SymmError> {
    conn.execute_batch(&format!("CREATE TABLE {CURRENT_LINKS_TABLE_SQL};"))
        .map_err(db_err)?;
    create_current_indexes(conn)
}

fn create_current_indexes(conn: &Connection) -> Result<(), SymmError> {
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS ux_links_link_path ON links(link_path);
         CREATE UNIQUE INDEX IF NOT EXISTS ux_links_name_nonempty ON links(name) WHERE name <> '';",
    )
    .map_err(db_err)?;
    Ok(())
}

fn links_table_exists(conn: &Connection) -> Result<bool, SymmError> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'links')",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|exists| exists != 0)
    .map_err(db_err)
}

fn classify_links_schema(conn: &Connection) -> Result<LinksSchemaState, SymmError> {
    let sql = read_links_table_sql(conn)?;
    let columns = read_links_columns(conn)?;
    if is_current_links_schema(conn, &sql, &columns)? && link_kind_values_are_current(conn)? {
        return Ok(LinksSchemaState::Current);
    }

    if known_legacy_columns(&columns) {
        let has_link_kind = columns.iter().any(|column| column.name == "link_kind");
        if !has_link_kind || link_kind_values_are_migratable(conn)? {
            return Ok(LinksSchemaState::KnownLegacy { has_link_kind });
        }
    }

    Err(SymmError::DbError {
        message: "links 表不是已知 schema；无法自动迁移".to_string(),
    })
}

fn ensure_current_links_schema(conn: &Connection) -> Result<(), SymmError> {
    let sql = read_links_table_sql(conn)?;
    let columns = read_links_columns(conn)?;
    if is_current_links_schema(conn, &sql, &columns)? {
        ensure_current_link_kind_values(conn)?;
        return Ok(());
    }
    Err(SymmError::DbError {
        message: "links 表不是当前 schema；请使用新的数据目录重新初始化".to_string(),
    })
}

fn read_links_table_sql(conn: &Connection) -> Result<String, SymmError> {
    conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'links'",
        [],
        |row| row.get(0),
    )
    .map_err(db_err)
}

fn is_current_links_schema(
    conn: &Connection,
    sql: &str,
    columns: &[ColumnInfo],
) -> Result<bool, SymmError> {
    Ok(columns == expected_links_columns()
        && normalize_ddl(sql).contains("AUTOINCREMENT")
        && has_current_indexes(conn)?)
}

fn ensure_current_link_kind_values(conn: &Connection) -> Result<(), SymmError> {
    let invalid_count = conn
        .query_row(
            "SELECT COUNT(*) FROM links WHERE link_kind NOT IN ('symlink', 'junction')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(db_err)?;
    if invalid_count == 0 {
        return Ok(());
    }
    Err(SymmError::DbError {
        message: "links 表包含旧版 link_kind 值；请使用新的数据目录重新初始化".to_string(),
    })
}

fn link_kind_values_are_current(conn: &Connection) -> Result<bool, SymmError> {
    let invalid_count = conn
        .query_row(
            "SELECT COUNT(*) FROM links WHERE link_kind NOT IN ('symlink', 'junction')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(db_err)?;
    Ok(invalid_count == 0)
}

fn link_kind_values_are_migratable(conn: &Connection) -> Result<bool, SymmError> {
    let invalid_count = conn
        .query_row(
            "SELECT COUNT(*) FROM links
             WHERE link_kind NOT IN ('symlink', 'junction', '软链接', '目录联接')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(db_err)?;
    Ok(invalid_count == 0)
}

fn known_legacy_columns(columns: &[ColumnInfo]) -> bool {
    columns == expected_links_columns() || columns == expected_legacy_columns_without_link_kind()
}

fn backup_database_file(db_path: &Path) -> Result<(), SymmError> {
    let backup_path = next_backup_path(db_path)?;
    let backup_conn = Connection::open(db_path).map_err(|e| SymmError::IoError {
        message: format!("打开待备份数据库失败：{}：{e}", db_path.display()),
    })?;
    backup_conn
        .backup(DatabaseName::Main, &backup_path, None)
        .map_err(|e| SymmError::IoError {
            message: format!(
                "迁移前备份数据库失败：{} -> {}：{e}",
                db_path.display(),
                backup_path.display()
            ),
        })?;
    Ok(())
}

fn next_backup_path(db_path: &Path) -> Result<PathBuf, SymmError> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| SymmError::IoError {
            message: format!("生成数据库备份时间戳失败：{e}"),
        })?
        .as_secs();
    let base = db_path.with_file_name(format!(
        "{}.bak-{ts}",
        db_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("symm.db")
    ));
    if !base.exists() {
        return Ok(base);
    }
    for n in 1..1000u32 {
        let candidate = db_path.with_file_name(format!(
            "{}.bak-{ts}-{n}",
            db_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("symm.db")
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(SymmError::IoError {
        message: format!("无法为 {} 生成不冲突的备份文件名", db_path.display()),
    })
}

fn rebuild_links_table(
    conn: &Connection,
    has_link_kind: bool,
    db_path: Option<&Path>,
) -> Result<(), SymmError> {
    conn.execute_batch("BEGIN IMMEDIATE").map_err(db_err)?;
    let result = (|| {
        if let Some(path) = db_path {
            backup_database_file(path)?;
        }
        rebuild_links_table_inner(conn, has_link_kind)?;
        create_current_indexes(conn)?;
        ensure_current_links_schema(conn)
    })();
    match result {
        Ok(()) => conn.execute_batch("COMMIT").map_err(db_err),
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(err)
        }
    }
}

fn rebuild_links_table_inner(conn: &Connection, has_link_kind: bool) -> Result<(), SymmError> {
    conn.execute_batch(
        "DROP TABLE IF EXISTS links__symm_migration_new;
         CREATE TABLE links__symm_migration_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL DEFAULT '',
            link_path TEXT NOT NULL,
            target_path TEXT NOT NULL,
            link_kind TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )
    .map_err(db_err)?;

    stream_legacy_rows_into_new_table(conn, has_link_kind)?;

    conn.execute_batch(
        "DROP TABLE links;
         ALTER TABLE links__symm_migration_new RENAME TO links;",
    )
    .map_err(db_err)?;
    Ok(())
}

fn stream_legacy_rows_into_new_table(
    conn: &Connection,
    has_link_kind: bool,
) -> Result<(), SymmError> {
    let select_sql = if has_link_kind {
        "SELECT id, name, link_path, target_path, link_kind, created_at, updated_at FROM links"
    } else {
        "SELECT id, name, link_path, target_path, created_at, updated_at FROM links"
    };
    let mut select_stmt = conn.prepare(select_sql).map_err(db_err)?;
    let mut rows = select_stmt.query([]).map_err(db_err)?;
    {
        let mut insert_stmt = conn
            .prepare(
                "INSERT INTO links__symm_migration_new(
                    id, name, link_path, target_path, link_kind, created_at, updated_at
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(db_err)?;
        while let Some(row) = rows.next().map_err(db_err)? {
            let row = read_legacy_row(row, has_link_kind)?;
            let kind = migrate_link_kind(&row)?;
            insert_stmt
                .execute(params![
                    row.id,
                    row.name,
                    row.link_path,
                    row.target_path,
                    kind.as_db_str(),
                    row.created_at,
                    row.updated_at
                ])
                .map_err(db_err)?;
        }
    }
    Ok(())
}

fn read_legacy_row(row: &Row<'_>, has_link_kind: bool) -> Result<LegacyLinkRow, SymmError> {
    Ok(if has_link_kind {
        LegacyLinkRow {
            id: row.get(0).map_err(db_err)?,
            name: row.get(1).map_err(db_err)?,
            link_path: row.get(2).map_err(db_err)?,
            target_path: row.get(3).map_err(db_err)?,
            link_kind: Some(row.get(4).map_err(db_err)?),
            created_at: row.get(5).map_err(db_err)?,
            updated_at: row.get(6).map_err(db_err)?,
        }
    } else {
        LegacyLinkRow {
            id: row.get(0).map_err(db_err)?,
            name: row.get(1).map_err(db_err)?,
            link_path: row.get(2).map_err(db_err)?,
            target_path: row.get(3).map_err(db_err)?,
            link_kind: None,
            created_at: row.get(4).map_err(db_err)?,
            updated_at: row.get(5).map_err(db_err)?,
        }
    })
}

fn migrate_link_kind(row: &LegacyLinkRow) -> Result<LinkKind, SymmError> {
    match row.link_kind.as_deref() {
        Some("symlink") | Some("软链接") => Ok(LinkKind::Symlink),
        Some("junction") | Some("目录联接") => Ok(LinkKind::Junction),
        Some(value) => Err(SymmError::DbError {
            message: format!("links 表包含未知 link_kind 值：{value}"),
        }),
        None => Ok(infer_link_kind_for_legacy_row(&row.link_path)),
    }
}

fn infer_link_kind_for_legacy_row(link_path: &str) -> LinkKind {
    match symlink::inspect_link_path(Path::new(link_path)) {
        Ok(symlink::LinkPathState::Link { kind }) => kind,
        Ok(_) | Err(_) => LinkKind::Symlink,
    }
}

fn normalize_ddl(sql: &str) -> String {
    sql.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase()
}

fn read_links_columns(conn: &Connection) -> Result<Vec<ColumnInfo>, SymmError> {
    let mut stmt = conn.prepare("PRAGMA table_info(links)").map_err(db_err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ColumnInfo {
                name: row.get(1)?,
                column_type: row.get::<_, String>(2)?.to_ascii_uppercase(),
                not_null: row.get::<_, i64>(3)? != 0,
                default_value: row.get(4)?,
                primary_key: row.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(db_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
}

fn expected_links_columns() -> Vec<ColumnInfo> {
    CURRENT_COLUMNS
        .iter()
        .map(|column| ColumnInfo {
            name: column.name.to_string(),
            column_type: column.column_type.to_string(),
            not_null: column.not_null,
            default_value: column.default_value.map(str::to_string),
            primary_key: column.primary_key,
        })
        .collect()
}

fn expected_legacy_columns_without_link_kind() -> Vec<ColumnInfo> {
    CURRENT_COLUMNS
        .iter()
        .filter(|column| column.name != "link_kind")
        .map(|column| ColumnInfo {
            name: column.name.to_string(),
            column_type: column.column_type.to_string(),
            not_null: column.not_null,
            default_value: column.default_value.map(str::to_string),
            primary_key: column.primary_key,
        })
        .collect()
}

fn has_current_indexes(conn: &Connection) -> Result<bool, SymmError> {
    Ok(has_unique_index_for_column(conn, "link_path")?
        && has_named_index(conn, "ux_links_name_nonempty")?)
}

fn has_named_index(conn: &Connection, name: &str) -> Result<bool, SymmError> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1)",
        params![name],
        |row| row.get::<_, i64>(0),
    )
    .map(|exists| exists != 0)
    .map_err(db_err)
}

fn has_unique_index_for_column(conn: &Connection, column_name: &str) -> Result<bool, SymmError> {
    let mut stmt = conn.prepare("PRAGMA index_list(links)").map_err(db_err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(2)? != 0))
        })
        .map_err(db_err)?;
    for row in rows {
        let (index_name, unique) = row.map_err(db_err)?;
        if !unique {
            continue;
        }
        let columns = index_columns(conn, &index_name)?;
        if columns.len() == 1 && columns[0] == column_name {
            return Ok(true);
        }
    }
    Ok(false)
}

fn index_columns(conn: &Connection, index_name: &str) -> Result<Vec<String>, SymmError> {
    let mut stmt = conn
        .prepare(&format!(
            "PRAGMA index_info({})",
            quote_identifier(index_name)
        ))
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(2))
        .map_err(db_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(db_err)
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn db_err(e: SqlError) -> SymmError {
    SymmError::DbError {
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrate_creates_current_schema_and_indexes() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("migrate");
        let table_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'links'",
                [],
                |row| row.get(0),
            )
            .expect("table sql");
        assert!(table_sql.contains("AUTOINCREMENT"));
        assert!(
            !normalize_ddl(&table_sql).contains("LINK_PATH TEXT NOT NULL UNIQUE"),
            "link_path uniqueness should live in the named index only"
        );
        let index_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index'
                 AND name IN ('ux_links_link_path', 'ux_links_name_nonempty')",
                [],
                |row| row.get(0),
            )
            .expect("index count");
        assert_eq!(index_count, 2);
    }

    #[test]
    fn migrate_upgrades_old_schema_without_autoincrement() {
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
        .expect("old schema");
        conn.execute(
            "INSERT INTO links(name, link_path, target_path, link_kind, created_at, updated_at)
             VALUES('old', '/tmp/old-link', '/tmp/old-target', 'symlink', 1, 2)",
            [],
        )
        .expect("insert old row");

        migrate(&conn).expect("old schema should migrate");

        let table_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'links'",
                [],
                |row| row.get(0),
            )
            .expect("table sql");
        assert!(table_sql.contains("AUTOINCREMENT"));
        let row: (String, String, String) = conn
            .query_row(
                "SELECT name, link_path, link_kind FROM links WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("migrated row");
        assert_eq!(
            row,
            (
                "old".to_string(),
                "/tmp/old-link".to_string(),
                "symlink".to_string()
            )
        );
    }

    #[test]
    fn migrate_upgrades_old_schema_without_link_kind() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            "CREATE TABLE links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL DEFAULT '',
                link_path TEXT NOT NULL UNIQUE,
                target_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )
        .expect("old schema without link_kind");
        conn.execute(
            "INSERT INTO links(name, link_path, target_path, created_at, updated_at)
             VALUES('old', '/tmp/missing-kind-link', '/tmp/target', 1, 2)",
            [],
        )
        .expect("insert old row");

        migrate(&conn).expect("old schema should migrate");

        let kind: String = conn
            .query_row(
                "SELECT link_kind FROM links WHERE name = 'old'",
                [],
                |row| row.get(0),
            )
            .expect("link kind");
        assert_eq!(kind, "symlink");
    }

    #[test]
    fn migrate_accepts_autoincrement_table_without_inline_link_path_unique() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            "CREATE TABLE links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL DEFAULT '',
                link_path TEXT NOT NULL,
                target_path TEXT NOT NULL,
                link_kind TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )
        .expect("schema without inline unique");

        migrate(&conn).expect("schema without inline unique should migrate");
        conn.execute(
            "INSERT INTO links(name, link_path, target_path, link_kind, created_at, updated_at)
             VALUES('a', '/tmp/unique-link', '/tmp/t', 'symlink', 0, 0)",
            [],
        )
        .expect("first insert");
        conn.execute(
            "INSERT INTO links(name, link_path, target_path, link_kind, created_at, updated_at)
             VALUES('b', '/tmp/unique-link', '/tmp/t2', 'symlink', 0, 0)",
            [],
        )
        .expect_err("link_path should be unique after migrate");
    }

    #[test]
    fn migrate_converts_legacy_link_kind_values() {
        let conn = Connection::open_in_memory().expect("open memory db");
        migrate(&conn).expect("create current schema");
        conn.execute(
            "INSERT INTO links(name, link_path, target_path, link_kind, created_at, updated_at)
             VALUES('legacy', '/tmp/legacy', '/tmp/t', '目录联接', 0, 0)",
            [],
        )
        .expect("insert legacy value");

        migrate(&conn).expect("legacy link_kind value should migrate");

        let kind: String = conn
            .query_row(
                "SELECT link_kind FROM links WHERE name = 'legacy'",
                [],
                |row| row.get(0),
            )
            .expect("link kind");
        assert_eq!(kind, "junction");
    }

    #[test]
    fn migrate_rejects_unknown_schema_with_same_table_name() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            "CREATE TABLE links (
                id TEXT PRIMARY KEY,
                name TEXT,
                link_path TEXT,
                target_path TEXT,
                link_kind TEXT,
                created_at TEXT,
                updated_at TEXT
            );",
        )
        .expect("unknown schema");

        let err = migrate(&conn).expect_err("unknown schema should be rejected");

        assert!(
            matches!(err, SymmError::DbError { message } if message.contains("不是已知 schema"))
        );
    }

    #[test]
    fn migration_failure_keeps_old_table_and_next_attempt_can_retry() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            "CREATE TABLE links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL DEFAULT '',
                link_path TEXT NOT NULL,
                target_path TEXT NOT NULL,
                link_kind TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            INSERT INTO links(name, link_path, target_path, link_kind, created_at, updated_at)
            VALUES('a', '/tmp/dup', '/tmp/t1', 'symlink', 0, 0);
            INSERT INTO links(name, link_path, target_path, link_kind, created_at, updated_at)
            VALUES('b', '/tmp/dup', '/tmp/t2', 'symlink', 0, 0);",
        )
        .expect("duplicate legacy rows");

        migrate(&conn).expect_err("duplicate link_path should fail migration");
        let table_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'links'",
                [],
                |row| row.get(0),
            )
            .expect("table sql");
        assert!(!table_sql.contains("UNIQUE"));

        conn.execute("DELETE FROM links WHERE name = 'b'", [])
            .expect("remove duplicate");
        migrate(&conn).expect("second migration should retry and succeed");
        let migrated_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'links'",
                [],
                |row| row.get(0),
            )
            .expect("migrated table sql");
        assert!(migrated_sql.contains("AUTOINCREMENT"));
    }

    #[test]
    fn migration_index_failure_rolls_back_old_table() {
        let conn = Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            "CREATE TABLE links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL DEFAULT '',
                link_path TEXT NOT NULL,
                target_path TEXT NOT NULL,
                link_kind TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            INSERT INTO links(name, link_path, target_path, link_kind, created_at, updated_at)
            VALUES('same', '/tmp/a', '/tmp/t1', 'symlink', 0, 0);
            INSERT INTO links(name, link_path, target_path, link_kind, created_at, updated_at)
            VALUES('same', '/tmp/b', '/tmp/t2', 'symlink', 0, 0);",
        )
        .expect("duplicate names");

        migrate(&conn).expect_err("duplicate nonempty names should fail current index creation");

        let table_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'links'",
                [],
                |row| row.get(0),
            )
            .expect("table sql");
        assert!(!table_sql.contains("UNIQUE"));
        let duplicate_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM links WHERE name = 'same'",
                [],
                |row| row.get(0),
            )
            .expect("duplicate count");
        assert_eq!(duplicate_count, 2);
    }

    #[test]
    fn migrate_file_backs_up_before_upgrading_old_schema() {
        let temp = tempfile::tempdir().expect("temp dir");
        let db_path = temp.path().join("symm.db");
        let conn = Connection::open(&db_path).expect("open db file");
        conn.execute_batch(
            "CREATE TABLE links (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL DEFAULT '',
                link_path TEXT NOT NULL UNIQUE,
                target_path TEXT NOT NULL,
                link_kind TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            INSERT INTO links(name, link_path, target_path, link_kind, created_at, updated_at)
            VALUES('old', '/tmp/old', '/tmp/t', '软链接', 0, 0);",
        )
        .expect("old schema");

        migrate_file(&conn, &db_path).expect("migrate file");

        let backups = std::fs::read_dir(temp.path())
            .expect("read temp dir")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("symm.db.bak-")
            })
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        let backup = Connection::open(&backups[0]).expect("open backup");
        let backup_table_sql: String = backup
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'links'",
                [],
                |row| row.get(0),
            )
            .expect("backup table sql");
        assert!(!backup_table_sql.contains("AUTOINCREMENT"));
        let backup_kind: String = backup
            .query_row(
                "SELECT link_kind FROM links WHERE name = 'old'",
                [],
                |row| row.get(0),
            )
            .expect("backup link kind");
        assert_eq!(backup_kind, "软链接");
        let kind: String = conn
            .query_row(
                "SELECT link_kind FROM links WHERE name = 'old'",
                [],
                |row| row.get(0),
            )
            .expect("link kind");
        assert_eq!(kind, "symlink");
    }
}
