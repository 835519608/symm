use crate::adapters::db::link_store;
use crate::adapters::lock::ProcInfo;
use crate::adapters::migrate;
use crate::adapters::paths::runtime_paths;
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkKind, LinkRecord, prepare_link_name_for_storage};
use crate::ui::progress::migration_reporter::{MigrationProgressReporter, WorkflowProgressEvent};
use crate::workflows::link_ops::lock_gate;
use crate::workflows::perf;
use std::fs;
use std::io::ErrorKind;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOpLockChoice {
    Unlock,
    Cancel,
}

impl From<LinkOpLockChoice> for lock_gate::LockResolutionAction {
    fn from(value: LinkOpLockChoice) -> Self {
        match value {
            LinkOpLockChoice::Unlock => Self::UnlockAll,
            LinkOpLockChoice::Cancel => Self::Cancel,
        }
    }
}

pub trait LinkOpDecisionProvider {
    fn name(&mut self, default_name: &str) -> Result<String, SymmError>;
    fn lock_choice(&mut self, procs: &[ProcInfo]) -> Result<LinkOpLockChoice, SymmError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub enum LinkOperation {
    #[default]
    Add,
    Adopt,
    Point,
}

#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn run_operation<W: Write>(
    conn: &rusqlite::Connection,
    operation: LinkOperation,
    link: &Path,
    target: &Path,
    decisions: &mut impl LinkOpDecisionProvider,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let (link_norm, target_norm) =
        execute_operation(conn, operation, link, target, decisions, writer)?;
    perf::log_perf_lazy(operation.perf_label(), started.elapsed(), || {
        vec![("link_path", link_norm), ("target_path", target_norm)]
    });
    Ok(())
}

fn execute_operation<W: Write>(
    conn: &rusqlite::Connection,
    operation: LinkOperation,
    link: &Path,
    target: &Path,
    decisions: &mut impl LinkOpDecisionProvider,
    writer: &mut W,
) -> Result<(String, String), SymmError> {
    let link_norm = runtime_paths::normalize_link(link);
    let existing = link_store::find_by_link_path(conn, &link_norm)?;
    let link_path = Path::new(&link_norm);
    let link_state = symlink::inspect_link_path(link_path)?;
    let change = plan_filesystem_change(operation, link_path, target, link_state)?;
    let applies_filesystem_change = change.applies_filesystem_change();
    let name_input = prepare_record_name(conn, decisions, existing.as_ref())?;
    let mut reporter = MigrationProgressReporter::new(writer);
    let (target_norm, link_kind) =
        apply_filesystem_change(&mut reporter, decisions, &link_norm, link_path, change)?;
    if let Err(err) = reporter.handle_workflow_event(WorkflowProgressEvent::PersistingDb {
        link: link_norm.to_string(),
    }) {
        if applies_filesystem_change {
            return Err(filesystem_applied_but_db_failed(
                operation,
                &link_norm,
                &target_norm,
                err,
            ));
        }
        return Err(err);
    }
    let persist_result = persist_record(
        conn,
        &mut reporter,
        PersistRecord {
            name_input,
            link_norm: &link_norm,
            target_norm: &target_norm,
            link_kind,
            verb: operation.verb(),
        },
    )?;
    if let Err(err) = persist_result {
        if !applies_filesystem_change {
            return Err(err);
        }
        return Err(filesystem_applied_but_db_failed(
            operation,
            &link_norm,
            &target_norm,
            err,
        ));
    }
    Ok((link_norm, target_norm))
}

fn filesystem_applied_but_db_failed(
    operation: LinkOperation,
    link_norm: &str,
    target_norm: &str,
    err: SymmError,
) -> SymmError {
    SymmError::FilesystemAppliedButDbFailed {
        operation: operation.perf_label().to_string(),
        link_path: link_norm.to_string(),
        target_path: target_norm.to_string(),
        message: err.to_string(),
    }
}

