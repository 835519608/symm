use crate::adapters::db::repository;
use crate::domain::error::SymmError;
use crate::ui::interaction::choice;
use std::collections::BTreeMap;
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RiskLevel {
    Low,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HighRiskDecision {
    ConfirmAndMarkManual,
    SkipKeepPending,
}

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
        "检测到 {} 条未完成操作，启动分级恢复扫描",
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
        let risk = classify_risk(latest.step);
        writeln!(
            writer,
            "- {operation_id} [{}] 风险={risk:?} 步骤={:?} 状态={:?} payload={} detail={}",
            latest.command, latest.step, latest.status, latest.payload, latest.detail
        )
        .map_err(io_err)?;
        match risk {
            RiskLevel::Low => {
                repository::mark_operation_failed(
                    conn,
                    &operation_id,
                    "自动恢复：已将低风险未完成操作标记为失败，请重试原命令",
                )?;
                writeln!(
                    writer,
                    "  -> 低风险自动处理：已标记为 failed，可直接重试命令"
                )
                .map_err(io_err)?;
            }
            RiskLevel::High => {
                let decision = select_high_risk_decision(&operation_id, &latest.command)?;
                match decision {
                    HighRiskDecision::ConfirmAndMarkManual => {
                        repository::mark_operation_failed(
                            conn,
                            &operation_id,
                            "高风险恢复已确认，当前版本仅标记为失败，请按提示人工恢复后重试",
                        )?;
                        writeln!(
                            writer,
                            "  -> 高风险已确认：已标记为 failed，请执行人工恢复步骤后重试"
                        )
                        .map_err(io_err)?;
                    }
                    HighRiskDecision::SkipKeepPending => {
                        writeln!(writer, "  -> 高风险已跳过：保留 pending，不做自动处理")
                            .map_err(io_err)?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn classify_risk(step: repository::OperationStep) -> RiskLevel {
    match step {
        repository::OperationStep::DbWrite | repository::OperationStep::Finalize => RiskLevel::Low,
        repository::OperationStep::Staging
        | repository::OperationStep::Migrate
        | repository::OperationStep::LinkChange => RiskLevel::High,
    }
}

fn select_high_risk_decision(
    operation_id: &str,
    command: &str,
) -> Result<HighRiskDecision, SymmError> {
    let prompt =
        format!("检测到高风险恢复操作：{operation_id}（command={command}）。请选择处理方式：");
    choice::choose_with_env(
        "SYMM_RECOVERY_HIGH_RISK",
        parse_high_risk_decision,
        &prompt,
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            (
                "确认并标记为需人工恢复（不自动执行高风险文件操作）".to_string(),
                HighRiskDecision::ConfirmAndMarkManual,
            ),
            (
                "跳过并保留 pending（稍后处理）".to_string(),
                HighRiskDecision::SkipKeepPending,
            ),
        ],
    )
}

fn parse_high_risk_decision(raw: &str) -> Result<HighRiskDecision, SymmError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "confirm" | "manual" | "mark_failed" => Ok(HighRiskDecision::ConfirmAndMarkManual),
        "skip" | "pending" | "keep_pending" => Ok(HighRiskDecision::SkipKeepPending),
        _ => Err(SymmError::InvalidArgument {
            message: format!(
                "环境变量 SYMM_RECOVERY_HIGH_RISK 值无效：{raw}（可选：confirm/skip）"
            ),
        }),
    }
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
            repository::OperationStep::DbWrite,
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

    #[test]
    fn recover_low_risk_marks_operation_failed() {
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
        let op = repository::begin_operation(&conn, "rm", r#"{"selector":"x"}"#).expect("begin");
        repository::advance_operation_step(
            &conn,
            &op,
            repository::OperationStep::DbWrite,
            repository::OperationStatus::Pending,
            "db pending",
        )
        .expect("advance");
        let mut out = Vec::new();
        recover_pending_operations(&conn, &mut out).expect("recover");
        let pending_after = repository::list_pending_operations(&conn).expect("pending");
        assert!(
            pending_after.iter().all(|record| record.operation_id != op),
            "low risk operation should be auto-resolved from pending"
        );
    }
}
