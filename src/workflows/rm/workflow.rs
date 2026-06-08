//! `rm`：删除 link 后删库，或将 target 迁回 link 路径。支持多个 selector；省略参数时交互多选。
use crate::adapters::db::link_store;
use crate::adapters::migrate;
use crate::adapters::status;
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkRecord, LinkStatus};
use crate::ui::progress::migration_reporter::MigrationProgressReporter;
use crate::workflows::perf;
use crate::workflows::select;
use crate::workflows::selector;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveMode {
    DeleteLinkOnly,
    RestoreTargetToLink,
}

impl RemoveMode {
    fn to_action(self) -> RmAction {
        match self {
            RemoveMode::DeleteLinkOnly => RmAction::DeleteLinkOnly,
            RemoveMode::RestoreTargetToLink => RmAction::RestoreTargetToLink,
        }
    }
}

/// GUI / 脚本：指定删除方式，不走交互选择。
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn run_with_mode<W: Write>(
    conn: &rusqlite::Connection,
    selectors: &[String],
    mode: RemoveMode,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let records = resolve_records(conn, selectors)?;
    run_resolved_records(conn, records, mode.to_action(), writer, started)
}

/// CLI：先解析 selector，再询问删除方式，避免非 TTY 脚本把 selector 错误误报成交互失败。
pub fn run_with_mode_picker<W: Write>(
    conn: &rusqlite::Connection,
    selectors: &[String],
    pick_mode: impl FnOnce() -> Result<RemoveMode, SymmError>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let records = resolve_records(conn, selectors)?;
    let action = pick_mode()?.to_action();
    run_resolved_records(conn, records, action, writer, started)
}

fn run_resolved_records<W: Write>(
    conn: &rusqlite::Connection,
    records: Vec<LinkRecord>,
    action: RmAction,
    writer: &mut W,
    started: Instant,
) -> Result<(), SymmError> {
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
            ("failures", failures.len().to_string()),
            ("action", format!("{action:?}")),
        ],
    );
    if !failures.is_empty() {
        return Err(SymmError::IoError {
            message: format!("部分删除失败：{}", failures.join("\n")),
        });
    }
    Ok(())
}

fn resolve_records(
    conn: &rusqlite::Connection,
    selectors: &[String],
) -> Result<Vec<LinkRecord>, SymmError> {
    if selectors.is_empty() {
        return select::pick_many_records(conn);
    }

    let records = selector::records_from_tokens(conn, selectors)?;
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
        && matches!(
            link_status,
            LinkStatus::Broken | LinkStatus::Stale | LinkStatus::Missing
        )
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
        RmAction::RestoreTargetToLink => {
            if let Err(err) = restore_target_to_link(writer, link, target) {
                match err {
                    RestoreFailure::LinkUnchanged(err) => return Err(err),
                    RestoreFailure::LinkRemoved(err) => {
                        if let Err(delete_err) = link_store::delete_by_id(conn, record.id) {
                            return Err(SymmError::IoError {
                                message: format!(
                                    "{err}；链接已删除，但数据库记录删除失败：{delete_err}"
                                ),
                            });
                        }
                        return Err(err);
                    }
                }
            }
        }
        RmAction::DeleteLinkOnly => apply_delete_link_only(writer, record, link, link_status)?,
    }

    link_store::delete_by_id(conn, record.id)?;
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

fn restore_target_to_link<W: Write>(
    writer: &mut W,
    link: &Path,
    target: &Path,
) -> Result<(), RestoreFailure> {
    symlink::unlink(link).map_err(RestoreFailure::LinkUnchanged)?;
    let mut reporter = MigrationProgressReporter::new(writer);
    migrate::migrate_path(target, link, &mut |event| {
        reporter.handle_migration_event(event)
    })
    .map_err(|e| {
        RestoreFailure::LinkRemoved(SymmError::IoError {
            message: format!("移回目标到链接位置失败：{e}"),
        })
    })
}

enum RestoreFailure {
    LinkUnchanged(SymmError),
    LinkRemoved(SymmError),
}
