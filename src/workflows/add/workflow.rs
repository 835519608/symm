use crate::adapters::db::link_store;
use crate::adapters::lock::ProcInfo;
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

pub trait AddDecisionProvider {
    fn name(&mut self, default_name: &str) -> Result<String, SymmError>;
    fn lock_choice(&mut self, procs: &[ProcInfo]) -> Result<AddLockChoice, SymmError>;
    fn conflict_choice(&mut self) -> Result<AddConflictChoice, SymmError>;
    fn symlink_conflict_choice(&mut self) -> Result<AddSymlinkConflictChoice, SymmError>;
}

pub fn run_with_decisions<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    decisions: &mut impl AddDecisionProvider,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    execute_add(conn, link, target, decisions, writer)?;
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
    decisions: &mut impl AddDecisionProvider,
    writer: &mut W,
) -> Result<(), SymmError> {
    let link_norm = runtime_paths::normalize_link(link);
    let existing = link_store::find_by_link_path(conn, &link_norm)?;
    let mut reporter = MigrationProgressReporter::new(writer);
    lock_gate::ensure_link_not_locked_with_choice(
        Path::new(&link_norm),
        &mut reporter,
        &mut |procs| decisions.lock_choice(procs).map(Into::into),
    )?;
    let prep = adopt::resolve_add_conflict_with_choices(
        Path::new(&link_norm),
        target,
        &mut |event| reporter.handle_migration_event(event),
        &mut AdoptDecisionAdapter(decisions),
    )?;

    let target_norm = if prep.skip_target_exists_check {
        runtime_paths::normalize_target_known_exists(target)?
    } else {
        runtime_paths::normalize_target(target)?
    };
    let link_kind = if prep.link_exists_at_path {
        prep.existing_link_kind
            .or_else(|| existing.as_ref().map(|r| r.link_kind))
            .unwrap_or(LinkKind::Symlink)
    } else {
        reporter.handle_workflow_event(WorkflowProgressEvent::CreatingLink {
            link: link_norm.clone(),
            target: target_norm.clone(),
        })?;
        symlink::create_link(Path::new(&target_norm), Path::new(&link_norm))?
    };

    let default_name = existing.as_ref().map(|r| r.name.as_str()).unwrap_or("");
    let name_input = decisions.name(default_name)?;
    reporter.handle_workflow_event(WorkflowProgressEvent::PersistingDb {
        link: link_norm.clone(),
    })?;
    let name = link_store::upsert_link(conn, &name_input, &link_norm, &target_norm, link_kind)?;
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

struct AdoptDecisionAdapter<'a, D>(&'a mut D);

impl<D> adopt::ConflictDecisionProvider for AdoptDecisionAdapter<'_, D>
where
    D: AddDecisionProvider,
{
    fn conflict_choice(&mut self) -> Result<adopt::ConflictChoice, SymmError> {
        self.0.conflict_choice().map(Into::into)
    }

    fn symlink_conflict_choice(&mut self) -> Result<adopt::SymlinkConflictChoice, SymmError> {
        self.0.symlink_conflict_choice().map(Into::into)
    }
}