enum PlannedFilesystemChange {
    CreateNewLink { target_norm: String },
    ReuseExistingLink { target_norm: String, kind: LinkKind },
    AdoptEntity { target: PathBuf },
    ReplaceExistingLink { target_norm: String },
}

impl PlannedFilesystemChange {
    fn applies_filesystem_change(&self) -> bool {
        !matches!(self, Self::ReuseExistingLink { .. })
    }
}

enum LinkPathMutation<'a> {
    Create { target_norm: &'a str },
    Replace { target_norm: &'a str },
}

impl LinkOperation {
    fn perf_label(self) -> &'static str {
        match self {
            LinkOperation::Add => "add",
            LinkOperation::Adopt => "adopt",
            LinkOperation::Point => "point",
        }
    }

    fn verb(self) -> &'static str {
        match self {
            LinkOperation::Add => "已添加",
            LinkOperation::Adopt => "已接管",
            LinkOperation::Point => "已改指向",
        }
    }
}

fn plan_filesystem_change(
    operation: LinkOperation,
    link_path: &Path,
    target: &Path,
    link_state: symlink::LinkPathState,
) -> Result<PlannedFilesystemChange, SymmError> {
    match operation {
        LinkOperation::Add => plan_add(link_path, target, link_state),
        LinkOperation::Adopt => plan_adopt(link_path, target, link_state),
        LinkOperation::Point => plan_point(link_path, target, link_state),
    }
}

fn plan_add(
    link_path: &Path,
    target: &Path,
    link_state: symlink::LinkPathState,
) -> Result<PlannedFilesystemChange, SymmError> {
    let target_norm = runtime_paths::normalize_target(target)?;
    match link_state {
        symlink::LinkPathState::Missing => {
            Ok(PlannedFilesystemChange::CreateNewLink { target_norm })
        }
        symlink::LinkPathState::Link { kind } => {
            if !symlink::link_points_to(link_path, Path::new(&target_norm))? {
                return Err(SymmError::InvalidArgument {
                    message: format!(
                        "link 路径已是指向其他 target 的链接，请使用 point：{}",
                        link_path.display()
                    ),
                });
            }
            Ok(PlannedFilesystemChange::ReuseExistingLink { target_norm, kind })
        }
        symlink::LinkPathState::Entity => Err(SymmError::InvalidArgument {
            message: format!(
                "link 路径已被真实实体占用，请使用 adopt：{}",
                link_path.display()
            ),
        }),
    }
}

fn plan_adopt(
    link_path: &Path,
    target: &Path,
    link_state: symlink::LinkPathState,
) -> Result<PlannedFilesystemChange, SymmError> {
    if path_exists(target)? {
        return Err(SymmError::InvalidArgument {
            message: format!("target 路径已存在，adopt 默认不替换：{}", target.display()),
        });
    }
    match link_state {
        symlink::LinkPathState::Entity => Ok(PlannedFilesystemChange::AdoptEntity {
            target: target.to_path_buf(),
        }),
        symlink::LinkPathState::Link { .. } => Err(SymmError::InvalidArgument {
            message: format!(
                "link 路径已是链接，请使用 add 或 point：{}",
                link_path.display()
            ),
        }),
        symlink::LinkPathState::Missing => Err(SymmError::InvalidArgument {
            message: format!("link 路径不存在，无法 adopt：{}", link_path.display()),
        }),
    }
}

fn plan_point(
    link_path: &Path,
    target: &Path,
    link_state: symlink::LinkPathState,
) -> Result<PlannedFilesystemChange, SymmError> {
    let target_norm = runtime_paths::normalize_target(target)?;
    match link_state {
        symlink::LinkPathState::Link { .. } => {
            Ok(PlannedFilesystemChange::ReplaceExistingLink { target_norm })
        }
        symlink::LinkPathState::Missing => Err(SymmError::InvalidArgument {
            message: format!("link 路径不存在，无法 point：{}", link_path.display()),
        }),
        symlink::LinkPathState::Entity => Err(SymmError::InvalidArgument {
            message: format!("link 路径不是链接，无法 point：{}", link_path.display()),
        }),
    }
}

