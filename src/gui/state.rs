use crate::domain::gui_settings::{ColorScheme, FONT_SIZE_PT_DEFAULT, GuiSettings, Locale};
use crate::domain::model::{LinkKind, LinkView};
use crate::gui::i18n::GuiTexts;
use crate::workflows::link_ops::workflow::LinkOperation;
use crate::workflows::rm::workflow::RemoveMode;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub use crate::gui::theme::ThemePreference;
pub const PAGE_SIZE_OPTIONS: [u32; 3] = [50, 100, 200];
pub const DEFAULT_PAGE_SIZE: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkOpLockPolicy {
    #[default]
    Unlock,
    Cancel,
}

#[derive(Debug, Default)]
pub struct LinkOpForm {
    pub operation: LinkOperation,
    pub link_path: String,
    pub target_path: String,
    pub name: String,
    pub lock_policy: LinkOpLockPolicy,
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
    pub theme: ThemePreference,
    pub locale: Locale,
    pub color_scheme: ColorScheme,
    pub font_size_pt: f32,
    pub sidebar_width: f32,
    pub data_dir: String,
}

impl SettingsDraft {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            section: SettingsSection::Appearance,
            theme: state.theme,
            locale: state.locale,
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
            theme: d.theme,
            locale: d.locale,
            color_scheme: d.color_scheme,
            font_size_pt: d.font_size_pt,
            sidebar_width: d.sidebar_width,
            data_dir: String::new(),
        }
    }

    pub fn restore_defaults(&mut self, preserve_data_dir: bool) {
        let section = self.section;
        let data_dir = self.data_dir.clone();
        *self = Self::appearance_defaults();
        self.section = section;
        if preserve_data_dir {
            self.data_dir = data_dir;
        }
    }
}

#[derive(Debug, Clone)]
pub struct RmDialog {
    pub ids: Vec<i64>,
    pub summary: String,
    pub mode: RemoveMode,
}

#[derive(Debug)]
pub struct AppState {
    pub search: String,
    pub selected_id: Option<i64>,
    pub checked_ids: HashSet<i64>,
    pub page_index: u32,
    pub page_size: u32,
    pub sidebar_width: f32,
    pub show_link_op_dialog: bool,
    /// 侧栏「已刷新」提示截止时间（与统计行同排右侧）。
    pub refresh_notice_until: Option<Instant>,
    pub toast: Option<String>,
    pub db_error: Option<String>,
    pub theme: ThemePreference,
    pub color_scheme: ColorScheme,
    pub locale: Locale,
    pub font_size_pt: f32,
    /// 当前生效的数据目录（空 = 默认）；应用时写入 `SYMM_HOME`。
    pub data_dir: String,
    /// 稳定设置中保存的数据目录；`SYMM_HOME` 运行时覆盖不会自动写入这里。
    pub persisted_data_dir: String,
    pub data_dir_runtime_override: bool,
    pub settings_draft: Option<SettingsDraft>,
    pub link_op_form: LinkOpForm,
    pub rm_dialog: Option<RmDialog>,
    pub busy: bool,
}

#[derive(Debug, Default)]
pub struct LinkSnapshot {
    pub views: Vec<LinkView>,
    display_names: Vec<String>,
    id_to_index: HashMap<i64, usize>,
    total_count: usize,
    matched_count: usize,
    kind_counts: (usize, usize),
}

impl LinkSnapshot {
    pub fn new(views: Vec<LinkView>) -> Self {
        let mut kind_counts = (0usize, 0usize);
        for view in &views {
            match view.link_kind {
                LinkKind::Symlink => kind_counts.0 += 1,
                LinkKind::Junction => kind_counts.1 += 1,
            }
        }
        Self::with_counts(
            views,
            kind_counts.0 + kind_counts.1,
            kind_counts.0 + kind_counts.1,
            kind_counts,
        )
    }

    pub fn with_counts(
        views: Vec<LinkView>,
        total_count: usize,
        matched_count: usize,
        kind_counts: (usize, usize),
    ) -> Self {
        let mut display_names = Vec::with_capacity(views.len());
        let mut id_to_index = HashMap::with_capacity(views.len());

        for (i, view) in views.iter().enumerate() {
            let display_name = display_name_for(view);
            display_names.push(display_name);
            id_to_index.insert(view.id, i);
        }

        Self {
            views,
            display_names,
            id_to_index,
            total_count,
            matched_count,
            kind_counts,
        }
    }

