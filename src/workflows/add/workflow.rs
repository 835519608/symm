use crate::adapters::db::{LinkQuery, repository};
use crate::adapters::paths::runtime_paths;
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use crate::ui::progress::migration_reporter::{MigrationProgressReporter, WorkflowProgressEvent};
use crate::workflows::add::{adopt, lock_gate, paths};
use crate::workflows::perf;
use inquire::Text;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddConflictChoice {
    KeepLink,
    KeepTarget,
    Cancel,
}

impl From<AddConflictChoice> for adopt::ConflictChoice {
    fn from(value: AddConflictChoice) -> Self {
        match value {
            AddConflictChoice::KeepLink => Self::KeepLink,
            AddConflictChoice::KeepTarget => Self::KeepTarget,
            AddConflictChoice::Cancel => Self::Cancel,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddSymlinkConflictChoice {
    Retarget,
    Cancel,
}

impl From<AddSymlinkConflictChoice> for adopt::SymlinkConflictChoice {
    fn from(value: AddSymlinkConflictChoice) -> Self {
        match value {
            AddSymlinkConflictChoice::Retarget => Self::Retarget,
            AddSymlinkConflictChoice::Cancel => Self::Cancel,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AddWorkflowOptions<'a> {
    pub name: Option<&'a str>,
    pub lock_choice: Option<AddLockChoice>,
    pub conflict_choice: Option<AddConflictChoice>,
    pub symlink_conflict_choice: Option<AddSymlinkConflictChoice>,
}

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    link: Option<&Path>,
    target: Option<&Path>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let (link, target) = paths::resolve_add_paths(conn, link, target)?;
    execute_add(conn, &link, &target, AddWorkflowOptions::default(), writer)?;
    perf::log_perf(
        "add",
        started.elapsed(),
        &[
            ("link_path", link.display().to_string()),
            ("target_path", target.display().to_string()),
        ],
    );
    Ok(())
}

/// GUI / 脚本：已给定 link、target，可选名称，不走交互式路径与名称提示。
pub fn run_named<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    name: Option<&str>,
    writer: &mut W,
) -> Result<(), SymmError> {
    run_with_options(
        conn,
        link,
        target,
        AddWorkflowOptions {
            name,
            ..Default::default()
        },
        writer,
    )
}

pub fn run_with_options<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    options: AddWorkflowOptions<'_>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    execute_add(conn, link, target, options, writer)?;
    let link_norm = runtime_paths::normalize_link(link);
    let target_norm = runtime_paths::normalize_target(target)?;
    perf::log_perf(
        "add",
        started.elapsed(),
        &[("link_path", link_norm), ("target_path", target_norm)],
    );
    Ok(())
}

fn execute_add<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    options: AddWorkflowOptions<'_>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let link_norm = runtime_paths::normalize_link(link);
    let existing = repository::find_optional(conn, &LinkQuery::link_path_exact(&link_norm))?;
    let mut reporter = MigrationProgressReporter::new(writer);
    lock_gate::ensure_link_not_locked_with_choice(
        Path::new(&link_norm),
        &mut reporter,
        options.lock_choice.map(Into::into),
    )?;
    let prep = adopt::resolve_add_conflict_with_choices(
        Path::new(&link_norm),
        target,
        &mut |event| reporter.handle_migration_event(event),
        options.conflict_choice.map(Into::into),
        options.symlink_conflict_choice.map(Into::into),
    )?;

    let target_norm = if prep.skip_target_exists_check {
        runtime_paths::normalize_target_known_exists(target)?
    } else {
        runtime_paths::normalize_target(target)?
    };
    let link_kind = if prep.link_exists_at_path {
        existing
            .as_ref()
            .map(|r| r.link_kind)
            .unwrap_or(LinkKind::Symlink)
    } else {
        reporter.handle_workflow_event(WorkflowProgressEvent::CreatingLink {
            link: link_norm.clone(),
            target: target_norm.clone(),
        })?;
        symlink::create_link(Path::new(&target_norm), Path::new(&link_norm))?
    };

    let default_name = existing.as_ref().map(|r| r.name.as_str()).unwrap_or("");
    let name_input = resolve_add_name(default_name, options.name)?;
    reporter.handle_workflow_event(WorkflowProgressEvent::PersistingDb {
        link: link_norm.clone(),
    })?;
    let name = repository::insert_link(conn, &name_input, &link_norm, &target_norm, link_kind)?;
    if name_input != name && !name_input.is_empty() {
        reporter.write_line(&format!(
            "名称「{name_input}」已改为「{name}」（纯数字名称会自动加前缀，避免与序号查询混淆）"
        ))?;
    }
    reporter.handle_workflow_event(WorkflowProgressEvent::Done {
        link: link_norm.clone(),
    })?;
    let display_name = if name.is_empty() {
        "(空)"
    } else {
        name.as_str()
    };
    reporter.write_line(&format!("已添加：{link_norm}（名称：{display_name}）"))?;
    Ok(())
}

fn resolve_add_name(default_name: &str, explicit: Option<&str>) -> Result<String, SymmError> {
    if let Some(name) = explicit {
        return Ok(name.trim().to_string());
    }
    if let Ok(v) = std::env::var("SYMM_ADD_NAME") {
        return Ok(v.trim().to_string());
    }
    Text::new("名称（可选，回车沿用默认）:")
        .with_default(default_name)
        .prompt()
        .map(|s| s.trim().to_string())
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })
}
