use crate::adapters::db::link_store;
use crate::domain::error::SymmError;
use crate::gui::state::{AddConflictPolicy, AddLockPolicy, LinkSnapshot};
use crate::gui::util::VecWriter;
use crate::workflows::add::workflow::{
    AddConflictChoice, AddDecisionProvider, AddLockChoice, AddSymlinkConflictChoice,
};
use crate::workflows::list_views;
use crate::workflows::rm::workflow::{self, RemoveMode};
use std::path::Path;

pub fn reload() -> Result<LinkSnapshot, SymmError> {
    let conn = link_store::open()?;
    let collected = list_views::collect_all(&conn, None, None, 0)?;
    Ok(LinkSnapshot::new(collected.items))
}

pub fn add_link(
    link: &Path,
    target: &Path,
    name: &str,
    lock: AddLockPolicy,
    conflict: AddConflictPolicy,
) -> Result<String, SymmError> {
    let conn = link_store::open()?;
    let mut writer = VecWriter(Vec::new());
    let mut decisions = GuiAddDecisions {
        name,
        lock,
        conflict,
    };
    crate::workflows::add::workflow::run_with_decisions(
        &conn,
        link,
        target,
        &mut decisions,
        &mut writer,
    )?;
    Ok(writer.into_log())
}

pub fn remove_links(selectors: &[String], mode: RemoveMode) -> Result<String, SymmError> {
    if selectors.is_empty() {
        return Ok(String::new());
    }
    let conn = link_store::open()?;
    let mut writer = VecWriter(Vec::new());
    workflow::run_with_mode(&conn, selectors, mode, &mut writer)?;
    Ok(writer.into_log())
}

struct GuiAddDecisions<'a> {
    name: &'a str,
    lock: AddLockPolicy,
    conflict: AddConflictPolicy,
}

impl AddDecisionProvider for GuiAddDecisions<'_> {
    fn name(&mut self, _default_name: &str) -> Result<String, SymmError> {
        Ok(self.name.trim().to_string())
    }

    fn lock_choice(
        &mut self,
        _procs: &[crate::adapters::lock::ProcInfo],
    ) -> Result<AddLockChoice, SymmError> {
        Ok(match self.lock {
            AddLockPolicy::Unlock => AddLockChoice::Unlock,
            AddLockPolicy::Cancel => AddLockChoice::Cancel,
        })
    }

    fn conflict_choice(&mut self) -> Result<AddConflictChoice, SymmError> {
        Ok(match self.conflict {
            AddConflictPolicy::KeepLink => AddConflictChoice::KeepLink,
            AddConflictPolicy::KeepTarget => AddConflictChoice::KeepTarget,
        })
    }

    fn symlink_conflict_choice(&mut self) -> Result<AddSymlinkConflictChoice, SymmError> {
        Ok(AddSymlinkConflictChoice::Retarget)
    }
}
