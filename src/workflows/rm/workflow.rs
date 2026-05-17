//! `rm`：删库后删除 link，或将 target 迁回 link 路径。支持多个 selector；省略参数时交互多选。
use crate::adapters::db::resolve;
use crate::adapters::db::{LinkQuery, repository};
use crate::adapters::migrate as migration;
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;
use crate::ui::interaction::{choice, pick_record};
use crate::ui::progress::migration_reporter::MigrationProgressReporter;
use crate::workflows::perf;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    selectors: &[String],
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let records = resolve_records(conn, selectors)?;
    let action = select_rm_action()?;
    let mut labels = Vec::with_capacity(records.len());
    for record in records {
        labels.push(remove_one(conn, &record, action, writer)?);
    }

    let summary = if labels.len() == 1 {
        labels[0].clone()
    } else {
        format!("共 {} 条：{}", labels.len(), labels.join("、"))
    };
    let action_hint = match action {
        RmAction::DeleteLinkOnly => "删除成功",
        RmAction::RestoreTargetToLink => "删除成功并已恢复 target 到 link",
    };
    writeln!(writer, "{action_hint}：{summary}").map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })?;

    perf::log_perf(
        "rm",
        started.elapsed(),
        &[
            ("count", labels.len().to_string()),
            ("action", format!("{action:?}")),
        ],
    );
    Ok(())
}

fn resolve_records(
    conn: &rusqlite::Connection,
    selectors: &[String],
) -> Result<Vec<LinkRecord>, SymmError> {
    if selectors.is_empty() {
        return pick_record::pick_many(conn);
    }

    let mut records = Vec::with_capacity(selectors.len());
    let mut seen_ids = HashSet::new();
    for selector in selectors {
        let record = resolve::record_from_token(conn, selector)?;
        if seen_ids.insert(record.id) {
            records.push(record);
        }
    }
    if records.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "未指定要删除的记录".to_string(),
        });
    }
    Ok(records)
}

fn remove_one<W: Write>(
    conn: &rusqlite::Connection,
    record: &LinkRecord,
    action: RmAction,
    writer: &mut W,
) -> Result<String, SymmError> {
    let link = Path::new(&record.link_path);
    let target = Path::new(&record.target_path);
    if action == RmAction::RestoreTargetToLink {
        restore_target_to_link(writer, link, target)?;
    }
    repository::delete_one(conn, &LinkQuery::id(record.id))?;
    if action == RmAction::DeleteLinkOnly {
        symlink::unlink(link)?;
    }
    Ok(record_label(record))
}

fn record_label(record: &LinkRecord) -> String {
    if !record.name.is_empty() {
        return record.name.clone();
    }
    format!("#{}", record.id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RmAction {
    DeleteLinkOnly,
    RestoreTargetToLink,
}

fn select_rm_action() -> Result<RmAction, SymmError> {
    choice::choose_with_env(
        "SYMM_RM_ACTION",
        parse_rm_action,
        "是否将 target 恢复到原 link 位置？",
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            (
                "否：仅删除软链接并删除数据库记录".to_string(),
                RmAction::DeleteLinkOnly,
            ),
            (
                "是：删除软链接并将 target 恢复到 link 位置".to_string(),
                RmAction::RestoreTargetToLink,
            ),
        ],
    )
}

fn parse_rm_action(raw: &str) -> Result<RmAction, SymmError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "no" | "n" | "delete" | "delete_only" => Ok(RmAction::DeleteLinkOnly),
        "yes" | "y" | "restore" | "restore_target" => Ok(RmAction::RestoreTargetToLink),
        _ => Err(SymmError::InvalidArgument {
            message: format!(
                "环境变量 SYMM_RM_ACTION 值无效：{raw}（可选：no/yes 或 delete/restore）"
            ),
        }),
    }
}

fn restore_target_to_link<W: Write>(
    writer: &mut W,
    link: &Path,
    target: &Path,
) -> Result<(), SymmError> {
    symlink::unlink(link)?;
    let mut reporter = MigrationProgressReporter::new(writer);
    migration::migrate_path(target, link, &mut |event| {
        reporter.handle_migration_event(event)
    })
    .map_err(|e| SymmError::IoError {
        message: format!("恢复 target 到 link 失败：{e}"),
    })
}
