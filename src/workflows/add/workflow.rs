use crate::adapters::db::link_store;
use crate::adapters::lock::ProcInfo;
use crate::adapters::migrate;
use crate::adapters::paths::runtime_paths;
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use crate::ui::progress::migration_reporter::{MigrationProgressReporter, WorkflowProgressEvent};
use crate::workflows::add::{adopt, lock_gate};
use crate::workflows::perf;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddLockChoice {
    Unlock,
    Cancel,
}

impl From<AddLockChoice> for lock_gate::LockResolutionAction {
    fn from(value: AddLockChoice) -> Self {
        match value {
            AddLockChoice::Unlock => Self::UnlockAll,
            AddLockChoice::Cancel => Self::Cancel,
        }
    }
}

pub trait AddDecisionProvider {
    fn name(&mut self, default_name: &str) -> Result<String, SymmError>;
    fn lock_choice(&mut self, procs: &[ProcInfo]) -> Result<AddLockChoice, SymmError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub enum LinkOperation {
    #[default]
    Add,
    Adopt,
    Point,
}

pub fn run_add<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    decisions: &mut impl AddDecisionProvider,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let (link_norm, target_norm) = execute_add(conn, link, target, decisions, writer)?;
    perf::log_perf(
        "add",
        started.elapsed(),
        &[("link_path", link_norm), ("target_path", target_norm)],
    );
    Ok(())
}

pub fn run_adopt<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    decisions: &mut impl AddDecisionProvider,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let (link_norm, target_norm) = execute_adopt(conn, link, target, decisions, writer)?;
    perf::log_perf(
        "adopt",
        started.elapsed(),
        &[("link_path", link_norm), ("target_path", target_norm)],
    );
    Ok(())
}

pub fn run_point<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    decisions: &mut impl AddDecisionProvider,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let (link_norm, target_norm) = execute_point(conn, link, target, decisions, writer)?;
    perf::log_perf(
        "point",
        started.elapsed(),
        &[("link_path", link_norm), ("target_path", target_norm)],
    );
    Ok(())
}

#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn run_operation<W: Write>(
    conn: &rusqlite::Connection,
    operation: LinkOperation,
    link: &Path,
    target: &Path,
    decisions: &mut impl AddDecisionProvider,
    writer: &mut W,
) -> Result<(), SymmError> {
    match operation {
        LinkOperation::Add => run_add(conn, link, target, decisions, writer),
        LinkOperation::Adopt => run_adopt(conn, link, target, decisions, writer),
        LinkOperation::Point => run_point(conn, link, target, decisions, writer),
    }
}

fn execute_add<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    decisions: &mut impl AddDecisionProvider,
    writer: &mut W,
) -> Result<(String, String), SymmError> {
    let link_norm = runtime_paths::normalize_link(link);
    let existing = link_store::find_by_link_path(conn, &link_norm)?;
    let mut reporter = MigrationProgressReporter::new(writer);
    lock_gate::ensure_link_not_locked_with_choice(
        Path::new(&link_norm),
        &mut reporter,
        &mut |procs| decisions.lock_choice(procs).map(Into::into),
    )?;
    let target_norm = runtime_paths::normalize_target(target)?;
    let link_path = Path::new(&link_norm);
    let link_meta = std::fs::symlink_metadata(link_path).ok();
    let existing_link_kind = link_meta
        .as_ref()
        .and_then(|meta| symlink::kind_from_path_and_metadata(link_path, meta));

    let link_kind = match (link_meta, existing_link_kind) {
        (None, _) => {
            reporter.handle_workflow_event(WorkflowProgressEvent::CreatingLink {
                link: link_norm.clone(),
                target: target_norm.clone(),
            })?;
            symlink::create_link(Path::new(&target_norm), link_path)?
        }
        (Some(_), Some(kind)) => {
            if !adopt::symlink_points_to_target(link_path, Path::new(&target_norm))? {
                return Err(SymmError::InvalidArgument {
                    message: format!(
                        "link 路径已是指向其他 target 的链接，请使用 point：{}",
                        link_path.display()
                    ),
                });
            }
            kind
        }
        (Some(_), None) => {
            return Err(SymmError::InvalidArgument {
                message: format!(
                    "link 路径已被真实实体占用，请使用 adopt：{}",
                    link_path.display()
                ),
            });
        }
    };

    persist_record(
        conn,
        &mut reporter,
        decisions,
        PersistRecord {
            default_name: existing.as_ref().map(|r| r.name.as_str()).unwrap_or(""),
            link_norm: &link_norm,
            target_norm: &target_norm,
            link_kind,
            verb: "已添加",
        },
    )?;
    Ok((link_norm, target_norm))
}