fn apply_filesystem_change<W: Write>(
    reporter: &mut MigrationProgressReporter<'_, W>,
    decisions: &mut impl LinkOpDecisionProvider,
    link_norm: &str,
    link_path: &Path,
    change: PlannedFilesystemChange,
) -> Result<(String, LinkKind), SymmError> {
    match change {
        PlannedFilesystemChange::CreateNewLink { target_norm } => {
            let link_kind = mutate_link_path_after_lock(
                reporter,
                decisions,
                link_path,
                link_norm,
                LinkPathMutation::Create {
                    target_norm: &target_norm,
                },
            )?;
            Ok((target_norm, link_kind))
        }
        PlannedFilesystemChange::ReuseExistingLink { target_norm, kind } => Ok((target_norm, kind)),
        PlannedFilesystemChange::AdoptEntity { target } => {
            ensure_link_not_locked(reporter, decisions, link_path)?;
            ensure_target_parent_dir(&target)?;
            migrate::migrate_path(link_path, &target, &mut |event| {
                reporter.handle_migration_event(event)
            })
            .map_err(|e| SymmError::IoError {
                message: format!("接管失败：无法把 link 实体迁到 target：{e}"),
            })?;
            let target_norm = runtime_paths::normalize_target_known_exists(&target)?;
            let link_kind = create_managed_link(reporter, link_path, link_norm, &target_norm)
                .map_err(|err| SymmError::EntityMigratedButLinkCreateFailed {
                    link_path: link_norm.to_string(),
                    target_path: target_norm.clone(),
                    message: err.to_string(),
                })?;
            Ok((target_norm, link_kind))
        }
        PlannedFilesystemChange::ReplaceExistingLink { target_norm } => {
            let link_kind = mutate_link_path_after_lock(
                reporter,
                decisions,
                link_path,
                link_norm,
                LinkPathMutation::Replace {
                    target_norm: &target_norm,
                },
            )?;
            Ok((target_norm, link_kind))
        }
    }
}

fn mutate_link_path_after_lock<W: Write>(
    reporter: &mut MigrationProgressReporter<'_, W>,
    decisions: &mut impl LinkOpDecisionProvider,
    link_path: &Path,
    link_norm: &str,
    mutation: LinkPathMutation<'_>,
) -> Result<LinkKind, SymmError> {
    ensure_link_not_locked(reporter, decisions, link_path)?;
    match mutation {
        LinkPathMutation::Create { target_norm } => {
            create_managed_link(reporter, link_path, link_norm, target_norm)
        }
        LinkPathMutation::Replace { target_norm } => {
            emit_creating_link(reporter, link_norm, target_norm)?;
            replace_link_via_temp(link_path, Path::new(target_norm))
        }
    }
}

fn create_managed_link<W: Write>(
    reporter: &mut MigrationProgressReporter<'_, W>,
    link_path: &Path,
    link_norm: &str,
    target_norm: &str,
) -> Result<LinkKind, SymmError> {
    emit_creating_link(reporter, link_norm, target_norm)?;
    symlink::create_link(Path::new(target_norm), link_path)
}

fn ensure_link_not_locked<W: Write>(
    reporter: &mut MigrationProgressReporter<'_, W>,
    decisions: &mut impl LinkOpDecisionProvider,
    link_path: &Path,
) -> Result<(), SymmError> {
    lock_gate::ensure_link_not_locked_with_choice(link_path, reporter, &mut |procs| {
        decisions.lock_choice(procs).map(Into::into)
    })
}

fn emit_creating_link<W: Write>(
    reporter: &mut MigrationProgressReporter<'_, W>,
    link: &str,
    target: &str,
) -> Result<(), SymmError> {
    reporter.handle_workflow_event(WorkflowProgressEvent::CreatingLink {
        link: link.to_string(),
        target: target.to_string(),
    })
}

