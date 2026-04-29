use crate::adapters::db::repository;
use crate::domain::error::SymmError;
use std::collections::BTreeMap;
use std::io::Write;

pub fn recover_pending_operations<W: Write>(
    conn: &rusqlite::Connection,
    writer: &mut W,
) -> Result<(), SymmError> {
    let pending = repository::list_pending_operations(conn)?;
    if pending.is_empty() {
        return Ok(());
    }

    writeln!(
        writer,
        "检测到 {} 条未完成操作，启动恢复扫描（当前为计划预览模式）",
        pending.len()
    )
    .map_err(io_err)?;

    let mut grouped: BTreeMap<String, Vec<repository::OperationRecord>> = BTreeMap::new();
    for op in pending {
        grouped.entry(op.operation_id.clone()).or_default().push(op);
    }

    for (operation_id, records) in grouped {
        let latest = records
            .iter()
            .max_by_key(|record| record.updated_at)
            .ok_or_else(|| SymmError::DbError {
                message: format!("operation 缺少记录：{operation_id}"),
            })?;
        writeln!(
            writer,
            "- {operation_id} [{}] 步骤={:?} 状态={:?} payload={} detail={}",
            latest.command, latest.step, latest.status, latest.payload, latest.detail
        )
        .map_err(io_err)?;
    }

    Ok(())
}

fn io_err(err: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::db::repository;

    #[test]
    fn recover_pending_operations_prints_plan() {
        let conn = rusqlite::Connection::open_in_memory().expect("open memory db");
        conn.execute_batch(
            "CREATE TABLE operations (
                operation_id TEXT PRIMARY KEY,
                command TEXT NOT NULL,
                step TEXT NOT NULL,
                status TEXT NOT NULL,
                payload TEXT NOT NULL,
                detail TEXT NOT NULL DEFAULT '',
                updated_at INTEGER NOT NULL
            );",
        )
        .expect("create operations");
        let op = repository::begin_operation(&conn, "add", r#"{"link":"a"}"#).expect("begin");
        repository::advance_operation_step(
            &conn,
            &op,
            repository::OperationStep::Migrate,
            repository::OperationStatus::Pending,
            "scan",
        )
        .expect("advance");

        let mut out = Vec::new();
        recover_pending_operations(&conn, &mut out).expect("recover scan");
        let text = String::from_utf8(out).expect("utf8");
        assert!(
            text.contains("未完成操作"),
            "should announce pending operations"
        );
        assert!(text.contains(&op), "should print operation id");
    }
}
