use crate::domain::gui_settings::{ColorScheme, FONT_SIZE_PT_DEFAULT, GuiSettings, Locale};
use crate::domain::model::{LinkKind, LinkView};
use crate::gui::i18n::GuiTexts;
use crate::workflows::rm::workflow::RemoveMode;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainView {
    #[default]
    Detail,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    Appearance,
    About,
}

#[derive(Debug, Clone)]
pub struct SettingsDraft {
    pub section: SettingsSection,
    pub color_scheme: ColorScheme,
    pub font_size_pt: f32,
    pub sidebar_width: f32,
    pub data_dir: String,
}

impl SettingsDraft {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            section: SettingsSection::Appearance,
            color_scheme: state.color_scheme,
            font_size_pt: state.font_size_pt,
            sidebar_width: state.sidebar_width,
            data_dir: state.data_dir.clone(),
        }
    }

    pub fn appearance_defaults() -> Self {
        let d = GuiSettings::default();
        Self {
            section: SettingsSection::Appearance,
            color_scheme: d.color_scheme,
            font_size_pt: d.font_size_pt,
            sidebar_width: d.sidebar_width,
            data_dir: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RmDialog {
    pub selectors: Vec<String>,
    pub summary: String,
    pub mode: RemoveMode,
}

#[derive(Debug)]
pub struct AppState {
    pub search: String,
    pub selected_id: Option<i64>,
    pub checked_ids: HashSet<i64>,
    pub main_view: MainView,
    pub sidebar_width: f32,
    pub toast: Option<String>,
    pub db_error: Option<String>,
    pub theme: ThemePreference,
    pub color_scheme: ColorScheme,
    pub locale: Locale,
    pub font_size_pt: f32,
    /// 持久化数据目录（空 = 默认）；应用时写入 `SYMM_HOME`。
    pub data_dir: String,
    pub settings_draft: Option<SettingsDraft>,
    pub add_form: AddForm,
    pub rm_dialog: Option<RmDialog>,
    pub busy: bool,
}

#[derive(Debug, Default)]
pub struct LinkSnapshot {
    pub views: Vec<LinkView>,
}

impl LinkSnapshot {
    pub fn total(&self) -> usize {
        self.views.len()
    }

    /// (软链条数, 联接条数)
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

    pub fn filtered_by_name<'a>(&'a self, search: &str) -> Vec<&'a LinkView> {
        let q = search.trim().to_lowercase();
        let mut out: Vec<&LinkView> = self
            .views
            .iter()
            .filter(|v| {
                if q.is_empty() {
                    return true;
                }
                v.display_name().to_lowercase().contains(&q)
                    || (!v.name.is_empty() && v.name.to_lowercase().contains(&q))
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

impl AppState {
    pub fn texts(&self) -> GuiTexts {
        GuiTexts::new(self.locale)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            search: String::new(),
            selected_id: None,
            checked_ids: HashSet::new(),
            main_view: MainView::Detail,
            sidebar_width: crate::gui::theme::SIDEBAR_DEFAULT_WIDTH,
            toast: None,
            db_error: None,
            theme: ThemePreference::System,
            color_scheme: ColorScheme::default(),
            locale: Locale::default(),
            font_size_pt: FONT_SIZE_PT_DEFAULT,
            data_dir: String::new(),
            settings_draft: None,
            add_form: AddForm::default(),
            rm_dialog: None,
            busy: false,
        }
    }
}
