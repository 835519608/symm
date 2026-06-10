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
    pub fn apply_link_op(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "链接操作",
            Locale::En => "Link operation",
        }
    }

    pub fn apply_link_op_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "创建、接管或改指向链接",
            Locale::En => "Add, adopt, or point a link",
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

    pub fn settings_theme(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "主题",
            Locale::En => "Theme",
        }
    }

    pub fn settings_locale(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "界面语言",
            Locale::En => "Language",
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

    pub fn settings_data_dir(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "数据目录",
            Locale::En => "Data directory",
        }
    }

    pub fn settings_data_dir_hint(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "默认 data/（可执行文件旁）",
            Locale::En => "Default: data/ next to executable",
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
            Locale::ZhCn => "存放 symm.db；GUI 设置仍保存在默认 data/settings.json",
            Locale::En => "Stores symm.db; GUI settings stay in default data/settings.json",
        }
    }

    pub fn settings_data_dir_env_override_note(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "当前由 SYMM_HOME 覆盖；移除环境变量后才使用 GUI 保存的数据目录",
            Locale::En => {
                "Currently overridden by SYMM_HOME; saved GUI data directory applies after removing it"
            }
        }
    }

    pub fn settings_save_failed(&self, err: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("设置保存失败：{err}"),
            Locale::En => format!("Failed to save settings: {err}"),
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

    pub fn settings_restore_defaults_tip_data_dir_overridden(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "重置可编辑项：配色、字号、侧栏宽度",
            Locale::En => "Reset editable items: color, font size, sidebar width",
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
            Locale::ZhCn => format!("共 {total} · 软链接 {symlink} · 目录联接 {junction}"),
            Locale::En => format!("{total} total · {symlink} symlinks · {junction} junctions"),
        }
    }

    pub fn sidebar_match_stats(&self, matched: usize) -> String {
        match self.locale {
            Locale::ZhCn => format!("匹配 {matched}"),
            Locale::En => format!("{matched} matched"),
        }
    }

    pub fn page_status(&self, page: u32, page_count: u32) -> String {
        match self.locale {
            Locale::ZhCn => format!("{page} / {page_count}"),
            Locale::En => format!("{page} / {page_count}"),
        }
    }

    pub fn previous_page(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "上一页",
            Locale::En => "Previous page",
        }
    }

    pub fn next_page(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "下一页",
            Locale::En => "Next page",
        }
    }

    pub fn page_size_label(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "每页",
            Locale::En => "Rows",
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

    pub fn search_label(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "搜索链接",
            Locale::En => "Search links",
        }
    }

    pub fn select_link_label(&self, name: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("选择 {name}"),
            Locale::En => format!("Select {name}"),
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

    // --- 链接操作 ---
    pub fn link_op_heading(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "链接操作",
            Locale::En => "Link operation",
        }
    }

    pub fn link_op_add(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "创建/登记链接",
            Locale::En => "Add/register",
        }
    }

    pub fn link_op_adopt(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "接管实体",
            Locale::En => "Adopt",
        }
    }

    pub fn link_op_point(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "改指向",
            Locale::En => "Point",
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
            Locale::ZhCn => "留空则不保存名称，列表会显示链接文件名",
            Locale::En => "Leave empty to store no name; the list shows the link file name",
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
            Locale::ZhCn => "选择文件或文件夹",
            Locale::En => "Pick a file or folder",
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn browse_pick_file(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "选择文件",
            Locale::En => "Choose file",
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn browse_pick_file_title(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "选择文件",
            Locale::En => "Choose file",
        }
    }

    pub fn browse_pick_folder(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "选择文件夹",
            Locale::En => "Choose folder",
        }
    }

    pub fn browse_pick_folder_title(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "选择文件夹",
            Locale::En => "Choose folder",
        }
    }

    #[cfg(target_os = "macos")]
    pub fn browse_pick_unified_title(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "选择路径",
            Locale::En => "Choose path",
        }
    }

    #[cfg(target_os = "macos")]
    pub fn browse_pick_unified_prompt(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "选择",
            Locale::En => "Choose",
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

    pub fn lock_unlock_confirmation_required(
        &self,
        procs: &[crate::adapters::lock::ProcInfo],
    ) -> String {
        let lines = procs
            .iter()
            .map(|proc| format!("{} {}", proc.pid, proc.display))
            .collect::<Vec<_>>()
            .join("\n");
        match self.locale {
            Locale::ZhCn => {
                format!("检测到占用进程。再次点击“确认并结束进程”才会结束这些进程：\n{lines}")
            }
            Locale::En => format!(
                "Locking processes were found. Click \"Confirm and close processes\" again to close them:\n{lines}"
            ),
        }
    }

    pub fn lock_confirm_submit(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "确认并结束进程",
            Locale::En => "Confirm and close processes",
        }
    }

    pub fn lock_confirm_submit_tip(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "结束已展示的占用进程并继续链接操作",
            Locale::En => "Close the shown locking processes and continue",
        }
    }

    pub fn link_op_submit(
        &self,
        operation: crate::workflows::link_ops::workflow::LinkOperation,
    ) -> &'static str {
        match (self.locale, operation) {
            (Locale::ZhCn, crate::workflows::link_ops::workflow::LinkOperation::Add) => {
                "创建/登记链接"
            }
            (Locale::ZhCn, crate::workflows::link_ops::workflow::LinkOperation::Adopt) => {
                "接管实体"
            }
            (Locale::ZhCn, crate::workflows::link_ops::workflow::LinkOperation::Point) => "改指向",
            (Locale::En, crate::workflows::link_ops::workflow::LinkOperation::Add) => {
                "Add/register"
            }
            (Locale::En, crate::workflows::link_ops::workflow::LinkOperation::Adopt) => "Adopt",
            (Locale::En, crate::workflows::link_ops::workflow::LinkOperation::Point) => "Point",
        }
    }

    pub fn link_op_submit_tip(
        &self,
        operation: crate::workflows::link_ops::workflow::LinkOperation,
    ) -> &'static str {
        match (self.locale, operation) {
            (Locale::ZhCn, crate::workflows::link_ops::workflow::LinkOperation::Add) => {
                "创建新链接，或登记已指向 target 的现有链接"
            }
            (Locale::ZhCn, crate::workflows::link_ops::workflow::LinkOperation::Adopt) => {
                "迁移 link 路径实体并创建链接"
            }
            (Locale::ZhCn, crate::workflows::link_ops::workflow::LinkOperation::Point) => {
                "把现有链接改为指向新的 target"
            }
            (Locale::En, crate::workflows::link_ops::workflow::LinkOperation::Add) => {
                "Create a new link, or register an existing link that already points to target"
            }
            (Locale::En, crate::workflows::link_ops::workflow::LinkOperation::Adopt) => {
                "Move the link-path entity and create link"
            }
            (Locale::En, crate::workflows::link_ops::workflow::LinkOperation::Point) => {
                "Point the existing link at the new target"
            }
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

    pub fn field_status_error(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "状态错误",
            Locale::En => "Status error",
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

    pub fn copy_field_tip(&self, label: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("复制{label}"),
            Locale::En => format!("Copy {label}"),
        }
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
            Locale::En => format!("{first} and {n} total links"),
        }
    }

    pub fn rm_selected_summary(&self, n: usize) -> String {
        match self.locale {
            Locale::ZhCn => format!("已选 {n} 条链接"),
            Locale::En => format!("{n} selected links"),
        }
    }

    pub fn rm_mode_delete_only(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "只删除 link 路径上的链接与数据库记录",
            Locale::En => "Remove link and database record only",
        }
    }

    pub fn rm_mode_restore(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "移除 link，并把 target 移回 link 路径",
            Locale::En => "Remove link and move target back to link path",
        }
    }

    pub fn confirm_delete(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "确认删除",
            Locale::En => "Confirm",
        }
    }

    pub fn confirm_restore(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "确认恢复",
            Locale::En => "Restore",
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

    pub fn task_failed(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "后台任务异常结束",
            Locale::En => "Background task ended unexpectedly",
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

    pub fn restored(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "已恢复",
            Locale::En => "Restored",
        }
    }

    pub fn delete_failed(&self, err: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("删除失败：{err}"),
            Locale::En => format!("Delete failed: {err}"),
        }
    }

    pub fn restore_failed(&self, err: &str) -> String {
        match self.locale {
            Locale::ZhCn => format!("恢复失败：{err}"),
            Locale::En => format!("Restore failed: {err}"),
        }
    }

    pub fn added(&self) -> &'static str {
        match self.locale {
            Locale::ZhCn => "已添加",
            Locale::En => "Added",
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
            (Locale::ZhCn, LinkStatus::Stale) => "链接已陈旧",
            (Locale::ZhCn, LinkStatus::Drift) => "指向不对",
            (Locale::ZhCn, LinkStatus::Unknown) => "未知",
            (Locale::En, LinkStatus::Ok) => "OK",
            (Locale::En, LinkStatus::Broken) => "Broken",
            (Locale::En, LinkStatus::Missing) => "Missing",
            (Locale::En, LinkStatus::Stale) => "Stale",
            (Locale::En, LinkStatus::Drift) => "Drift",
            (Locale::En, LinkStatus::Unknown) => "Unknown",
        }
    }
}
