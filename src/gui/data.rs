use crate::adapters::db::repository;
use crate::domain::error::SymmError;
use crate::gui::state::{AddConflictPolicy, AddLockPolicy, LinkSnapshot};
use crate::gui::util::VecWriter;
use crate::workflows::add::workflow::{
    AddConflictChoice, AddLockChoice, AddSymlinkConflictChoice, AddWorkflowOptions,
};
use crate::workflows::list_views;
use crate::workflows::rm::workflow::{self, RemoveMode};
use std::path::Path;

pub fn reload() -> Result<LinkSnapshot, SymmError> {
    let conn = repository::open_db()?;
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
    let conn = repository::open_db()?;
    let mut writer = VecWriter(Vec::new());
    crate::workflows::add::workflow::run_with_options(
        &conn,
        link,
        target,
        AddWorkflowOptions {
            name: Some(name),
            lock_choice: Some(match lock {
                AddLockPolicy::Unlock => AddLockChoice::Unlock,
                AddLockPolicy::Cancel => AddLockChoice::Cancel,
            }),
            conflict_choice: Some(match conflict {
                AddConflictPolicy::KeepLink => AddConflictChoice::KeepLink,
                AddConflictPolicy::KeepTarget => AddConflictChoice::KeepTarget,
            }),
            symlink_conflict_choice: Some(AddSymlinkConflictChoice::Retarget),
        },
        &mut writer,
    )?;
    Ok(writer.into_log())
}

pub fn remove_links(selectors: &[String], mode: RemoveMode) -> Result<String, SymmError> {
    if selectors.is_empty() {
        return Ok(String::new());
    }
    let conn = repository::open_db()?;
    let mut writer = VecWriter(Vec::new());
    workflow::run_with_mode(&conn, selectors, mode, &mut writer)?;
    Ok(writer.into_log())
}
