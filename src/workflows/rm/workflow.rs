//! `rm`：删库后删除 link，或将 target 迁回 link 路径。支持多个 selector；省略参数时交互多选。
use crate::adapters::db::resolve;
use crate::adapters::db::{LinkQuery, repository};
use crate::adapters::migrate;
use crate::adapters::status;
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkRecord, LinkStatus};
use crate::ui::interaction::choice;
use crate::ui::progress::migration_reporter::MigrationProgressReporter;
use crate::workflows::perf;
use crate::workflows::select;
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
    let mut failures = Vec::new();
    for record in records {
        match remove_one(conn, &record, action, writer) {
            Ok(label) => labels.push(label),
            Err(err) => failures.push(format!("{}：{err}", record_label(&record))),
        }
    }

    if labels.is_empty() {
        return Err(SymmError::IoError {
            message: failures.join("\n"),
        });
    }

    let action_hint = match action {
        RmAction::DeleteLinkOnly => "已删除",
        RmAction::RestoreTargetToLink if failures.is_empty() => "已删除，目标已移回链接位置",
        RmAction::RestoreTargetToLink => "已删除（部分无法移回目标，仅删了记录）",
    };
    let summary = if labels.len() == 1 {
        labels[0].clone()
    } else {
        format!("共 {} 条：{}", labels.len(), labels.join("、"))
    };
    writeln!(writer, "{action_hint}：{summary}").map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })?;
    for failure in &failures {
        writeln!(writer, "失败：{failure}").map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    }

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
        return select::pick_many_records(conn);
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
    let link_status = status::for_record(record);
    let mut action = action;

    if action == RmAction::RestoreTargetToLink
        && matches!(link_status, LinkStatus::Stale | LinkStatus::Missing)
    {
        writeln!(
            writer,
            "提示：{} 状态为 {}，无法移回目标，本条只删记录",
            record.link_path,
            link_status.label_zh()
        )
        .map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
        action = RmAction::DeleteLinkOnly;
    }

    match action {
        RmAction::RestoreTargetToLink => restore_target_to_link(writer, link, target)?,
        RmAction::DeleteLinkOnly => apply_delete_link_only(writer, record, link, link_status)?,
    }

    repository::delete_one(conn, &LinkQuery::id(record.id))?;
    Ok(record_label(record))
}

fn should_unlink_on_disk(status: LinkStatus) -> bool {
    matches!(
        status,
        LinkStatus::Ok | LinkStatus::Broken | LinkStatus::Drift
    )
}

fn apply_delete_link_only<W: Write>(
    writer: &mut W,
    record: &LinkRecord,
    link: &Path,
    link_status: LinkStatus,
) -> Result<(), SymmError> {
    if should_unlink_on_disk(link_status) {
        symlink::unlink(link)?;
    } else if link_status == LinkStatus::Stale {
        writeln!(
            writer,
            "提示：{} 已不是软链，只删记录（路径文件仍保留）",
            record.link_path
        )
        .map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    }
    Ok(())
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
        "是否把目标移回链接位置？",
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            ("只删软链和记录".to_string(), RmAction::DeleteLinkOnly),
            (
                "删软链，并把目标移回链接位置".to_string(),
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
                "环境变量 SYMM_RM_ACTION 无效：{raw}（可选：delete / restore 或 no / yes）"
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
    migrate::migrate_path(target, link, &mut |event| {
        reporter.handle_migration_event(event)
    })
    .map_err(|e| SymmError::IoError {
        message: format!("移回目标到链接位置失败：{e}"),
    })
}
