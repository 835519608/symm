use crate::adapters::db::repository;
use crate::domain::error::SymmError;
use crate::ui::interaction::choice;
use serde::Deserialize;
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
                        let manual_steps =
                            build_manual_recovery_steps(&latest.command, &latest.payload);
                        write_manual_steps(writer, &manual_steps)?;
                        repository::mark_operation_failed(
                            conn,
                            &operation_id,
                            &format!("高风险恢复已确认，需人工恢复：{}", manual_steps.join("；")),
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

#[derive(Debug, Deserialize)]
struct AddPayload {
    link_path: String,
    target_path: String,
}

#[derive(Debug, Deserialize)]
struct RmPayload {
    selector: Option<String>,
    link_path: String,
    target_path: String,
}

fn build_manual_recovery_steps(command: &str, payload: &str) -> Vec<String> {
    match command {
        "add" => {
            if let Ok(add) = serde_json::from_str::<AddPayload>(payload) {
                return vec![
                    format!(
                        "检查 link 路径状态：确认 `{}` 是否存在且指向预期目标",
                        add.link_path
                    ),
                    format!(
                        "检查 target 路径状态：确认 `{}` 数据完整，必要时先备份",
                        add.target_path
                    ),
                    format!(
                        "状态确认后重试命令：symm add \"{}\" \"{}\"",
                        add.link_path, add.target_path
                    ),
                ];
            }
        }
        "rm" => {
            if let Ok(rm) = serde_json::from_str::<RmPayload>(payload) {
                let selector = rm.selector.unwrap_or(rm.link_path.clone());
                return vec![
                    format!(
                        "检查 link/target 状态：`{}` 与 `{}` 是否符合预期",
                        rm.link_path, rm.target_path
                    ),
                    format!(
                        "如需恢复 target 到 link，请先手工确认路径占用后执行：symm rm \"{}\" 并选择恢复",
                        selector
                    ),
                    format!(
                        "如仅删除记录与链接，执行：symm rm \"{}\" 并选择仅删除",
                        selector
                    ),
                ];
            }
        }
        _ => {}
    }
    vec![
        format!("无法解析 payload：{payload}"),
        "请先核对文件系统状态后，重试原命令。".to_string(),
    ]
}

fn write_manual_steps<W: Write>(writer: &mut W, steps: &[String]) -> Result<(), SymmError> {
    writeln!(writer, "  -> 人工恢复建议：").map_err(io_err)?;
    for (idx, step) in steps.iter().enumerate() {
        writeln!(writer, "     {}. {}", idx + 1, step).map_err(io_err)?;
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

    #[test]
    fn build_manual_recovery_steps_for_add_contains_retry_command() {
        let steps =
            build_manual_recovery_steps("add", r#"{"link_path":"C:\\a","target_path":"D:\\b"}"#);
        assert!(
            steps.iter().any(|s| s.contains("symm add")),
            "manual steps should include add retry command"
        );
    }

    #[test]
    fn build_manual_recovery_steps_for_rm_contains_two_branches() {
        let steps = build_manual_recovery_steps(
            "rm",
            r#"{"selector":"demo","link_path":"C:\\a","target_path":"D:\\b"}"#,
        );
        assert!(
            steps.iter().any(|s| s.contains("选择恢复")),
            "manual steps should include restore branch"
        );
        assert!(
            steps.iter().any(|s| s.contains("选择仅删除")),
            "manual steps should include delete-only branch"
        );
    }
}