fn path_exists(path: &Path) -> Result<bool, SymmError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(false),
        Err(err) => Err(SymmError::IoError {
            message: format!("无法读取路径 {}：{err}", path.display()),
        }),
    }
}

struct PersistRecord<'a> {
    name_input: String,
    link_norm: &'a str,
    target_norm: &'a str,
    link_kind: LinkKind,
    verb: &'a str,
}

fn persist_record<W: Write>(
    conn: &rusqlite::Connection,
    reporter: &mut MigrationProgressReporter<'_, W>,
    input: PersistRecord<'_>,
) -> Result<Result<(), SymmError>, SymmError> {
    let name = match link_store::upsert_link(
        conn,
        &input.name_input,
        input.link_norm,
        input.target_norm,
        input.link_kind,
    ) {
        Ok(name) => name,
        Err(err) => return Ok(Err(err)),
    };
    if input.name_input != name && !input.name_input.is_empty() {
        reporter.write_line(&format!(
            "名称「{}」已改为「{name}」（纯数字名称会自动加前缀，避免与序号查询混淆）",
            input.name_input
        ))?;
    }
    reporter.handle_workflow_event(WorkflowProgressEvent::Done {
        link: input.link_norm.to_string(),
    })?;
    let display_name = if name.is_empty() {
        "(空)"
    } else {
        name.as_str()
    };
    reporter.write_line(&format!(
        "{}：{}（名称：{display_name}）",
        input.verb, input.link_norm
    ))?;
    Ok(Ok(()))
}

fn prepare_record_name(
    conn: &rusqlite::Connection,
    decisions: &mut impl LinkOpDecisionProvider,
    existing: Option<&LinkRecord>,
) -> Result<String, SymmError> {
    let default_name = existing.map(|r| r.name.as_str()).unwrap_or("");
    let name_input = decisions.name(default_name)?;
    let prepared = prepare_link_name_for_storage(&name_input);
    if prepared.stored.is_empty() {
        return Ok(name_input);
    }
    if let Some(conflict) = link_store::find_by_name_optional(conn, &prepared.stored)?
        && existing.is_none_or(|record| record.id != conflict.id)
    {
        return Err(SymmError::NameConflict {
            name: prepared.stored,
        });
    }
    Ok(name_input)
}

fn replace_link_via_temp(link: &Path, target: &Path) -> Result<LinkKind, SymmError> {
    let old_target = fs::read_link(link).map_err(|e| SymmError::IoError {
        message: format!("改指向失败：无法读取旧 link 指向：{e}"),
    })?;
    let spec = symlink::capture_recreate_spec(link)?;
    let temp = unique_temp_link_path(link);
    symlink::write_symlink_from_spec(spec, &temp, target)?;
    if let Err(err) = symlink::unlink(link) {
        let _ = symlink::unlink(&temp);
        return Err(err);
    }
    if let Err(err) = fs::rename(&temp, link) {
        let restore = symlink::write_symlink_from_spec(spec, link, &old_target);
        if restore.is_ok() {
            let _ = symlink::unlink(&temp);
        }
        let state = match restore {
            Ok(()) => "旧 link 已恢复".to_string(),
            Err(restore_err) => format!(
                "旧 link 已移除且恢复失败：{restore_err}；临时 link 保留在 {}",
                temp.display()
            ),
        };
        return Err(SymmError::IoError {
            message: format!(
                "改指向失败：临时 link 已创建，但替换 {} 失败：{err}；{state}",
                link.display(),
            ),
        });
    }
    let meta = fs::symlink_metadata(link).map_err(|e| SymmError::IoError {
        message: format!("改指向失败：无法确认新 link 类型：{e}"),
    })?;
    symlink::kind_from_path_and_metadata(link, &meta).ok_or_else(|| SymmError::IoError {
        message: format!("改指向失败：新 link 类型无效：{}", link.display()),
    })
}