    pub fn total(&self) -> usize {
        self.total_count
    }

    pub fn matched_total(&self) -> usize {
        self.matched_count
    }

    pub fn page_count(&self, page_size: u32) -> u32 {
        let page_size = page_size.max(1) as usize;
        self.matched_count.div_ceil(page_size).max(1) as u32
    }

    /// (软链条数, 联接条数)
    pub fn kind_counts(&self) -> (usize, usize) {
        self.kind_counts
    }

    pub fn view_at(&self, index: usize) -> Option<&LinkView> {
        self.views.get(index)
    }

    pub fn display_name_at(&self, index: usize) -> Option<&str> {
        self.display_names.get(index).map(String::as_str)
    }

    pub fn view_by_id(&self, id: i64) -> Option<&LinkView> {
        self.id_to_index
            .get(&id)
            .and_then(|&index| self.views.get(index))
    }
}

fn display_name_for(view: &LinkView) -> String {
    view.display_name().into_owned()
}

impl AppState {
    pub fn texts(&self) -> GuiTexts {
        GuiTexts::new(self.locale)
    }

    pub fn refresh_notice_active(&self) -> bool {
        self.refresh_notice_until
            .is_some_and(|deadline| Instant::now() < deadline)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            search: String::new(),
            selected_id: None,
            checked_ids: HashSet::new(),
            page_index: 0,
            page_size: DEFAULT_PAGE_SIZE,
            sidebar_width: crate::gui::theme::SIDEBAR_DEFAULT_WIDTH,
            show_link_op_dialog: false,
            refresh_notice_until: None,
            toast: None,
            db_error: None,
            theme: ThemePreference::System,
            color_scheme: ColorScheme::default(),
            locale: Locale::default(),
            font_size_pt: FONT_SIZE_PT_DEFAULT,
            data_dir: String::new(),
            persisted_data_dir: String::new(),
            data_dir_runtime_override: false,
            settings_draft: None,
            link_op_form: LinkOpForm::default(),
            rm_dialog: None,
            busy: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_draft_shows_active_data_dir_when_runtime_override_is_active() {
        let state = AppState {
            data_dir: "/tmp/symm-env".to_string(),
            persisted_data_dir: "/tmp/symm-saved".to_string(),
            data_dir_runtime_override: true,
            ..AppState::default()
        };

        let draft = SettingsDraft::from_state(&state);

        assert_eq!(draft.data_dir, "/tmp/symm-env");
    }

    #[test]
    fn settings_draft_uses_active_data_dir_without_runtime_override() {
        let state = AppState {
            data_dir: "/tmp/symm-active".to_string(),
            persisted_data_dir: "/tmp/symm-saved".to_string(),
            data_dir_runtime_override: false,
            ..AppState::default()
        };

        let draft = SettingsDraft::from_state(&state);

        assert_eq!(draft.data_dir, "/tmp/symm-active");
    }

    #[test]
    fn settings_draft_restore_defaults_resets_all_editable_appearance_fields() {
        let mut draft = SettingsDraft {
            section: SettingsSection::About,
            theme: ThemePreference::Dark,
            locale: Locale::En,
            color_scheme: ColorScheme::Ember,
            font_size_pt: 22.0,
            sidebar_width: 260.0,
            data_dir: "/tmp/symm-custom".to_string(),
        };

        draft.restore_defaults(false);
        let defaults = SettingsDraft::appearance_defaults();

        assert_eq!(draft.section, SettingsSection::About);
        assert_eq!(draft.theme, defaults.theme);
        assert_eq!(draft.locale, defaults.locale);
        assert_eq!(draft.color_scheme, defaults.color_scheme);
        assert_eq!(draft.font_size_pt, defaults.font_size_pt);
        assert_eq!(draft.sidebar_width, defaults.sidebar_width);
        assert_eq!(draft.data_dir, "");
    }

    #[test]
    fn settings_draft_restore_defaults_can_preserve_runtime_data_dir() {
        let mut draft = SettingsDraft {
            section: SettingsSection::Appearance,
            theme: ThemePreference::Dark,
            locale: Locale::En,
            color_scheme: ColorScheme::Ember,
            font_size_pt: 22.0,
            sidebar_width: 260.0,
            data_dir: "/tmp/symm-env".to_string(),
        };

        draft.restore_defaults(true);

        assert_eq!(draft.data_dir, "/tmp/symm-env");
    }
}
