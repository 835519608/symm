use crate::domain::gui_settings::{ColorScheme, Locale, ThemeMode};
use crate::domain::model::{LinkKind, LinkStatus};

/// 当前界面文案（按 `Locale` 选中文或英文）。
#[derive(Debug, Clone, Copy)]
pub struct GuiTexts {
    pub locale: Locale,
}

impl GuiTexts {
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    // --- 顶栏 ---
    pub fn add_link(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "添加链接",
            Locale::En => "Add link",
        }
    }

    pub fn add_link_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "创建新软链",
            Locale::En => "Create a new symlink",
        }
    }

    pub fn theme_tip(&self, theme_label: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("主题：{theme_label}（点击切换）"),
            Locale::En => format!("Theme: {theme_label} (click to cycle)"),
        }
    }

    pub fn locale_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "界面语言（点击切换中/英）",
            Locale::En => "UI language (click to toggle)",
        }
    }

    pub fn settings_open_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "设置",
            Locale::En => "Settings",
        }
    }

    pub fn settings_title(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "设置",
            Locale::En => "Settings",
        }
    }

    pub fn settings_nav_appearance(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "外观",
            Locale::En => "Appearance",
        }
    }

    pub fn settings_nav_about(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "关于",
            Locale::En => "About",
        }
    }

    pub fn settings_color_scheme(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "配色",
            Locale::En => "Color scheme",
        }
    }

    pub fn settings_font_size(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "字号",
            Locale::En => "Font size",
        }
    }

    pub fn settings_font_size_hint(&self, min: f32, max: f32) -> String {
        match self.locale {
            Locale::ZhCn => format!("正文字号 {min:.0}–{max:.0}px，标题与按钮按比例缩放"),
            Locale::En => {
                format!("Body text {min:.0}–{max:.0}px; headings and buttons scale proportionally")
            }
        }
    }

    pub fn settings_sidebar_width(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "侧栏宽度",
            Locale::En => "Sidebar width",
        }
    }

    pub fn settings_data_dir(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "数据目录",
            Locale::En => "Data directory",
        }
    }

    pub fn settings_data_dir_hint(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "留空使用默认可执行文件旁 data/",
            Locale::En => "Leave empty for default data/ next to executable",
        }
    }

    pub fn settings_data_dir_browse_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "选择数据目录",
            Locale::En => "Choose data directory",
        }
    }

    pub fn settings_data_dir_note(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "存放 symm.db 与 settings.json；应用后重新加载链接库",
            Locale::En => "Stores symm.db and settings.json; link list reloads after Apply",
        }
    }

    pub fn settings_restore_defaults(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "恢复默认",
            Locale::En => "Restore defaults",
        }
    }

    pub fn settings_restore_defaults_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "重置外观页：配色、字号、侧栏宽度、数据目录",
            Locale::En => "Reset appearance: color, font size, sidebar width, data directory",
        }
    }

    pub fn settings_close(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "关闭",
            Locale::En => "Close",
        }
    }

    pub fn settings_apply(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "应用",
            Locale::En => "Apply",
        }
    }

    pub fn settings_about_heading(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "symm",
            Locale::En => "symm",
        }
    }

    pub fn settings_about_tagline(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "跨平台软链管理工具",
            Locale::En => "Cross-platform symlink manager",
        }
    }

    pub fn settings_version_label(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "版本",
            Locale::En => "Version",
        }
    }

    pub fn theme_mode_label(&self, mode: ThemeMode) -> &'static str {
        match (self.locale, mode) {
            (Locale::ZhCn, ThemeMode::System) => "跟随系统",
            (Locale::ZhCn, ThemeMode::Light) => "浅色",
            (Locale::ZhCn, ThemeMode::Dark) => "深色",
            (Locale::En, ThemeMode::System) => "System",
            (Locale::En, ThemeMode::Light) => "Light",
            (Locale::En, ThemeMode::Dark) => "Dark",
        }
    }

    pub fn color_scheme_label(&self, scheme: ColorScheme) -> &'static str {
        match (self.locale, scheme) {
            (Locale::ZhCn, ColorScheme::Slate) => "中性",
            (Locale::ZhCn, ColorScheme::Ocean) => "海洋",
            (Locale::ZhCn, ColorScheme::Forest) => "森林",
            (Locale::ZhCn, ColorScheme::Violet) => "紫罗兰",
            (Locale::ZhCn, ColorScheme::Ember) => "暖色",
            (Locale::En, ColorScheme::Slate) => "Slate",
            (Locale::En, ColorScheme::Ocean) => "Ocean",
            (Locale::En, ColorScheme::Forest) => "Forest",
            (Locale::En, ColorScheme::Violet) => "Violet",
            (Locale::En, ColorScheme::Ember) => "Ember",
        }
    }

    // --- 侧栏 ---
    pub fn sidebar_title(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "链接库",
            Locale::En => "Links",
        }
    }

    pub fn refresh(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "重新加载",
            Locale::En => "Reload",
        }
    }

    pub fn sidebar_stats(&self, total: usize, symlink: usize, junction: usize) -> String {
        match self.locale {
            Locale::ZhCn => format!("共 {total} · 软链 {symlink} · 链接 {junction}"),
            Locale::En => format!("{total} total · {symlink} symlinks · {junction} junctions"),
        }
    }

    pub fn delete_selected(&self, n: usize) -> String {
        match self.locale {
            Locale::ZhCn => format!("删除 ({n})"),
            Locale::En => format!("Delete ({n})"),
        }
    }

    pub fn delete_selected_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "删除选中项",
            Locale::En => "Delete selected links",
        }
    }

    pub fn clear_selection(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "取消选择",
            Locale::En => "Clear selection",
        }
    }

    pub fn search_hint(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "搜索名称…",
            Locale::En => "Search by name…",
        }
    }

    pub fn no_links(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "暂无链接",
            Locale::En => "No links yet",
        }
    }

    pub fn no_match(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "无匹配",
            Locale::En => "No matches",
        }
    }

    pub fn delete_link_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "删除此链接",
            Locale::En => "Delete this link",
        }
    }

    // --- 添加页 ---
    pub fn add_heading(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "添加链接",
            Locale::En => "Add link",
        }
    }

    pub fn add_subtitle(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "与 CLI `symm add` 相同",
            Locale::En => "Same as CLI `symm add`",
        }
    }

    pub fn link_path_label(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "链接路径",
            Locale::En => "Link path",
        }
    }

    pub fn target_path_label(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "目标路径",
            Locale::En => "Target path",
        }
    }

    pub fn name_optional_label(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "名称（可选）",
            Locale::En => "Name (optional)",
        }
    }

    pub fn name_hint(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "留空则使用链接文件名",
            Locale::En => "Leave empty to use the link file name",
        }
    }

    pub fn browse(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "浏览",
            Locale::En => "Browse",
        }
    }

    pub fn browse_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "选择路径",
            Locale::En => "Pick a path",
        }
    }

    pub fn advanced_options(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "高级选项",
            Locale::En => "Advanced",
        }
    }

    pub fn lock_unlock(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "尝试结束占用进程并继续",
            Locale::En => "Try to close locking processes and continue",
        }
    }

    pub fn lock_cancel(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "取消（不结束进程）",
            Locale::En => "Cancel (do not kill processes)",
        }
    }

    pub fn conflict_keep_link(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "保留链接侧",
            Locale::En => "Keep link side",
        }
    }

    pub fn conflict_keep_target(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "保留目标侧",
            Locale::En => "Keep target side",
        }
    }

    pub fn create_link(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "创建链接",
            Locale::En => "Create link",
        }
    }

    pub fn create_link_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "写入数据库并创建软链",
            Locale::En => "Save to database and create symlink",
        }
    }

    pub fn clear_form(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "清空",
            Locale::En => "Clear",
        }
    }

    pub fn clear_form_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "清空表单",
            Locale::En => "Clear form",
        }
    }

    pub fn paths_required(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "请填写链接路径与目标路径",
            Locale::En => "Link path and target path are required",
        }
    }

    // --- 详情 ---
    pub fn select_link_hint(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "在左侧选择一条链接",
            Locale::En => "Select a link on the left",
        }
    }

    pub fn field_name(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "名称",
            Locale::En => "Name",
        }
    }

    pub fn field_kind(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "类型",
            Locale::En => "Kind",
        }
    }

    pub fn field_status(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "状态",
            Locale::En => "Status",
        }
    }

    pub fn field_link_path(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "链接路径",
            Locale::En => "Link path",
        }
    }

    pub fn field_target_path(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "目标路径",
            Locale::En => "Target path",
        }
    }

    pub fn field_index(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "序号",
            Locale::En => "Index",
        }
    }

    pub fn field_id(&self) -> &'static str {
        "ID"
    }

    // --- 删除对话框 ---
    pub fn rm_dialog_title(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "删除链接",
            Locale::En => "Delete link",
        }
    }

    pub fn rm_confirm_prompt(&self, summary: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("确定删除「{summary}」？"),
            Locale::En => format!("Delete “{summary}”?"),
        }
    }

    pub fn rm_batch_summary(&self, first: &str, n: usize) -> String {
        match self.locale {
            Locale::ZhCn => format!("{first} 等 {n} 条链接"),
            Locale::En => format!("{first} and {n} more links"),
        }
    }

    pub fn rm_mode_delete_only(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "只删除软链与数据库记录",
            Locale::En => "Remove symlink and database record only",
        }
    }

    pub fn rm_mode_restore(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "删除软链，并把目标移回链接位置",
            Locale::En => "Remove symlink and move target back to link path",
        }
    }

    pub fn confirm_delete(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "确认删除",
            Locale::En => "Confirm",
        }
    }

    pub fn cancel(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "取消",
            Locale::En => "Cancel",
        }
    }

    pub fn busy(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "处理中…",
            Locale::En => "Working…",
        }
    }

    pub fn db_open_failed(&self, err: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("无法打开数据库：{err}"),
            Locale::En => format!("Cannot open database: {err}"),
        }
    }

    pub fn select_before_delete(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "请先勾选要删除的链接",
            Locale::En => "Select links to delete first",
        }
    }

    pub fn deleted(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "已删除",
            Locale::En => "Deleted",
        }
    }

    pub fn delete_failed(&self, err: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("删除失败：{err}"),
            Locale::En => format!("Delete failed: {err}"),
        }
    }

    pub fn added(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "已添加",
            Locale::En => "Added",
        }
    }

    pub fn link_created(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "链接已创建",
            Locale::En => "Link created",
        }
    }

    pub fn refreshed(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "已刷新",
            Locale::En => "Reloaded",
        }
    }

    // --- 领域枚举展示 ---
    pub fn link_kind(&self, kind: LinkKind) -> &'static str {
        match (self.locale, kind) {
            (Locale::ZhCn, LinkKind::Symlink) => "软链接",
            (Locale::ZhCn, LinkKind::Junction) => "目录联接",
            (Locale::En, LinkKind::Symlink) => "Symlink",
            (Locale::En, LinkKind::Junction) => "Junction",
        }
    }

    pub fn link_status(&self, status: LinkStatus) -> &'static str {
        match (self.locale, status) {
            (Locale::ZhCn, LinkStatus::Ok) => "正常",
            (Locale::ZhCn, LinkStatus::Broken) => "目标没了",
            (Locale::ZhCn, LinkStatus::Missing) => "链接没了",
            (Locale::ZhCn, LinkStatus::Stale) => "不是软链",
            (Locale::ZhCn, LinkStatus::Drift) => "指向不对",
            (Locale::En, LinkStatus::Ok) => "OK",
            (Locale::En, LinkStatus::Broken) => "Broken",
            (Locale::En, LinkStatus::Missing) => "Missing",
            (Locale::En, LinkStatus::Stale) => "Stale",
            (Locale::En, LinkStatus::Drift) => "Drift",
        }
    }
}