fn unique_temp_link_path(link: &Path) -> PathBuf {
    let parent = link.parent().unwrap_or_else(|| Path::new("."));
    let file_name = link
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "link".into());
    let pid = std::process::id();
    for n in 0..1000u32 {
        let candidate = parent.join(format!(".{file_name}.symm-point-{pid}-{n}.tmp"));
        if fs::symlink_metadata(&candidate).is_err() {
            return candidate;
        }
    }
    parent.join(format!(".{file_name}.symm-point-{pid}.tmp"))
}

fn ensure_target_parent_dir(target: &Path) -> Result<(), SymmError> {
    let parent = target.parent().ok_or_else(|| SymmError::InvalidArgument {
        message: format!("无法解析 target 路径的父目录：{}", target.display()),
    })?;
    if crate::adapters::paths::presence::target_exists(parent)? {
        return Ok(());
    }
    std::fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
        message: format!("adopt 失败：无法创建目录 {}：{e}", parent.display()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::db::{link_store, schema};
    use rusqlite::Connection;
    use std::io::Write;
    use tempfile::tempdir;

    struct TestDecisions;

    impl LinkOpDecisionProvider for TestDecisions {
        fn name(&mut self, _default_name: &str) -> Result<String, SymmError> {
            Ok("db-fail".to_string())
        }

        fn lock_choice(
            &mut self,
            _procs: &[crate::adapters::lock::ProcInfo],
        ) -> Result<LinkOpLockChoice, SymmError> {
            Ok(LinkOpLockChoice::Cancel)
        }
    }

    struct NumericNameDecisions;

    impl LinkOpDecisionProvider for NumericNameDecisions {
        fn name(&mut self, _default_name: &str) -> Result<String, SymmError> {
            Ok("42".to_string())
        }

        fn lock_choice(
            &mut self,
            _procs: &[crate::adapters::lock::ProcInfo],
        ) -> Result<LinkOpLockChoice, SymmError> {
            Ok(LinkOpLockChoice::Cancel)
        }
    }

    struct FailAfterDbWriter;

    impl Write for FailAfterDbWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let text = String::from_utf8_lossy(buf);
            if text.contains("完成") || text.contains("已添加") {
                return Err(std::io::Error::other("writer failed after db"));
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct FailOnRenamedNameWriter;

    impl Write for FailOnRenamedNameWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let text = String::from_utf8_lossy(buf);
            if text.contains("已改为") {
                return Err(std::io::Error::other("writer failed after numeric rename"));
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct FailOnPersistingDbWriter;

    impl Write for FailOnPersistingDbWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let text = String::from_utf8_lossy(buf);
            if text.contains("正在保存记录") {
                return Err(std::io::Error::other("writer failed before db"));
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn add_reports_when_filesystem_changed_but_db_write_fails() {
        let temp = tempdir().expect("temp dir");
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate");
        conn.execute_batch("PRAGMA query_only = ON;")
            .expect("make db readonly");
        let link = temp.path().join("link.txt");
        let target = temp.path().join("target.txt");
        std::fs::write(&target, "payload").expect("write target");

        let mut decisions = TestDecisions;
        let mut out = Vec::new();
        let err = run_operation(
            &conn,
            LinkOperation::Add,
            &link,
            &target,
            &mut decisions,
            &mut out,
        )
        .expect_err("db failure after link creation should be explicit");

        let SymmError::FilesystemAppliedButDbFailed {
            operation,
            link_path,
            target_path,
            message,
        } = err
        else {
            panic!("unexpected error: {err:?}");
        };
        assert_eq!(operation, "add");
        assert_eq!(link_path, runtime_paths::normalize_link(&link));
        assert_eq!(
            target_path,
            runtime_paths::normalize_target(&target).expect("target norm")
        );
        assert!(
            message.contains("数据库错误"),
            "unexpected message: {message}"
        );
        assert_eq!(
            std::fs::read_to_string(&link).expect("read through created link"),
            "payload"
        );
    }

    #[test]
    fn output_failure_after_db_write_is_not_reported_as_db_failure() {
        let temp = tempdir().expect("temp dir");
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate");
        let link = temp.path().join("link.txt");
        let target = temp.path().join("target.txt");
        std::fs::write(&target, "payload").expect("write target");

        let mut decisions = TestDecisions;
        let mut writer = FailAfterDbWriter;
        let err = run_operation(
            &conn,
            LinkOperation::Add,
            &link,
            &target,
            &mut decisions,
            &mut writer,
        )
        .expect_err("writer failure after db should bubble as io");

        assert!(
            matches!(err, SymmError::IoError { ref message } if message.contains("writer failed after db")),
            "unexpected error: {err:?}"
        );
        let stored = link_store::find_by_link_path(&conn, &runtime_paths::normalize_link(&link))
            .expect("query link")
            .expect("db record should exist");
        assert_eq!(stored.name, "db-fail");
    }

    #[test]
    fn renamed_name_output_failure_after_db_write_is_not_reported_as_db_failure() {
        let temp = tempdir().expect("temp dir");
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate");
        let link = temp.path().join("link.txt");
        let target = temp.path().join("target.txt");
        std::fs::write(&target, "payload").expect("write target");

        let mut decisions = NumericNameDecisions;
        let mut writer = FailOnRenamedNameWriter;
        let err = run_operation(
            &conn,
            LinkOperation::Add,
            &link,
            &target,
            &mut decisions,
            &mut writer,
        )
        .expect_err("writer failure after numeric rename should bubble as io");

        assert!(
            matches!(err, SymmError::IoError { ref message } if message.contains("writer failed after numeric rename")),
            "unexpected error: {err:?}"
        );
        let stored = link_store::find_by_link_path(&conn, &runtime_paths::normalize_link(&link))
            .expect("query link")
            .expect("db record should exist");
        assert_eq!(stored.name, "link-42");
    }

    #[test]
    fn output_failure_before_db_write_reports_half_applied_filesystem_change() {
        let temp = tempdir().expect("temp dir");
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate");
        let link = temp.path().join("link.txt");
        let target = temp.path().join("target.txt");
        std::fs::write(&target, "payload").expect("write target");

        let mut decisions = TestDecisions;
        let mut writer = FailOnPersistingDbWriter;
        let err = run_operation(
            &conn,
            LinkOperation::Add,
            &link,
            &target,
            &mut decisions,
            &mut writer,
        )
        .expect_err("writer failure before db should report half-applied filesystem");

        assert!(matches!(
            err,
            SymmError::FilesystemAppliedButDbFailed { ref message, .. }
                if message.contains("writer failed before db")
        ));
        assert_eq!(
            std::fs::read_to_string(&link).expect("read through created link"),
            "payload"
        );
        assert!(
            link_store::find_by_link_path(&conn, &runtime_paths::normalize_link(&link))
                .expect("query link")
                .is_none(),
            "db record should not exist when persisting output failed before upsert"
        );
    }

    #[test]
    fn reuse_existing_link_db_failure_is_plain_db_failure() {
        let temp = tempdir().expect("temp dir");
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate");
        let link = temp.path().join("link.txt");
        let target = temp.path().join("target.txt");
        std::fs::write(&target, "payload").expect("write target");
        symlink::create_link(&target, &link).expect("create existing link");
        conn.execute_batch("PRAGMA query_only = ON;")
            .expect("make db readonly");

        let mut decisions = TestDecisions;
        let mut out = Vec::new();
        let err = run_operation(
            &conn,
            LinkOperation::Add,
            &link,
            &target,
            &mut decisions,
            &mut out,
        )
        .expect_err("db failure without filesystem change should stay plain");

        assert!(
            matches!(err, SymmError::DbError { .. }),
            "unexpected error: {err:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&link).expect("read existing link"),
            "payload"
        );
    }
}
