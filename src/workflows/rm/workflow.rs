//! `rm`：删除 link 后删库，或将 target 迁回 link 路径。支持多个 selector；省略参数时交互多选。
use crate::adapters::db::link_store;
use crate::adapters::migrate;
use crate::adapters::status;
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkRecord, LinkStatus};
use crate::ui::progress::migration_reporter::{MigrationProgressReporter, ProgressSinkMode};
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

impl RemoveMode {
    fn action_label(self) -> &'static str {
        match self {
            RemoveMode::DeleteLinkOnly => "删除",
            RemoveMode::RestoreTargetToLink => "恢复",
        }
    }

    fn success_hint(self) -> &'static str {
        match self {
            RemoveMode::DeleteLinkOnly => "已删除链接关系",
            RemoveMode::RestoreTargetToLink => "已恢复实体位置",
        }
    }

    fn partial_failure_hint(self) -> &'static str {
        match self {
            RemoveMode::DeleteLinkOnly => "部分删除失败",
            RemoveMode::RestoreTargetToLink => "部分恢复失败",
        }
    }

    fn perf_action(self) -> &'static str {
        match self {
            RemoveMode::DeleteLinkOnly => "delete_link_only",
            RemoveMode::RestoreTargetToLink => "restore_target_to_link",
        }
    }
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
        ProgressSinkMode::Terminal,
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
        ProgressSinkMode::Terminal,
    )
}

#[allow(dead_code)]
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
        ProgressSinkMode::Terminal,
    )
}

#[allow(dead_code)]
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
        ProgressSinkMode::Terminal,
    )
}

#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn run_rm_by_ids_buffered<W: Write>(
    conn: &rusqlite::Connection,
    ids: &[i64],
    writer: &mut W,
) -> Result<(), SymmError> {
    run_remove(
        conn,
        RmSelection::RecordIds(ids),
        RemoveMode::DeleteLinkOnly,
        writer,
        ProgressSinkMode::Buffered,
    )
}

#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn run_restore_by_ids_buffered<W: Write>(
    conn: &rusqlite::Connection,
    ids: &[i64],
    writer: &mut W,
) -> Result<(), SymmError> {
    run_remove(
        conn,
        RmSelection::RecordIds(ids),
        RemoveMode::RestoreTargetToLink,
        writer,
        ProgressSinkMode::Buffered,
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
    progress_mode: ProgressSinkMode,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let resolved = match selection {
        RmSelection::Selectors(selectors) => ResolvedRecords {
            records: resolve_records(conn, selectors, mode)?,
            failures: Vec::new(),
        },
        RmSelection::RecordIds(ids) => records_from_ids(conn, ids)?,
    };
    run_resolved_records(
        conn,
        resolved.records,
        resolved.failures,
        mode,
        writer,
        started,
        progress_mode,
    )
}

struct ResolvedRecords {
    records: Vec<LinkRecord>,
    failures: Vec<(String, SymmError)>,
}

fn run_resolved_records<W: Write>(
    conn: &rusqlite::Connection,
    records: Vec<LinkRecord>,
    mut failures: Vec<(String, SymmError)>,
    mode: RemoveMode,
    writer: &mut W,
    started: Instant,
    progress_mode: ProgressSinkMode,
) -> Result<(), SymmError> {
    let mut labels = Vec::with_capacity(records.len());
    for record in records {
        match remove_one(conn, &record, mode, writer, progress_mode) {
            Ok(label) => labels.push(label),
            Err(err) => failures.push((record_label(&record), err)),
        }
    }

    if labels.is_empty() {
        return Err(batch_failure_error(failures));
    }

    let action_hint = mode.success_hint();
    let summary = if labels.len() == 1 {
        labels[0].clone()
    } else {
        format!("共 {} 条：{}", labels.len(), labels.join("、"))
    };
    writeln!(writer, "{action_hint}：{summary}")
        .map_err(|e| output_error_after_failures(&failures, e))?;
    for (label, err) in &failures {
        writeln!(writer, "失败：{label}：{err}")
            .map_err(|e| output_error_after_failures(&failures, e))?;
    }

    perf::log_perf_lazy("rm", started.elapsed(), || {
        vec![
            ("count", labels.len().to_string()),
            ("failures", failures.len().to_string()),
            ("action", mode.perf_action().to_string()),
        ]
    });
    if !failures.is_empty() {
        return Err(partial_failure_error(mode, &failures));
    }
    Ok(())
}

fn batch_failure_error(mut failures: Vec<(String, SymmError)>) -> SymmError {
    if failures.len() == 1 {
        return failures.remove(0).1;
    }
    batch_error_with_message(&failures, format_failures(&failures))
}

fn partial_failure_error(mode: RemoveMode, failures: &[(String, SymmError)]) -> SymmError {
    batch_error_with_message(
        failures,
        format!(
            "{}：{}",
            mode.partial_failure_hint(),
            format_failures(failures)
        ),
    )
}

fn output_error_after_failures(failures: &[(String, SymmError)], err: std::io::Error) -> SymmError {
    if failures.iter().any(|(_, err)| is_half_applied_error(err)) {
        return SymmError::BatchFilesystemAppliedButRecordIncomplete {
            message: format!("输出批量结果失败：{}\n{}", err, format_failures(failures)),
        };
    }
    SymmError::IoError {
        message: err.to_string(),
    }
}

fn batch_error_with_message(failures: &[(String, SymmError)], message: String) -> SymmError {
    if failures.iter().any(|(_, err)| is_half_applied_error(err)) {
        return SymmError::BatchFilesystemAppliedButRecordIncomplete { message };
    }
    SymmError::BatchFailure { message }
}

fn is_half_applied_error(err: &SymmError) -> bool {
    matches!(
        err,
        SymmError::FilesystemAppliedButRecordDeleteFailed { .. }
            | SymmError::FilesystemAppliedButRecordKept { .. }
            | SymmError::BatchFilesystemAppliedButRecordIncomplete { .. }
    )
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
    mode: RemoveMode,
) -> Result<Vec<LinkRecord>, SymmError> {
    if selectors.is_empty() {
        return select::pick_many_records(conn, mode.action_label());
    }

    let records = selector::records_from_tokens(conn, selectors)?;
    if records.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: format!("未指定要{}的记录", mode.action_label()),
        });
    }
    Ok(records)
}