fn execute_adopt<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    decisions: &mut impl AddDecisionProvider,
    writer: &mut W,
) -> Result<(String, String), SymmError> {
    let link_norm = runtime_paths::normalize_link(link);
    let existing = link_store::find_by_link_path(conn, &link_norm)?;
    let mut reporter = MigrationProgressReporter::new(writer);
    lock_gate::ensure_link_not_locked_with_choice(
        Path::new(&link_norm),
        &mut reporter,
        &mut |procs| decisions.lock_choice(procs).map(Into::into),
    )?;
    let link_path = Path::new(&link_norm);
    let link_meta = std::fs::symlink_metadata(link_path).map_err(|e| SymmError::IoError {
        message: format!("无法读取 link 路径：{e}"),
    })?;
    if symlink::kind_from_path_and_metadata(link_path, &link_meta).is_some() {
        return Err(SymmError::InvalidArgument {
            message: format!(
                "link 路径已是链接，请使用 add 或 point：{}",
                link_path.display()
            ),
        });
    }
    if std::fs::symlink_metadata(target).is_ok() {
        return Err(SymmError::InvalidArgument {
            message: format!("target 路径已存在，adopt 默认不替换：{}", target.display()),
        });
    }
    ensure_target_parent_dir(target)?;
    migrate::migrate_path(link_path, target, &mut |event| {
        reporter.handle_migration_event(event)
    })
    .map_err(|e| SymmError::IoError {
        message: format!("接管失败：无法把 link 实体迁到 target：{e}"),
    })?;
    let target_norm = runtime_paths::normalize_target_known_exists(target)?;
    reporter.handle_workflow_event(WorkflowProgressEvent::CreatingLink {
        link: link_norm.clone(),
        target: target_norm.clone(),
    })?;
    let link_kind = symlink::create_link(Path::new(&target_norm), link_path)?;
    persist_record(
        conn,
        &mut reporter,
        decisions,
        PersistRecord {
            default_name: existing.as_ref().map(|r| r.name.as_str()).unwrap_or(""),
            link_norm: &link_norm,
            target_norm: &target_norm,
            link_kind,
            verb: "已接管",
        },
    )?;
    Ok((link_norm, target_norm))
}

fn execute_point<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    decisions: &mut impl AddDecisionProvider,
    writer: &mut W,
) -> Result<(String, String), SymmError> {
    let link_norm = runtime_paths::normalize_link(link);
    let existing = link_store::find_by_link_path(conn, &link_norm)?;
    let target_norm = runtime_paths::normalize_target(target)?;
    let mut reporter = MigrationProgressReporter::new(writer);
    lock_gate::ensure_link_not_locked_with_choice(
        Path::new(&link_norm),
        &mut reporter,
        &mut |procs| decisions.lock_choice(procs).map(Into::into),
    )?;
    let link_path = Path::new(&link_norm);
    let meta = std::fs::symlink_metadata(link_path).map_err(|e| SymmError::IoError {
        message: format!("无法读取 link 路径：{e}"),
    })?;
    if symlink::kind_from_path_and_metadata(link_path, &meta).is_none() {
        return Err(SymmError::InvalidArgument {
            message: format!("link 路径不是链接，无法 point：{}", link_path.display()),
        });
    }
    symlink::unlink(link_path)?;
    reporter.handle_workflow_event(WorkflowProgressEvent::CreatingLink {
        link: link_norm.clone(),
        target: target_norm.clone(),
    })?;
    let link_kind = symlink::create_link(Path::new(&target_norm), link_path)?;
    persist_record(
        conn,
        &mut reporter,
        decisions,
        PersistRecord {
            default_name: existing.as_ref().map(|r| r.name.as_str()).unwrap_or(""),
            link_norm: &link_norm,
            target_norm: &target_norm,
            link_kind,
            verb: "已改指向",
        },
    )?;
    Ok((link_norm, target_norm))
}

struct PersistRecord<'a> {
    default_name: &'a str,
    link_norm: &'a str,
    target_norm: &'a str,
    link_kind: LinkKind,
    verb: &'a str,
}

fn persist_record<W: Write>(
    conn: &rusqlite::Connection,
    reporter: &mut MigrationProgressReporter<'_, W>,
    decisions: &mut impl AddDecisionProvider,
    input: PersistRecord<'_>,
) -> Result<(), SymmError> {
    reporter.handle_workflow_event(WorkflowProgressEvent::PersistingDb {
        link: input.link_norm.to_string(),
    })?;
    let name_input = decisions.name(input.default_name)?;
    let name = link_store::upsert_link(
        conn,
        &name_input,
        input.link_norm,
        input.target_norm,
        input.link_kind,
    )?;
    if name_input != name && !name_input.is_empty() {
        reporter.write_line(&format!(
            "名称「{name_input}」已改为「{name}」（纯数字名称会自动加前缀，避免与序号查询混淆）"
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
    Ok(())
}

fn ensure_target_parent_dir(target: &Path) -> Result<(), SymmError> {
    let parent = target.parent().ok_or_else(|| SymmError::InvalidArgument {
        message: format!("无法解析 target 路径的父目录：{}", target.display()),
    })?;
    if parent.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
        message: format!("adopt 失败：无法创建目录 {}：{e}", parent.display()),
    })
}
