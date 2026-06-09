use crate::adapters::db::link_store;
use crate::adapters::status;
use crate::domain::error::SymmError;
use crate::domain::gui_settings::{GuiSettings, data_dir_from_settings};
use crate::domain::model::{LinkRecord, LinkView};
use crate::gui::settings_store;
use crate::gui::state::{LinkOpLockPolicy, LinkSnapshot};
use crate::gui::tasks::SettingsApplyOutcome;
use crate::gui::util::VecWriter;
use crate::workflows::link_ops::workflow::{
    LinkOpDecisionProvider, LinkOpLockChoice, LinkOperation,
};
use crate::workflows::rm::workflow::{self, RemoveMode};
use std::collections::HashSet;
use std::path::Path;

pub struct ReloadedLinks {
    pub snapshot: LinkSnapshot,
    pub selected_view: Option<LinkView>,
    pub all_ids: HashSet<i64>,
    pub page_index: u32,
}

pub fn reload(
    search: &str,
    page_index: u32,
    page_size: u32,
    selected_id: Option<i64>,
    checked_ids: &[i64],
) -> Result<ReloadedLinks, SymmError> {
    let conn = link_store::open()?;
    let kind_counts = link_store::count_kinds(&conn)?;
    let total_count = kind_counts.0 + kind_counts.1;
    let matched_count = if search.trim().is_empty() {
        total_count
    } else {
        link_store::count_matching(&conn, search)?
    };
    let page_size = page_size.max(1);
    let page_count = (matched_count.div_ceil(page_size as usize)).max(1) as u32;
    let page_index = page_index.min(page_count.saturating_sub(1));
    let offset = page_index as usize * page_size as usize;
    let items = link_store::list_matching_paginated(&conn, search, page_size, offset as u32)?
        .into_iter()
        .map(|(index, record)| view_for_page_record(index, &record))
        .collect::<Result<Vec<_>, _>>()?;
    let selected_view = selected_id
        .map(|id| selected_view_from_page_or_db(&conn, id, &items))
        .transpose()?
        .flatten();
    let mut ids_to_check = checked_ids.to_vec();
    if let Some(id) = selected_id {
        ids_to_check.push(id);
    }
    Ok(ReloadedLinks {
        snapshot: LinkSnapshot::with_counts(items, total_count, matched_count, kind_counts),
        selected_view,
        all_ids: link_store::existing_ids(&conn, &ids_to_check)?,
        page_index,
    })
}

pub fn apply_settings(
    settings: GuiSettings,
    previous_data_dir: String,
    data_dir_changed: bool,
    page_size: u32,
) -> Result<SettingsApplyOutcome, String> {
    let mut snapshot = None;
    if data_dir_changed {
        let new_data_dir = data_dir_from_settings(&settings);
        settings_store::apply_data_dir(&new_data_dir)?;
        match reload("", 0, page_size.max(1), None, &[]) {
            Ok(reloaded) => snapshot = Some(reloaded.snapshot),
            Err(err) => {
                settings_store::restore_data_dir(&previous_data_dir);
                return Err(err.to_string());
            }
        }
    }

    let save_error = settings_store::save(&settings)
        .err()
        .map(|err| err.to_string());

    Ok(SettingsApplyOutcome {
        settings,
        snapshot,
        data_dir_changed,
        save_error,
    })
}

fn view_for_page_record(index: u32, record: &LinkRecord) -> Result<LinkView, SymmError> {
    let mut view = status::to_view(record.clone());
    view.index = index;
    Ok(view)
}

fn selected_view_from_page_or_db(
    conn: &rusqlite::Connection,
    id: i64,
    page_items: &[LinkView],
) -> Result<Option<LinkView>, SymmError> {
    if let Some(view) = page_items.iter().find(|view| view.id == id) {
        return Ok(Some(view.clone()));
    }
    let Some(record) = link_store::find_by_id_optional(conn, id)? else {
        return Ok(None);
    };
    let index = link_store::index_for_id(conn, id)?.unwrap_or(1);
    view_for_page_record(index, &record).map(Some)
}

pub fn apply_link_op(
    operation: LinkOperation,
    link: &Path,
    target: &Path,
    name: &str,
    lock: LinkOpLockPolicy,
) -> Result<String, SymmError> {
    let conn = link_store::open()?;
    let mut writer = VecWriter(Vec::new());
    let mut decisions = GuiLinkOpDecisions { name, lock };
    crate::workflows::link_ops::workflow::run_operation(
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

struct GuiLinkOpDecisions<'a> {
    name: &'a str,
    lock: LinkOpLockPolicy,
}

impl LinkOpDecisionProvider for GuiLinkOpDecisions<'_> {
    fn name(&mut self, default_name: &str) -> Result<String, SymmError> {
        let name = self.name.trim();
        if name.is_empty() {
            Ok(default_name.to_string())
        } else {
            Ok(name.to_string())
        }
    }

    fn lock_choice(
        &mut self,
        _procs: &[crate::adapters::lock::ProcInfo],
    ) -> Result<LinkOpLockChoice, SymmError> {
        Ok(match self.lock {
            LinkOpLockPolicy::Unlock => LinkOpLockChoice::Unlock,
            LinkOpLockPolicy::Cancel => LinkOpLockChoice::Cancel,
        })
    }
}
