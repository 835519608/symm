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
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub enum RemoveMode {
    DeleteLinkOnly,
    RestoreTargetToLink,
}

/// CLI：删除链接关系，不移动 target。
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn run_rm<W: Write>(
    conn: &rusqlite::Connection,
    selectors: &[String],
    writer: &mut W,
) -> Result<(), SymmError> {
    run_remove(
        conn,
        RmSelection::Selectors(selectors),
        RemoveMode::DeleteLinkOnly,
        writer,
    )
}

/// CLI：恢复 target 到 link 路径，然后删除记录。
pub fn run_restore<W: Write>(
    conn: &rusqlite::Connection,
    selectors: &[String],
    writer: &mut W,
) -> Result<(), SymmError> {
    run_remove(
        conn,
        RmSelection::Selectors(selectors),
        RemoveMode::RestoreTargetToLink,
        writer,
    )
}

#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn run_rm_by_ids<W: Write>(
    conn: &rusqlite::Connection,
    ids: &[i64],
    writer: &mut W,
) -> Result<(), SymmError> {
    run_remove(
        conn,
        RmSelection::RecordIds(ids),
        RemoveMode::DeleteLinkOnly,
        writer,
    )
}

#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn run_restore_by_ids<W: Write>(
    conn: &rusqlite::Connection,
    ids: &[i64],
    writer: &mut W,
) -> Result<(), SymmError> {
    run_remove(
        conn,
        RmSelection::RecordIds(ids),
        RemoveMode::RestoreTargetToLink,
        writer,
    )
}

enum RmSelection<'a> {
    Selectors(&'a [String]),
    RecordIds(&'a [i64]),
}

fn run_remove<W: Write>(
    conn: &rusqlite::Connection,
    selection: RmSelection<'_>,
    mode: RemoveMode,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let records = match selection {
        RmSelection::Selectors(selectors) => resolve_records(conn, selectors)?,
        RmSelection::RecordIds(ids) => records_from_ids(conn, ids)?,
    };
    run_resolved_records(conn, records, mode, writer, started)
}

fn run_resolved_records<W: Write>(
    conn: &rusqlite::Connection,
    records: Vec<LinkRecord>,
    mode: RemoveMode,
    writer: &mut W,
    started: Instant,
) -> Result<(), SymmError> {
    let mut labels = Vec::with_capacity(records.len());
    let mut failures = Vec::new();
    for record in records {
        match remove_one(conn, &record, mode, writer) {
            Ok(label) => labels.push(label),
            Err(err) => failures.push((record_label(&record), err)),
        }
    }

    if labels.is_empty() {
        return Err(batch_failure_error(failures));
    }

    let action_hint = match mode {
        RemoveMode::DeleteLinkOnly => "已删除链接关系",
        RemoveMode::RestoreTargetToLink => "已恢复实体位置",
    };
    let summary = if labels.len() == 1 {
        labels[0].clone()
    } else {
        format!("共 {} 条：{}", labels.len(), labels.join("、"))
    };
    writeln!(writer, "{action_hint}：{summary}").map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })?;
    for (label, err) in &failures {
        writeln!(writer, "失败：{label}：{err}").map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    }

    perf::log_perf_lazy("rm", started.elapsed(), || {
        vec![
            ("count", labels.len().to_string()),
            ("failures", failures.len().to_string()),
            ("action", format!("{mode:?}")),
        ]
    });
    if !failures.is_empty() {
        return Err(SymmError::BatchFailure {
            message: format!("部分删除失败：{}", format_failures(&failures)),
        });
    }
    Ok(())
}

fn batch_failure_error(mut failures: Vec<(String, SymmError)>) -> SymmError {
    if failures.len() == 1 {
        return failures.remove(0).1;
    }
    SymmError::BatchFailure {
        message: format_failures(&failures),
    }
}

fn format_failures(failures: &[(String, SymmError)]) -> String {
    failures
        .iter()
        .map(|(label, err)| format!("{label}：{err}"))
        .collect::<Vec<_>>()
        .join("\n")
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

fn records_from_ids(
    conn: &rusqlite::Connection,
    ids: &[i64],
) -> Result<Vec<LinkRecord>, SymmError> {
    if ids.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "未指定要操作的记录".to_string(),
        });
    }
    link_store::find_by_ids(conn, ids)
}

fn remove_one<W: Write>(
    conn: &rusqlite::Connection,
    record: &LinkRecord,
    mode: RemoveMode,
    writer: &mut W,
) -> Result<String, SymmError> {
    let link = Path::new(&record.link_path);
    let target = Path::new(&record.target_path);
    let link_status = status::try_for_record(record)?;

    match mode {
        RemoveMode::RestoreTargetToLink => {
            ensure_restorable(record, link_status)?;
            if let Err(err) = restore_target_to_link(writer, link, target) {
                match err {
                    RestoreFailure::LinkUnchanged(err) => return Err(err),
                    RestoreFailure::LinkRemoved(err) => return Err(err),
                }
            }
        }
        RemoveMode::DeleteLinkOnly => apply_delete_link_only(writer, record, link, link_status)?,
    }

    link_store::delete_known_id(conn, record.id)?;
    Ok(record_label(record))
}

fn ensure_restorable(record: &LinkRecord, status: LinkStatus) -> Result<(), SymmError> {
    match status {
        LinkStatus::Ok | LinkStatus::Missing => Ok(()),
        LinkStatus::Broken => Err(SymmError::InvalidArgument {
            message: format!("target 不存在，无法 restore：{}", record.target_path),
        }),
        LinkStatus::Drift => Err(SymmError::InvalidArgument {
            message: format!(
                "link 路径已指向记录以外的位置，无法 restore：{}",
                record.link_path
            ),
        }),
        LinkStatus::Stale => Err(SymmError::InvalidArgument {
            message: format!(
                "link 路径已被非预期实体占用，无法 restore：{}",
                record.link_path
            ),
        }),
        LinkStatus::Unknown => Err(SymmError::InvalidArgument {
            message: format!("link 状态未知，无法 restore：{}", record.link_path),
        }),
    }
}

fn should_unlink_on_disk(status: LinkStatus) -> bool {
    matches!(status, LinkStatus::Ok | LinkStatus::Broken)
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
    } else if link_status == LinkStatus::Drift {
        writeln!(
            writer,
            "提示：{} 已指向记录以外的位置，只删记录（路径链接仍保留）",
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
            message: format!(
                "移回目标到链接位置失败：link 已移除，数据库记录已保留，可修复原因后重试 restore：{e}"
            ),
        })
    })
}

enum RestoreFailure {
    LinkUnchanged(SymmError),
    LinkRemoved(SymmError),
}