fn records_from_ids(
    conn: &rusqlite::Connection,
    ids: &[i64],
) -> Result<ResolvedRecords, SymmError> {
    if ids.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "未指定要操作的记录".to_string(),
        });
    }
    let mut unique_ids = Vec::with_capacity(ids.len());
    let mut seen_ids = std::collections::HashSet::new();
    for id in ids {
        if seen_ids.insert(*id) {
            unique_ids.push(*id);
        }
    }
    let records = link_store::find_existing_by_ids(conn, &unique_ids)?;
    let existing = records
        .iter()
        .map(|record| record.id)
        .collect::<std::collections::HashSet<_>>();
    let failures = unique_ids
        .iter()
        .filter(|id| !existing.contains(id))
        .map(|id| {
            (
                format!("#{id}"),
                SymmError::NotFound {
                    selector: format!("#{id}"),
                },
            )
        })
        .collect();
    Ok(ResolvedRecords { records, failures })
}

fn remove_one<W: Write>(
    conn: &rusqlite::Connection,
    record: &LinkRecord,
    mode: RemoveMode,
    writer: &mut W,
    progress_mode: ProgressSinkMode,
) -> Result<String, SymmError> {
    let link = Path::new(&record.link_path);
    let link_status = status::try_for_record(record)?;

    let filesystem_applied = match mode {
        RemoveMode::RestoreTargetToLink => {
            ensure_restorable(record, link_status)?;
            if let Err(err) = restore_target_to_link(writer, record, link_status, progress_mode) {
                match err {
                    RestoreFailure::LinkUnchanged(err) => return Err(err),
                    RestoreFailure::LinkRemoved(err) => return Err(err),
                }
            }
            true
        }
        RemoveMode::DeleteLinkOnly => apply_delete_link_only(writer, record, link, link_status)?,
    };

    link_store::delete_known_id(conn, record.id).map_err(|err| {
        if filesystem_applied {
            filesystem_applied_but_record_delete_failed(record, mode, err)
        } else {
            err
        }
    })?;
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
) -> Result<bool, SymmError> {
    if should_unlink_on_disk(link_status) {
        let current_status = status::try_for_record(record)?;
        if should_unlink_on_disk(current_status) {
            match symlink::unlink_expected(link, record.link_kind, Path::new(&record.target_path)) {
                Ok(()) => return Ok(true),
                Err(err) => {
                    if !matches!(
                        symlink::inspect_link_path(link)?,
                        symlink::LinkPathState::Missing
                    ) {
                        return Err(err);
                    }
                    writeln!(
                        writer,
                        "提示：{} 状态已变化为 missing，只删记录（当前路径已不存在）",
                        record.link_path
                    )
                    .map_err(|e| SymmError::IoError {
                        message: e.to_string(),
                    })?;
                    return Ok(false);
                }
            }
        }
        writeln!(
            writer,
            "提示：{} 状态已变化，只删记录（当前路径不再按原计划删除）",
            record.link_path
        )
        .map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    } else if link_status == LinkStatus::Stale {
        writeln!(
            writer,
            "提示：{} 已不是记录期望的链接，只删记录（路径实体仍保留）",
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
    Ok(false)
}

fn record_label(record: &LinkRecord) -> String {
    if !record.name.is_empty() {
        return record.name.clone();
    }
    format!("未命名记录：{}", record.link_path)
}

fn restore_target_to_link<W: Write>(
    writer: &mut W,
    record: &LinkRecord,
    planned_status: LinkStatus,
    progress_mode: ProgressSinkMode,
) -> Result<(), RestoreFailure> {
    let link = Path::new(&record.link_path);
    let target = Path::new(&record.target_path);
    ensure_restore_target_ready(record, link, target).map_err(RestoreFailure::LinkUnchanged)?;
    remove_current_link_for_restore(record, link, target, planned_status)
        .map_err(RestoreFailure::LinkUnchanged)?;
    let mut reporter = MigrationProgressReporter::new_with_mode(writer, progress_mode);
    migrate::migrate_path(target, link, &mut |event| {
        let _ = reporter.handle_migration_event(event);
        Ok(())
    })
    .or_else(|err| finish_restore_migration_error(writer, record, err))
}

fn finish_restore_migration_error<W: Write>(
    writer: &mut W,
    record: &LinkRecord,
    err: SymmError,
) -> Result<(), RestoreFailure> {
    match err {
        SymmError::EntityCopiedButSourceCleanupFailed {
            source_path,
            target_path,
            message,
            ..
        } => {
            writeln!(
                writer,
                "提示：restore 已把实体恢复到 link 路径 {target_path}，但旧 target 清理失败：{message}；请手动检查并清理 {source_path}"
            )
            .map_err(|e| {
                RestoreFailure::LinkRemoved(filesystem_applied_but_record_kept(
                    record,
                    SymmError::IoError {
                        message: e.to_string(),
                    },
                ))
            })?;
            Ok(())
        }
        SymmError::EntityMovedButPostMoveFailed {
            source_path,
            target_path,
            message,
        } => {
            let _ = writeln!(
                writer,
                "提示：restore 已把实体恢复到 link 路径 {target_path}，但内部链接整理失败：{message}；旧 target {source_path} 已不存在，请手动检查恢复后的目录"
            );
            Ok(())
        }
        err => Err(RestoreFailure::LinkRemoved(
            filesystem_applied_but_record_kept(record, err),
        )),
    }
}

fn filesystem_applied_but_record_delete_failed(
    record: &LinkRecord,
    mode: RemoveMode,
    err: SymmError,
) -> SymmError {
    SymmError::FilesystemAppliedButRecordDeleteFailed {
        operation: mode.perf_action().to_string(),
        link_path: record.link_path.clone(),
        target_path: record.target_path.clone(),
        message: err.to_string(),
    }
}

fn filesystem_applied_but_record_kept(record: &LinkRecord, err: SymmError) -> SymmError {
    SymmError::FilesystemAppliedButRecordKept {
        operation: RemoveMode::RestoreTargetToLink.perf_action().to_string(),
        link_path: record.link_path.clone(),
        target_path: record.target_path.clone(),
        message: format!(
            "移回目标到链接位置失败：link 已移除，数据库记录已保留，可修复原因后重试 restore：{err}"
        ),
    }
}

fn ensure_restore_target_ready(
    record: &LinkRecord,
    link: &Path,
    target: &Path,
) -> Result<(), SymmError> {
    if !crate::adapters::paths::presence::target_exists(target)? {
        return Err(SymmError::TargetNotFound {
            path: record.target_path.clone(),
        });
    }
    if restore_link_path_is_inside_target(link, target)? {
        return Err(SymmError::InvalidArgument {
            message: format!(
                "link 路径位于 target 目录内部，无法安全 restore：{} -> {}",
                record.target_path, record.link_path
            ),
        });
    }
    Ok(())
}

fn restore_link_path_is_inside_target(link: &Path, target: &Path) -> Result<bool, SymmError> {
    let Some(link_parent) = link.parent() else {
        return Ok(false);
    };
    let target = dunce::canonicalize(target).map_err(|e| SymmError::IoError {
        message: format!("无法解析 target 路径：{}：{e}", target.display()),
    })?;
    let link_parent = dunce::canonicalize(link_parent).map_err(|e| SymmError::IoError {
        message: format!("无法解析 link 父目录：{}：{e}", link_parent.display()),
    })?;
    Ok(link_parent == target || link_parent.starts_with(&target))
}

fn remove_current_link_for_restore(
    record: &LinkRecord,
    link: &Path,
    target: &Path,
    planned_status: LinkStatus,
) -> Result<(), SymmError> {
    match planned_status {
        LinkStatus::Ok => {
            symlink::unlink_expected(link, record.link_kind, target)?;
            ensure_restore_link_missing_after_unlink(record, link)
        }
        LinkStatus::Missing => ensure_restore_link_still_missing(record, link),
        LinkStatus::Broken | LinkStatus::Stale | LinkStatus::Drift | LinkStatus::Unknown => {
            Err(restore_link_state_changed(record))
        }
    }
}

fn ensure_restore_link_still_missing(record: &LinkRecord, link: &Path) -> Result<(), SymmError> {
    if matches!(
        symlink::inspect_link_path(link)?,
        symlink::LinkPathState::Missing
    ) {
        return Ok(());
    }
    Err(restore_link_state_changed(record))
}

fn ensure_restore_link_missing_after_unlink(
    record: &LinkRecord,
    link: &Path,
) -> Result<(), SymmError> {
    if matches!(
        symlink::inspect_link_path(link)?,
        symlink::LinkPathState::Missing
    ) {
        return Ok(());
    }
    Err(SymmError::InvalidArgument {
        message: format!(
            "link 路径删除后仍被占用，无法 restore，请重新执行：{}",
            record.link_path
        ),
    })
}

fn restore_link_state_changed(record: &LinkRecord) -> SymmError {
    SymmError::InvalidArgument {
        message: format!(
            "link 路径状态已变化，无法 restore，请重新执行：{}",
            record.link_path
        ),
    }
}

#[derive(Debug)]
enum RestoreFailure {
    LinkUnchanged(SymmError),
    LinkRemoved(SymmError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::db::schema;
    use crate::domain::model::LinkKind;
    use rusqlite::Connection;
    use std::fs;
    use std::io;
    use tempfile::tempdir;

    struct FailWriter;

    impl Write for FailWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("writer failed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn path_text(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }

    fn insert_record(conn: &Connection, name: &str, link: &Path, target: &Path) {
        link_store::upsert_link(
            conn,
            name,
            &path_text(link),
            &path_text(target),
            LinkKind::Symlink,
        )
        .expect("insert link record");
    }

    fn record(name: &str, link: &Path, target: &Path) -> LinkRecord {
        LinkRecord {
            id: 1,
            name: name.to_string(),
            link_path: path_text(link),
            target_path: path_text(target),
            link_kind: LinkKind::Symlink,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate schema");
        conn
    }

    #[test]
    fn delete_link_only_does_not_unlink_when_current_link_drifted_after_planned_ok() {
        let temp = tempdir().expect("temp dir");
        let target_a = temp.path().join("target-a.txt");
        let target_b = temp.path().join("target-b.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target_a, "a").expect("write target a");
        fs::write(&target_b, "b").expect("write target b");
        symlink::create_link(&target_b, &link).expect("create drifted link");
        let record = record("drifted", &link, &target_a);
        let mut output = Vec::new();

        let applied =
            apply_delete_link_only(&mut output, &record, &link, LinkStatus::Ok).expect("apply rm");

        assert!(!applied);
        assert_eq!(
            fs::read_to_string(&link).expect("read current link"),
            "b",
            "rm must not delete a link that drifted after the planned status was read"
        );
        let text = String::from_utf8(output).expect("utf8");
        assert!(text.contains("状态已变化"));
    }

    #[test]
    fn delete_link_only_deletes_record_when_current_link_disappears_at_final_unlink() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, "payload").expect("write target");
        symlink::create_link(&target, &link).expect("create link");
        let record = record("missing-race", &link, &target);
        let mut output = Vec::new();

        symlink::set_before_expected_unlink_hook(|link| {
            symlink::unlink(link).expect("remove link in hook");
        });

        let applied = apply_delete_link_only(&mut output, &record, &link, LinkStatus::Ok)
            .expect("missing link race should still allow record deletion");

        assert!(!applied);
        assert!(
            fs::symlink_metadata(&link).is_err(),
            "link should remain missing"
        );
        let text = String::from_utf8(output).expect("utf8");
        assert!(text.contains("状态已变化为 missing"));
    }

    #[test]
    fn rm_reports_half_applied_error_when_record_delete_fails_after_unlink() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, "payload").expect("write target");
        symlink::create_link(&target, &link).expect("create link");
        let conn = memory_db();
        insert_record(&conn, "rm-half", &link, &target);
        conn.execute_batch("PRAGMA query_only = ON;")
            .expect("make db readonly");
        let mut output = Vec::new();

        let err = run_rm(&conn, &["rm-half".to_string()], &mut output)
            .expect_err("delete record should fail after unlink");

        let SymmError::FilesystemAppliedButRecordDeleteFailed {
            operation,
            link_path,
            target_path,
            ..
        } = err
        else {
            panic!("unexpected error: {err:?}");
        };
        assert_eq!(operation, "delete_link_only");
        assert_eq!(link_path, path_text(&link));
        assert_eq!(target_path, path_text(&target));
        assert!(fs::symlink_metadata(&link).is_err());
        assert!(target.exists());
    }

    #[test]
    fn restore_reports_half_applied_error_when_record_delete_fails_after_migration() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, "payload").expect("write target");
        symlink::create_link(&target, &link).expect("create link");
        let conn = memory_db();
        insert_record(&conn, "restore-half", &link, &target);
        conn.execute_batch("PRAGMA query_only = ON;")
            .expect("make db readonly");
        let mut output = Vec::new();

        let err = run_restore(&conn, &["restore-half".to_string()], &mut output)
            .expect_err("delete record should fail after restore");

        let SymmError::FilesystemAppliedButRecordDeleteFailed {
            operation,
            link_path,
            target_path,
            ..
        } = err
        else {
            panic!("unexpected error: {err:?}");
        };
        assert_eq!(operation, "restore_target_to_link");
        assert_eq!(link_path, path_text(&link));
        assert_eq!(target_path, path_text(&target));
        assert_eq!(
            fs::read_to_string(&link).expect("read restored entity"),
            "payload"
        );
        assert!(!target.exists());
    }

    #[test]
    fn restore_progress_output_failure_does_not_stop_migration_after_unlink() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, "payload").expect("write target");
        symlink::create_link(&target, &link).expect("create link");
        let record = record("restore-kept", &link, &target);
        let mut writer = FailWriter;

        restore_target_to_link(
            &mut writer,
            &record,
            LinkStatus::Ok,
            ProgressSinkMode::Terminal,
        )
        .expect("progress output failure should not stop restore migration");

        assert_eq!(
            fs::read_to_string(&link).expect("read restored entity"),
            "payload"
        );
        assert!(!target.exists());
    }

    #[test]
    fn restore_refuses_missing_target_before_unlinking_current_link() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("missing-target.txt");
        let live_target = temp.path().join("live-target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&live_target, "live").expect("write live target");
        symlink::create_link(&live_target, &link).expect("create current link");
        let record = record("restore-missing-target", &link, &target);

        let err = restore_target_to_link(
            &mut Vec::new(),
            &record,
            LinkStatus::Missing,
            ProgressSinkMode::Terminal,
        )
        .expect_err("missing target should fail before migration");

        assert!(matches!(
            err,
            RestoreFailure::LinkUnchanged(SymmError::TargetNotFound { .. })
        ));
        assert!(
            fs::symlink_metadata(&link).is_ok(),
            "restore should not unlink before target existence is confirmed"
        );
    }

    #[test]
    fn restore_refuses_link_inside_target_before_unlinking_current_link() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target-dir");
        let link = target.join("managed-link");
        fs::create_dir_all(&target).expect("create target");
        symlink::create_link(&target, &link).expect("create nested link");
        let record = record("restore-nested", &link, &target);

        let err = restore_target_to_link(
            &mut Vec::new(),
            &record,
            LinkStatus::Ok,
            ProgressSinkMode::Terminal,
        )
        .expect_err("nested link should fail before unlink");

        assert!(matches!(
            err,
            RestoreFailure::LinkUnchanged(SymmError::InvalidArgument { .. })
        ));
        assert!(
            fs::symlink_metadata(&link).is_ok(),
            "nested link should still exist after rejected restore"
        );
    }

    #[test]
    fn restore_refuses_to_unlink_when_current_link_drifted_after_planned_ok() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let other = temp.path().join("other.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, "payload").expect("write target");
        fs::write(&other, "other").expect("write other");
        symlink::create_link(&other, &link).expect("create drifted link");
        let record = record("restore-drift", &link, &target);

        let err = remove_current_link_for_restore(&record, &link, &target, LinkStatus::Ok)
            .expect_err("drifted link should not be removed");

        assert!(matches!(err, SymmError::InvalidArgument { .. }));
        assert_eq!(
            fs::read_to_string(&link).expect("read current drifted link"),
            "other"
        );
    }

    #[test]
    fn restore_refuses_to_unlink_when_current_path_became_entity_after_planned_ok() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, "payload").expect("write target");
        fs::write(&link, "external").expect("write competing entity");
        let record = record("restore-stale", &link, &target);

        let err = remove_current_link_for_restore(&record, &link, &target, LinkStatus::Ok)
            .expect_err("plain entity should not be removed");

        assert!(matches!(err, SymmError::InvalidArgument { .. }));
        assert_eq!(
            fs::read_to_string(&link).expect("read competing entity"),
            "external"
        );
    }

    #[test]
    fn restore_copy_cleanup_failure_can_still_delete_record() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        let record = record("restore-cleanup", &link, &target);
        let mut output = Vec::new();

        finish_restore_migration_error(
            &mut output,
            &record,
            SymmError::EntityCopiedButSourceCleanupFailed {
                source_path: path_text(&target),
                target_path: path_text(&link),
                message: "源路径删不掉".to_string(),
            },
        )
        .expect("cleanup failure should not keep restore record");

        let text = String::from_utf8(output).expect("utf8");
        assert!(text.contains("旧 target 清理失败"));
        assert!(text.contains(&path_text(&target)));
        assert!(text.contains(&path_text(&link)));
        assert!(
            text.find(&format!("清理 {}", path_text(&target))).is_some(),
            "cleanup instruction should point at old target, not restored link: {text}"
        );
    }

    #[test]
    fn restore_post_move_failure_can_still_delete_record_with_manual_check_warning() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target-dir");
        let link = temp.path().join("link-dir");
        let record = record("restore-rebase", &link, &target);
        let mut output = Vec::new();

        finish_restore_migration_error(
            &mut output,
            &record,
            SymmError::EntityMovedButPostMoveFailed {
                source_path: path_text(&target),
                target_path: path_text(&link),
                message: "内部链接重写失败".to_string(),
            },
        )
        .expect("post-move failure should not keep restore record");

        let text = String::from_utf8(output).expect("utf8");
        assert!(text.contains("内部链接整理失败"));
        assert!(text.contains("旧 target"));
        assert!(text.contains(&path_text(&target)));
        assert!(text.contains(&path_text(&link)));
    }

    #[test]
    fn restore_post_move_failure_ignores_warning_output_failure() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target-dir");
        let link = temp.path().join("link-dir");
        let record = record("restore-rebase-output", &link, &target);
        let mut writer = FailWriter;

        finish_restore_migration_error(
            &mut writer,
            &record,
            SymmError::EntityMovedButPostMoveFailed {
                source_path: path_text(&target),
                target_path: path_text(&link),
                message: "内部链接重写失败".to_string(),
            },
        )
        .expect("warning output failure must not keep an unretryable restore record");
    }

    #[test]
    fn partial_batch_preserves_half_applied_record_code() {
        let failures = vec![(
            "half".to_string(),
            SymmError::FilesystemAppliedButRecordDeleteFailed {
                operation: "delete_link_only".to_string(),
                link_path: "/tmp/link".to_string(),
                target_path: "/tmp/target".to_string(),
                message: "db failed".to_string(),
            },
        )];

        let err = partial_failure_error(RemoveMode::DeleteLinkOnly, &failures);

        assert!(matches!(
            err,
            SymmError::BatchFilesystemAppliedButRecordIncomplete { .. }
        ));
        assert_eq!(err.code(), "filesystem_applied_but_record_incomplete");
    }

    #[test]
    fn output_failure_after_partial_half_applied_batch_preserves_record_code() {
        let temp = tempdir().expect("temp dir");
        let success_target = temp.path().join("success-target.txt");
        let success_link = temp.path().join("success-link.txt");
        let half_target = temp.path().join("half-target.txt");
        let half_link = temp.path().join("half-link.txt");
        fs::write(&success_target, "success").expect("write success target");
        fs::write(&half_target, "half").expect("write half target");
        symlink::create_link(&success_target, &success_link).expect("create success link");
        symlink::create_link(&half_target, &half_link).expect("create half link");
        let conn = memory_db();
        insert_record(&conn, "success", &success_link, &success_target);
        insert_record(&conn, "half", &half_link, &half_target);
        conn.execute_batch(
            "CREATE TRIGGER block_half_delete
             BEFORE DELETE ON links
             WHEN old.name = 'half'
             BEGIN
                 SELECT RAISE(FAIL, 'blocked half delete');
             END;",
        )
        .expect("create delete trigger");
        let records =
            link_store::find_by_names(&conn, &["success".to_string(), "half".to_string()])
                .expect("load records");
        let mut writer = FailWriter;

        let err = run_resolved_records(
            &conn,
            records,
            Vec::new(),
            RemoveMode::DeleteLinkOnly,
            &mut writer,
            Instant::now(),
            ProgressSinkMode::Terminal,
        )
        .expect_err("summary output should fail after collecting half-applied failure");

        assert!(matches!(
            err,
            SymmError::BatchFilesystemAppliedButRecordIncomplete { .. }
        ));
        assert_eq!(err.code(), "filesystem_applied_but_record_incomplete");
        assert!(fs::symlink_metadata(&success_link).is_err());
        assert!(fs::symlink_metadata(&half_link).is_err());
        assert!(half_target.exists());
    }

    #[test]
    fn rm_by_ids_continues_when_one_selected_id_disappeared() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, "payload").expect("write target");
        symlink::create_link(&target, &link).expect("create link");
        let conn = memory_db();
        insert_record(&conn, "live", &link, &target);
        let mut output = Vec::new();

        let err = run_rm_by_ids_buffered(&conn, &[1, 99], &mut output)
            .expect_err("missing id should be reported as partial failure");

        assert!(matches!(err, SymmError::BatchFailure { .. }));
        assert!(fs::symlink_metadata(&link).is_err());
        assert!(target.exists());
        let text = String::from_utf8(output).expect("utf8");
        assert!(text.contains("已删除链接关系"));
        assert!(text.contains("#99"));
    }

    #[test]
    fn rm_by_ids_ignores_duplicate_ids_in_one_batch() {
        let temp = tempdir().expect("temp dir");
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, "payload").expect("write target");
        symlink::create_link(&target, &link).expect("create link");
        let conn = memory_db();
        insert_record(&conn, "dup", &link, &target);
        let mut output = Vec::new();

        run_rm_by_ids_buffered(&conn, &[1, 1], &mut output)
            .expect("duplicate ids should be a single operation");

        assert!(fs::symlink_metadata(&link).is_err());
        assert!(target.exists());
        assert_eq!(link_store::count(&conn).expect("count records"), 0);
    }
}
