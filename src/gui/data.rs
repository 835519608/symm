use crate::adapters::db::link_store;
use crate::domain::error::SymmError;
use crate::gui::state::{AddLockPolicy, LinkSnapshot};
use crate::gui::util::VecWriter;
use crate::workflows::add::workflow::{AddDecisionProvider, AddLockChoice, LinkOperation};
use crate::workflows::list_views;
use crate::workflows::rm::workflow::{self, RemoveMode};
use std::path::Path;

pub fn reload() -> Result<LinkSnapshot, SymmError> {
    let conn = link_store::open()?;
    let collected = list_views::collect_all(&conn, None, None, 0)?;
    Ok(LinkSnapshot::new(collected.items))
}

pub fn add_link(
    operation: LinkOperation,
    link: &Path,
    target: &Path,
    name: &str,
    lock: AddLockPolicy,
) -> Result<String, SymmError> {
    let conn = link_store::open()?;
    let mut writer = VecWriter(Vec::new());
    let mut decisions = GuiAddDecisions { name, lock };
    crate::workflows::add::workflow::run_operation(
        &conn,
        operation,
        link,
        target,
        &mut decisions,
        &mut writer,
    )?;
    Ok(writer.into_log())
}

pub fn remove_links(ids: &[i64], mode: RemoveMode) -> Result<String, SymmError> {
    if ids.is_empty() {
        return Ok(String::new());
    }
    let conn = link_store::open()?;
    let mut writer = VecWriter(Vec::new());
    match mode {
        RemoveMode::DeleteLinkOnly => workflow::run_rm_by_ids(&conn, ids, &mut writer)?,
        RemoveMode::RestoreTargetToLink => workflow::run_restore_by_ids(&conn, ids, &mut writer)?,
    }
    Ok(writer.into_log())
}

struct GuiAddDecisions<'a> {
    name: &'a str,
    lock: AddLockPolicy,
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
}
