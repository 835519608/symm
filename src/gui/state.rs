use crate::domain::gui_settings::{ColorScheme, FONT_SIZE_PT_DEFAULT, GuiSettings, Locale};
use crate::domain::model::{LinkKind, LinkView};
use crate::gui::i18n::GuiTexts;
use crate::workflows::rm::workflow::RemoveMode;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddSymlinkConflictPolicy {
    Retarget,
    #[default]
    Cancel,
}

#[derive(Debug, Default)]
pub struct AddForm {
    pub link_path: String,
    pub target_path: String,
    pub name: String,
    pub conflict_policy: AddConflictPolicy,
    pub symlink_conflict_policy: AddSymlinkConflictPolicy,
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
    pub sidebar_width: f32,
    pub show_add_dialog: bool,
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
    pub add_form: AddForm,
    pub rm_dialog: Option<RmDialog>,
    pub sidebar_filter: SidebarFilterCache,
    pub busy: bool,
}

#[derive(Debug, Default)]
pub struct SidebarFilterCache {
    search: String,
    indices: Vec<usize>,
    valid: bool,
}

impl SidebarFilterCache {
    pub fn clear(&mut self) {
        self.search.clear();
        self.indices.clear();
        self.valid = false;
    }

    pub fn refresh(&mut self, snapshot: &LinkSnapshot, search: &str) -> usize {
        if self.valid && self.search == search {
            return self.indices.len();
        }
        self.search.clear();
        self.search.push_str(search);
        snapshot.fill_filtered_indices(search, &mut self.indices);
        self.valid = true;
        self.indices.len()
    }

    pub fn index_at(&self, row: usize) -> Option<usize> {
        self.indices.get(row).copied()
    }
}

#[derive(Debug, Default)]
pub struct LinkSnapshot {
    pub views: Vec<LinkView>,
    display_names: Vec<String>,
    display_names_lower: Vec<String>,
    name_lower: Vec<String>,
    sorted_indices: Vec<usize>,
    id_to_index: HashMap<i64, usize>,
    kind_counts: (usize, usize),
}

impl LinkSnapshot {
    pub fn new(views: Vec<LinkView>) -> Self {
        let mut display_names = Vec::with_capacity(views.len());
        let mut display_names_lower = Vec::with_capacity(views.len());
        let mut name_lower = Vec::with_capacity(views.len());
        let mut id_to_index = HashMap::with_capacity(views.len());
        let mut kind_counts = (0usize, 0usize);

        for (i, view) in views.iter().enumerate() {
            let display_name = display_name_for(view);
            display_names_lower.push(display_name.to_lowercase());
            name_lower.push(view.name.to_lowercase());
            display_names.push(display_name);
            id_to_index.insert(view.id, i);
            match view.link_kind {
                LinkKind::Symlink => kind_counts.0 += 1,
                LinkKind::Junction => kind_counts.1 += 1,
            }
        }

        let mut sorted_indices: Vec<usize> = (0..views.len()).collect();
        sorted_indices.sort_by(|&a, &b| display_names[a].cmp(&display_names[b]));

        Self {
            views,
            display_names,
            display_names_lower,
            name_lower,
            sorted_indices,
            id_to_index,
            kind_counts,
        }
    }

    pub fn total(&self) -> usize {
        self.views.len()
    }

    /// (软链条数, 联接条数)
    pub fn kind_counts(&self) -> (usize, usize) {
        self.kind_counts
    }

    pub fn fill_filtered_indices(&self, search: &str, out: &mut Vec<usize>) {
        out.clear();
        let q = search.trim();
        if q.is_empty() {
            out.extend_from_slice(&self.sorted_indices);
            return;
        }
        let q = q.to_lowercase();
        out.extend(self.sorted_indices.iter().copied().filter(|&i| {
            self.display_names_lower[i].contains(&q)
                || (!self.name_lower[i].is_empty() && self.name_lower[i].contains(&q))
        }));
    }

    pub fn view_at(&self, index: usize) -> Option<&LinkView> {
        self.views.get(index)
    }

    pub fn display_name_at(&self, index: usize) -> Option<&str> {
        self.display_names.get(index).map(String::as_str)
    }

    pub fn selected_view(&self, id: Option<i64>) -> Option<&LinkView> {
        let id = id?;
        self.view_by_id(id)
    }

    pub fn view_by_id(&self, id: i64) -> Option<&LinkView> {
        self.id_to_index
            .get(&id)
            .and_then(|&index| self.views.get(index))
    }
}

fn display_name_for(view: &LinkView) -> String {
    if !view.name.is_empty() {
        return view.name.clone();
    }
    Path::new(&view.link_path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| view.link_path.clone())
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
            sidebar_width: crate::gui::theme::SIDEBAR_DEFAULT_WIDTH,
            show_add_dialog: false,
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
            add_form: AddForm::default(),
            rm_dialog: None,
            sidebar_filter: SidebarFilterCache::default(),
            busy: false,
        }
    }
}
