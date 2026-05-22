use crate::adapters::db::repository;
use crate::domain::error::SymmError;
use crate::gui::env::{with_add_policies, with_env_vars};
use crate::gui::state::{AddConflictPolicy, AddLockPolicy, LinkSnapshot};
use crate::gui::util::VecWriter;
use crate::workflows::list_views;
use crate::workflows::rm::workflow::{self, RemoveMode};
use rusqlite::Connection;
use std::path::Path;

pub struct DataStore {
    conn: Option<Connection>,
}

impl DataStore {
    pub fn new() -> Self {
        Self { conn: None }
    }

    pub fn invalidate(&mut self) {
        self.conn = None;
    }

    pub fn reload(&mut self) -> Result<LinkSnapshot, SymmError> {
        let conn = self.connection_mut()?;
        let collected = list_views::collect_all(conn, None, None, 0)?;
        Ok(LinkSnapshot {
            views: collected.items,
        })
    }

    pub fn add_link(
        &mut self,
        link: &Path,
        target: &Path,
        name: &str,
        lock: AddLockPolicy,
        conflict: AddConflictPolicy,
    ) -> Result<String, SymmError> {
        let conn = self.connection_mut()?;
        let mut writer = VecWriter(Vec::new());
        with_add_policies(name, lock, conflict, || {
            crate::workflows::add::workflow::run_named(conn, link, target, Some(name), &mut writer)
        })?;
        Ok(writer.into_log())
    }

    pub fn remove_links(
        &mut self,
        selectors: &[String],
        mode: RemoveMode,
    ) -> Result<String, SymmError> {
        if selectors.is_empty() {
            return Ok(String::new());
        }
        let conn = self.connection_mut()?;
        let mut writer = VecWriter(Vec::new());
        let action = match mode {
            RemoveMode::DeleteLinkOnly => "delete",
            RemoveMode::RestoreTargetToLink => "restore",
        };
        with_env_vars(&[("SYMM_RM_ACTION", action)], || {
            workflow::run_with_mode(conn, selectors, mode, &mut writer)
        })?;
        Ok(writer.into_log())
    }

    fn connection_mut(&mut self) -> Result<&mut Connection, SymmError> {
        if self.conn.is_none() {
            self.conn = Some(repository::open_db()?);
        }
        Ok(self.conn.as_mut().expect("connection"))
    }
}
