use crate::domain::model::{LinkKind, LinkStatus, LinkView};
use crate::workflows::rm::workflow::RemoveMode;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainView {
    #[default]
    Detail,
    List,
    Add,
}

pub use crate::gui::theme::ThemePreference;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddConflictPolicy {
    #[default]
    KeepLink,
    KeepTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddLockPolicy {
    #[default]
    Unlock,
    Cancel,
}

#[derive(Debug, Default)]
pub struct AddForm {
    pub link_path: String,
    pub target_path: String,
    pub name: String,
    pub conflict_policy: AddConflictPolicy,
    pub lock_policy: AddLockPolicy,
    pub status_message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RmDialog {
    pub selector: String,
    pub display_name: String,
    pub mode: RemoveMode,
}

#[derive(Debug)]
pub struct AppState {
    pub search: String,
    pub selected_id: Option<i64>,
    pub expanded_ids: HashSet<i64>,
    pub main_view: MainView,
    pub sidebar_width: f32,
    pub toast: Option<String>,
    pub data_home: Option<PathBuf>,
    pub db_error: Option<String>,
    pub theme: ThemePreference,
    pub locale: String,
    pub settings_open: bool,
    pub add_form: AddForm,
    pub rm_dialog: Option<RmDialog>,
    pub busy: bool,
}

#[derive(Debug, Default)]
pub struct LinkSnapshot {
    pub views: Vec<LinkView>,
    pub scanned: usize,
}

impl LinkSnapshot {
    pub fn total(&self) -> usize {
        self.scanned
    }

    pub fn ok_count(&self) -> usize {
        self.views
            .iter()
            .filter(|v| v.status == LinkStatus::Ok)
            .count()
    }

    pub fn kind_counts(&self) -> (usize, usize) {
        let mut symlink = 0usize;
        let mut junction = 0usize;
        for v in &self.views {
            match v.link_kind {
                LinkKind::Symlink => symlink += 1,
                LinkKind::Junction => junction += 1,
            }
        }
        (symlink, junction)
    }

    pub fn filtered<'a>(&'a self, search: &str) -> Vec<&'a LinkView> {
        let q = search.trim().to_lowercase();
        let mut out: Vec<&LinkView> = self
            .views
            .iter()
            .filter(|v| {
                if q.is_empty() {
                    return true;
                }
                v.display_name().to_lowercase().contains(&q)
                    || v.link_path.to_lowercase().contains(&q)
                    || v.target_path.to_lowercase().contains(&q)
            })
            .collect();
        out.sort_by_key(|v| v.display_name());
        out
    }

    pub fn selected_view(&self, id: Option<i64>) -> Option<&LinkView> {
        let id = id?;
        self.views.iter().find(|v| v.id == id)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            search: String::new(),
            selected_id: None,
            expanded_ids: HashSet::new(),
            main_view: MainView::Detail,
            sidebar_width: 280.0,
            toast: None,
            data_home: None,
            db_error: None,
            theme: ThemePreference::System,
            locale: crate::domain::gui_settings::GuiSettings::default().locale,
            settings_open: false,
            add_form: AddForm::default(),
            rm_dialog: None,
            busy: false,
        }
    }
}
