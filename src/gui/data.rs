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

pub struct RemoveOutcome {
    pub log: String,
    pub remaining_ids: HashSet<i64>,
    pub error: Option<String>,
}

pub fn reload(
    data_dir: &Path,
    search: &str,
    page_index: u32,
    page_size: u32,
    selected_id: Option<i64>,
    checked_ids: &[i64],
) -> Result<ReloadedLinks, SymmError> {
    let conn = link_store::open_at(data_dir)?;
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
        .map(|(index, record)| view_for_page_record(index, record))
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
    data_dir_changed: bool,
    page_size: u32,
) -> Result<SettingsApplyOutcome, String> {
    apply_settings_with_saver(settings, data_dir_changed, page_size, settings_store::save)
}

fn apply_settings_with_saver(
    settings: GuiSettings,
    data_dir_changed: bool,
    page_size: u32,
    save_settings: impl FnOnce(&GuiSettings) -> Result<(), SymmError>,
) -> Result<SettingsApplyOutcome, String> {
    let mut snapshot = None;
    if data_dir_changed {
        let new_data_dir = data_dir_from_settings(&settings);
        let resolved_data_dir = settings_store::resolve_data_dir(&new_data_dir)?;
        match reload(&resolved_data_dir, "", 0, page_size.max(1), None, &[]) {
            Ok(reloaded) => snapshot = Some(reloaded.snapshot),
            Err(err) => return Err(err.to_string()),
        }
    }

    let save_error = save_settings(&settings).err().map(|err| err.to_string());

    Ok(SettingsApplyOutcome {
        settings,
        snapshot,
        data_dir_changed,
        save_error,
    })
}

fn view_for_page_record(index: u32, record: LinkRecord) -> Result<LinkView, SymmError> {
    let mut view = status::to_view(record);
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
    view_for_page_record(index, record).map(Some)
}

pub fn apply_link_op(
    data_dir: &Path,
    operation: LinkOperation,
    link: &Path,
    target: &Path,
    name: &str,
    lock: LinkOpLockPolicy,
) -> Result<String, SymmError> {
    let conn = link_store::open_at(data_dir)?;
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

pub fn remove_links(
    data_dir: &Path,
    ids: &[i64],
    mode: RemoveMode,
) -> Result<RemoveOutcome, SymmError> {
    if ids.is_empty() {
        return Ok(RemoveOutcome {
            log: String::new(),
            remaining_ids: HashSet::new(),
            error: None,
        });
    }
    let conn = link_store::open_at(data_dir)?;
    let mut writer = VecWriter(Vec::new());
    let result = match mode {
        RemoveMode::DeleteLinkOnly => workflow::run_rm_by_ids(&conn, ids, &mut writer),
        RemoveMode::RestoreTargetToLink => workflow::run_restore_by_ids(&conn, ids, &mut writer),
    };
    let error = result.err().map(|err| err.to_string());
    let remaining_ids = if error.is_some() {
        link_store::existing_ids(&conn, ids)?
    } else {
        HashSet::new()
    };
    Ok(RemoveOutcome {
        log: writer.into_log(),
        remaining_ids,
        error,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::gui_settings::GuiSettings;
    use crate::domain::model::LinkKind;
    use tempfile::tempdir;

    fn insert_link(data_dir: &Path, name: &str) {
        let conn = link_store::open_at(data_dir).expect("open db");
        link_store::upsert_link(
            &conn,
            name,
            &format!("/tmp/{name}-link"),
            &format!("/tmp/{name}-target"),
            LinkKind::Symlink,
        )
        .expect("insert link");
    }

    #[test]
    fn reload_uses_explicit_data_dir() {
        let dir_a = tempdir().expect("dir a");
        let dir_b = tempdir().expect("dir b");
        insert_link(dir_a.path(), "a");
        insert_link(dir_b.path(), "b");

        let a = reload(dir_a.path(), "", 0, 100, None, &[]).expect("reload a");
        let b = reload(dir_b.path(), "", 0, 100, None, &[]).expect("reload b");

        assert_eq!(a.snapshot.total(), 1);
        assert_eq!(b.snapshot.total(), 1);
        assert_eq!(a.snapshot.views[0].name, "a");
        assert_eq!(b.snapshot.views[0].name, "b");
    }

    #[test]
    fn remove_uses_explicit_data_dir() {
        let dir_a = tempdir().expect("dir a");
        let dir_b = tempdir().expect("dir b");
        insert_link(dir_a.path(), "a");
        insert_link(dir_b.path(), "b");

        remove_links(dir_a.path(), &[1], RemoveMode::DeleteLinkOnly).expect("remove a");

        let a = reload(dir_a.path(), "", 0, 100, None, &[]).expect("reload a");
        let b = reload(dir_b.path(), "", 0, 100, None, &[]).expect("reload b");
        assert_eq!(a.snapshot.total(), 0);
        assert_eq!(b.snapshot.total(), 1);
        assert_eq!(b.snapshot.views[0].name, "b");
    }

    #[test]
    fn apply_settings_loads_new_data_dir_snapshot() {
        let dir = tempdir().expect("dir");
        insert_link(dir.path(), "new-dir-link");
        let settings = GuiSettings {
            data_dir: Some(dir.path().to_string_lossy().to_string()),
            ..GuiSettings::default()
        };

        let outcome =
            apply_settings_with_saver(settings, true, 100, |_| Ok(())).expect("apply settings");

        let snapshot = outcome.snapshot.expect("snapshot");
        assert!(outcome.data_dir_changed);
        assert_eq!(snapshot.total(), 1);
        assert_eq!(snapshot.views[0].name, "new-dir-link");
    }

    #[test]
    fn apply_settings_rejects_file_as_data_dir() {
        let dir = tempdir().expect("dir");
        let file = dir.path().join("not-a-dir");
        std::fs::write(&file, "x").expect("write file");
        let settings = GuiSettings {
            data_dir: Some(file.to_string_lossy().to_string()),
            ..GuiSettings::default()
        };

        let err = match apply_settings_with_saver(settings, true, 100, |_| Ok(())) {
            Ok(_) => panic!("file should fail"),
            Err(err) => err,
        };

        assert!(!err.is_empty());
    }
}
