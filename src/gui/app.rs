use crate::domain::gui_settings::{ColorScheme, GuiSettings, data_dir_from_settings};
use crate::domain::model::{LinkKind, LinkRecord, LinkStatus, LinkView};
use crate::gui::data::{GuiLinkOpError, ReloadedLinks, RemoveOutcome};
use crate::gui::icon;
use crate::gui::panels::{open_rm_dialog_batch_ids, validate_link_op_form};
use crate::gui::settings_store;
use crate::gui::shell::{self, LinkOpDialogAction, RmDialogAction, SettingsDialogAction};
use crate::gui::state::{
    AppState, LinkOpForm, LinkOpLockConfirmation, LinkSnapshot, RmDialog, SettingsDraft,
    SettingsSection,
};
use crate::gui::tasks::{GuiTask, GuiTaskResult, SettingsApplyOutcome, TaskPoll};
use crate::gui::theme;
use crate::gui::theme::ThemePreference;
use crate::workflows::rm::workflow::RemoveMode;
use eframe::CreationContext;
use std::path::PathBuf;
use std::time::{Duration, Instant};
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("symm")
            .with_icon(icon::viewport_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "symm",
        options,
        Box::new(|cc| Ok(Box::new(SymmApp::new(cc)))),
    )
}

pub struct SymmApp {
    state: AppState,
    snapshot: LinkSnapshot,
    selected_view: Option<LinkView>,
    needs_reload: bool,
    task: Option<GuiTask>,
    toast_until: Option<Instant>,
    search_reload_at: Option<Instant>,
    manual_refresh_pending: bool,
    applied_theme: Option<ThemeKey>,
    debug_open_settings: bool,
    debug_settings_section: Option<SettingsSection>,
    debug_open_link_op: bool,
    debug_open_rm: bool,
    debug_sample_data: bool,
    debug_screenshot_to: Option<PathBuf>,
    debug_screenshot_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ThemeKey {
    theme: ThemePreference,
    color_scheme: ColorScheme,
    font_size_pt: f32,
    resolved_dark: bool,
}

fn debug_settings_section() -> Option<SettingsSection> {
    match std::env::var("SYMM_DEBUG_SETTINGS_SECTION").ok()?.as_str() {
        "appearance" => Some(SettingsSection::Appearance),
        "about" => Some(SettingsSection::About),
        _ => None,
    }
}

fn debug_sample_snapshot() -> LinkSnapshot {
    LinkSnapshot::new(vec![
        LinkView {
            record: LinkRecord {
                id: 1,
                name: "工作区配置".to_string(),
                link_path: "/home/wjh/.config/symm/workspace".to_string(),
                target_path: "/mnt/d/projects/shared/workspace".to_string(),
                link_kind: LinkKind::Symlink,
                created_at: 0,
                updated_at: 0,
            },
            index: 1,
            status: LinkStatus::Ok,
            status_error: None,
        },
        LinkView {
            record: LinkRecord {
                id: 2,
                name: "长名称示例-用于检查侧栏是否稳定显示".to_string(),
                link_path: "/home/wjh/.local/share/symm/a/very/long/link/path".to_string(),
                target_path: "/mnt/archive/symm/a/very/long/target/path".to_string(),
                link_kind: LinkKind::Junction,
                created_at: 0,
                updated_at: 0,
            },
            index: 2,
            status: LinkStatus::Broken,
            status_error: None,
        },
    ])
}

impl SymmApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        let mut state = AppState::default();
        settings_store::load_into(&mut state);
        if let Some(ppp) = cc.egui_ctx.native_pixels_per_point().filter(|&p| p > 0.0) {
            cc.egui_ctx.set_pixels_per_point(ppp);
        }
        theme::install_fonts(&cc.egui_ctx);
        let mut app = Self {
            state,
            snapshot: LinkSnapshot::default(),
            selected_view: None,
            needs_reload: true,
            task: None,
            toast_until: None,
            search_reload_at: None,
            manual_refresh_pending: false,
            applied_theme: None,
            debug_open_settings: std::env::var_os("SYMM_DEBUG_OPEN_SETTINGS").is_some(),
            debug_settings_section: debug_settings_section(),
            debug_open_link_op: std::env::var_os("SYMM_DEBUG_OPEN_LINK_OP").is_some(),
            debug_open_rm: std::env::var_os("SYMM_DEBUG_OPEN_RM").is_some(),
            debug_sample_data: std::env::var_os("SYMM_DEBUG_SAMPLE_DATA").is_some(),
            debug_screenshot_to: std::env::var_os("SYMM_DEBUG_SCREENSHOT_TO").map(PathBuf::from),
            debug_screenshot_requested: false,
        };
        app.apply_theme(&cc.egui_ctx);
        if app.debug_sample_data {
            let snapshot = debug_sample_snapshot();
            app.selected_view = snapshot.view_by_id(1).cloned();
            app.snapshot = snapshot;
            app.state.selected_id = Some(1);
            app.needs_reload = false;
        }
        app
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let key = ThemeKey {
            theme: self.state.theme,
            color_scheme: self.state.color_scheme,
            font_size_pt: self.state.font_size_pt,
            resolved_dark: theme::resolve_dark(self.state.theme),
        };
        if self.applied_theme == Some(key) {
            return;
        }
        theme::apply(
            ctx,
            self.state.theme,
            self.state.color_scheme,
            self.state.font_size_pt,
        );
        self.applied_theme = Some(key);
    }

    fn spawn_task(
        &mut self,
        ctx: &egui::Context,
        run: impl FnOnce() -> GuiTaskResult + Send + 'static,
    ) {
        if self.task.is_some() {
            return;
        }
        self.task = Some(GuiTask::spawn(run));
        self.state.busy = true;
        ctx.request_repaint();
    }

    fn start_reload(&mut self, ctx: &egui::Context) {
        self.needs_reload = false;
        let search = self.state.search.clone();
        let page_index = self.state.page_index;
        let page_size = self.state.page_size;
        let selected_id = self.state.selected_id;
        let data_dir = PathBuf::from(self.state.data_dir.trim());
        let checked_ids = self.state.checked_ids.iter().copied().collect::<Vec<_>>();
        self.spawn_task(ctx, move || {
            GuiTaskResult::Reload(
                crate::gui::data::reload(
                    &data_dir,
                    &search,
                    page_index,
                    page_size,
                    selected_id,
                    &checked_ids,
                )
                .map_err(|err| err.to_string()),
            )
        });
    }

    fn apply_reload_result(&mut self, result: Result<ReloadedLinks, String>) {
        match result {
            Ok(reloaded) => {
                self.snapshot = reloaded.snapshot;
                self.state.page_index = reloaded.page_index;
                self.state.db_error = None;
                if self.manual_refresh_pending {
                    self.state.refresh_notice_until =
                        Some(Instant::now() + Duration::from_millis(1800));
                }
                self.state
                    .checked_ids
                    .retain(|id| reloaded.all_ids.contains(id));
                if let Some(selected_id) = self.state.selected_id
                    && !reloaded.all_ids.contains(&selected_id)
                {
                    self.state.selected_id = None;
                    self.selected_view = None;
                }
            }
            Err(err) => {
                self.snapshot = LinkSnapshot::default();
                self.selected_view = None;
                self.state.selected_id = None;
                self.state.checked_ids.clear();
                self.state.rm_dialog = None;
                self.state.db_error = Some(self.state.texts().db_open_failed(&err));
            }
        }
        self.manual_refresh_pending = false;
    }

    fn refresh_selected_view_from_snapshot(&mut self) {
        self.selected_view = self
            .state
            .selected_id
            .and_then(|id| self.snapshot.view_by_id(id).cloned());
    }

    fn apply_settings_draft(&mut self, ctx: &egui::Context) {
        if self.task.is_some() {
            return;
        }
        let Some(draft) = self.state.settings_draft.take() else {
            return;
        };

        let sidebar_max = theme::sidebar_max_width(ctx);
        let settings = self.settings_from_draft(&draft, sidebar_max);
        let new_data_dir = data_dir_from_settings(&settings);
        let page_size = self.state.page_size;
        let data_dir_changed =
            !self.state.data_dir_runtime_override && self.state.data_dir != new_data_dir;

        self.spawn_task(ctx, move || GuiTaskResult::SettingsApply {
            draft,
            result: crate::gui::data::apply_settings(settings, data_dir_changed, page_size),
        });
    }

    fn settings_from_draft(&self, draft: &SettingsDraft, sidebar_max: f32) -> GuiSettings {
        let data_dir = if self.state.data_dir_runtime_override {
            self.state.persisted_data_dir.trim()
        } else {
            draft.data_dir.trim()
        };
        GuiSettings {
            theme: draft.theme,
            locale: draft.locale,
            color_scheme: draft.color_scheme,
            sidebar_width: draft
                .sidebar_width
                .clamp(theme::SIDEBAR_WIDTH_MIN, sidebar_max),
            font_size_pt: crate::domain::gui_settings::sanitize_font_size_pt(draft.font_size_pt),
            data_dir: if data_dir.is_empty() {
                None
            } else {
                Some(data_dir.to_string())
            },
        }
    }

    fn finish_settings_apply(
        &mut self,
        draft: SettingsDraft,
        result: Result<SettingsApplyOutcome, String>,
        ctx: &egui::Context,
    ) {
        let outcome = match result {
            Ok(outcome) => outcome,
            Err(err) => {
                self.state.settings_draft = Some(draft);
                self.toast(err, 4200);
                return;
            }
        };

        self.state.theme = outcome.settings.theme;
        self.state.locale = outcome.settings.locale;
        self.state.color_scheme = outcome.settings.color_scheme;
        self.state.font_size_pt = outcome.settings.font_size_pt;
        self.state.sidebar_width = outcome.settings.sidebar_width;
        self.state.transient_sidebar_width = outcome.settings.sidebar_width;
        let persisted_data_dir = data_dir_from_settings(&outcome.settings);
        if !self.state.data_dir_runtime_override {
            self.state.data_dir = persisted_data_dir.clone();
            self.state.data_dir_runtime_override = false;
        }
        if outcome.save_error.is_none() {
            self.state.persisted_data_dir = persisted_data_dir;
        }
        theme::pin_side_panel_width(ctx, theme::SIDEBAR_PANEL_ID, self.state.sidebar_width);

        if let Some(snapshot) = outcome.snapshot {
            self.snapshot = snapshot;
            self.selected_view = None;
            self.state.db_error = None;
            self.manual_refresh_pending = false;
        }
        if outcome.data_dir_changed {
            self.state.page_index = 0;
            self.state.selected_id = None;
            self.selected_view = None;
            self.state.checked_ids.clear();
            self.state.rm_dialog = None;
        }
        if let Some(err) = outcome.save_error {
            self.toast(self.state.texts().settings_save_failed(&err), 4200);
        }
    }

    fn poll_task(&mut self, ctx: &egui::Context) {
        let Some(task) = &self.task else {
            return;
        };

        let result = match task.poll() {
            TaskPoll::Ready(result) => *result,
            TaskPoll::Pending => {
                ctx.request_repaint_after(Duration::from_millis(50));
                return;
            }
            TaskPoll::Disconnected => {
                self.task = None;
                self.state.busy = false;
                self.toast(self.state.texts().task_failed(), 4200);
                ctx.request_repaint();
                return;
            }
        };

        self.task = None;
        self.state.busy = false;
        self.handle_task_result(result, ctx);
        ctx.request_repaint();
    }

    fn handle_task_result(&mut self, result: GuiTaskResult, ctx: &egui::Context) {
        match result {
            GuiTaskResult::Reload(result) => self.apply_reload_result(result),
            GuiTaskResult::LinkOp(result) => self.finish_link_op(result),
            GuiTaskResult::Remove(result) => self.finish_remove(result),
            GuiTaskResult::SettingsApply { draft, result } => {
                self.finish_settings_apply(draft, result, ctx)
            }
        }
    }

    fn toast(&mut self, msg: impl Into<String>, millis: u64) {
        self.state.toast = Some(msg.into());
        self.toast_until = Some(Instant::now() + Duration::from_millis(millis));
    }

    fn expire_notices(&mut self) {
        let now = Instant::now();
        if self.toast_until.is_some_and(|deadline| now >= deadline) {
            self.toast_until = None;
            self.state.toast = None;
        }
        if self
            .state
            .refresh_notice_until
            .is_some_and(|deadline| now >= deadline)
        {
            self.state.refresh_notice_until = None;
        }
    }

    fn queue_search_reload(&mut self) {
        self.search_reload_at = Some(Instant::now() + Duration::from_millis(250));
    }

    fn apply_deferred_search_reload(&mut self, ctx: &egui::Context) {
        let Some(deadline) = self.search_reload_at else {
            return;
        };
        let now = Instant::now();
        if now >= deadline {
            self.search_reload_at = None;
            self.needs_reload = true;
        } else {
            ctx.request_repaint_after(deadline.saturating_duration_since(now));
        }
    }

    fn request_repaint_when_needed(&self, ctx: &egui::Context) {
        let now = Instant::now();
        let next_deadline = [
            self.toast_until,
            self.state.refresh_notice_until,
            self.search_reload_at,
        ]
        .into_iter()
        .flatten()
        .filter(|deadline| *deadline > now)
        .min();
        if let Some(deadline) = next_deadline {
            ctx.request_repaint_after(deadline.saturating_duration_since(now));
        }
    }

    fn save_debug_screenshot(&self, image: &egui::ColorImage, path: &PathBuf) {
        let mut rgba = Vec::with_capacity(image.pixels.len() * 4);
        for pixel in &image.pixels {
            rgba.extend_from_slice(&pixel.to_array());
        }
        if let Err(err) = image::save_buffer(
            path,
            &rgba,
            image.size[0] as u32,
            image.size[1] as u32,
            image::ColorType::Rgba8,
        ) {
            eprintln!(
                "[SYMM_GUI_DEBUG] failed to save screenshot to {}: {err}",
                path.display()
            );
            return;
        }
        eprintln!("[SYMM_GUI_DEBUG] screenshot saved to {}", path.display());
    }

    fn handle_debug_screenshot_events(&self, ctx: &egui::Context) {
        let Some(path) = &self.debug_screenshot_to else {
            return;
        };
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            if let egui::Event::Screenshot { image, .. } = event {
                self.save_debug_screenshot(&image, path);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                break;
            }
        }
    }

    fn begin_rm_checked(&mut self) {
        if self.state.checked_ids.is_empty() {
            self.toast(self.state.texts().select_before_delete(), 2400);
            return;
        }
        let mut ids = self.state.checked_ids.iter().copied().collect::<Vec<_>>();
        ids.sort_unstable();
        let first_name = ids
            .iter()
            .find_map(|id| self.snapshot.view_by_id(*id))
            .map(|view| view.display_name().into_owned());
        let count = ids.len();
        open_rm_dialog_batch_ids(&mut self.state, ids, first_name, count);
    }

    fn confirm_rm(&mut self, ctx: &egui::Context) {
        if self.task.is_some() {
            return;
        }
        let Some(dialog) = self.state.rm_dialog.clone() else {
            return;
        };
        let ids = dialog.ids;
        let mode = dialog.mode;
        let data_dir = PathBuf::from(self.state.data_dir.trim());
        self.spawn_task(ctx, move || {
            GuiTaskResult::Remove(
                crate::gui::data::remove_links(&data_dir, &ids, mode)
                    .map_err(|err| err.to_string()),
            )
        });
    }

    fn finish_remove(&mut self, result: Result<RemoveOutcome, String>) {
        match result {
            Ok(outcome) => {
                self.state.rm_dialog = None;
                self.needs_reload = true;
                self.state.checked_ids = outcome.remaining_ids;
                if self.state.selected_id.is_some_and(|id| {
                    outcome.attempted_ids.contains(&id) && !self.state.checked_ids.contains(&id)
                }) {
                    self.state.selected_id = None;
                    self.selected_view = None;
                }
                let first_log_line = outcome.log.lines().next();
                let msg = match (first_log_line, outcome.error.as_deref()) {
                    (Some(line), Some(err)) => {
                        format!("{line}\n{}", self.state.texts().delete_failed(err))
                    }
                    (Some(line), None) => line.to_string(),
                    (None, Some(err)) => self.state.texts().delete_failed(err),
                    (None, None) => self.state.texts().deleted().to_string(),
                };
                self.toast(msg, if outcome.error.is_some() { 4200 } else { 3600 });
            }
            Err(err) => {
                self.state.rm_dialog = None;
                self.needs_reload = true;
                self.state.selected_id = None;
                self.state.checked_ids.clear();
                let msg = self.state.texts().delete_failed(&err);
                self.toast(msg, 4200);
            }
        }
    }

    fn submit_link_op(&mut self, ctx: &egui::Context) {
        if self.task.is_some() {
            return;
        }
        let locale = self.state.locale;
        let t = self.state.texts();
        let form = &mut self.state.link_op_form;
        form.error = None;
        form.status_message = None;
        let Ok((link, target)) = validate_link_op_form(form, locale) else {
            form.error = Some(t.paths_required().to_owned());
            return;
        };
        let name = form.name.trim().to_string();
        let operation = form.operation;
        let lock = form.lock_policy;
        let unlock_confirmed = form.lock_confirmation.as_ref().is_some_and(|confirmation| {
            confirmation.matches(operation, &form.link_path, &form.target_path, &name)
        });
        let data_dir = PathBuf::from(self.state.data_dir.trim());
        self.spawn_task(ctx, move || {
            GuiTaskResult::LinkOp(crate::gui::data::apply_link_op(
                &data_dir,
                operation,
                &link,
                &target,
                &name,
                lock,
                unlock_confirmed,
            ))
        });
    }

    fn finish_link_op(&mut self, result: Result<String, GuiLinkOpError>) {
        let t = self.state.texts();
        let form = &mut self.state.link_op_form;
        match result {
            Ok(log) => {
                let message = log
                    .lines()
                    .last()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| t.added().to_string());
                *form = LinkOpForm {
                    status_message: Some(message.clone()),
                    ..LinkOpForm::default()
                };
                self.needs_reload = true;
                self.state.show_link_op_dialog = false;
                self.toast(message, 3000);
            }
            Err(GuiLinkOpError::LockConfirmationRequired { procs }) => {
                form.lock_confirmation = Some(LinkOpLockConfirmation::new(
                    form.operation,
                    form.link_path.clone(),
                    form.target_path.clone(),
                    form.name.trim().to_string(),
                ));
                form.error = Some(t.lock_unlock_confirmation_required(&procs));
            }
            Err(GuiLinkOpError::Workflow(err)) => {
                if link_op_error_needs_reload(&err) {
                    self.needs_reload = true;
                }
                form.lock_confirmation = None;
                form.error = Some(err.to_string());
            }
        }
    }
}

fn link_op_error_needs_reload(err: &crate::domain::error::SymmError) -> bool {
    matches!(
        err,
        crate::domain::error::SymmError::FilesystemAppliedButDbFailed { .. }
            | crate::domain::error::SymmError::EntityMigratedButLinkCreateFailed { .. }
            | crate::domain::error::SymmError::EntityCopiedButSourceCleanupFailed { .. }
    )
}

impl eframe::App for SymmApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let p = theme::resolve(self.state.theme, self.state.color_scheme);
        p.bg.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_debug_screenshot_events(ctx);
        self.poll_task(ctx);
        self.apply_theme(ctx);
        self.apply_deferred_search_reload(ctx);
        if self.needs_reload && self.task.is_none() {
            self.start_reload(ctx);
        }
        if self.debug_open_settings && self.state.settings_draft.is_none() {
            crate::gui::panels::open_settings(&mut self.state);
            if let (Some(section), Some(draft)) = (
                self.debug_settings_section,
                self.state.settings_draft.as_mut(),
            ) {
                draft.section = section;
            }
        }
        if self.debug_open_link_op && !self.state.show_link_op_dialog {
            crate::gui::panels::open_link_op_dialog(&mut self.state);
        }
        if self.debug_open_rm && self.state.rm_dialog.is_none() {
            self.state.rm_dialog = Some(RmDialog {
                ids: vec![],
                summary: "debug-link".to_string(),
                mode: RemoveMode::DeleteLinkOnly,
            });
        }
        self.expire_notices();

        let before_theme = ThemeKey {
            theme: self.state.theme,
            color_scheme: self.state.color_scheme,
            font_size_pt: self.state.font_size_pt,
            resolved_dark: theme::resolve_dark(self.state.theme),
        };
        let before_locale = self.state.locale;
        let before_search = self.state.search.clone();
        let before_selected_id = self.state.selected_id;
        let frame_actions = shell::show_frame(
            ctx,
            &mut self.state,
            &self.snapshot,
            self.selected_view.as_ref(),
        );
        if self.state.search != before_search {
            self.state.page_index = 0;
            self.queue_search_reload();
        }
        if self.state.selected_id != before_selected_id || frame_actions.selected_id.is_some() {
            self.refresh_selected_view_from_snapshot();
            ctx.request_repaint();
        }
        if frame_actions.refresh_requested {
            self.search_reload_at = None;
            self.needs_reload = true;
            self.state.refresh_notice_until = None;
            self.manual_refresh_pending = true;
        }
        if frame_actions.page_changed {
            self.search_reload_at = None;
            self.needs_reload = true;
        }
        if frame_actions.delete_checked_requested {
            self.begin_rm_checked();
        }

        let dialog_actions = shell::show_dialogs(ctx, &mut self.state);
        if dialog_actions.link_op == LinkOpDialogAction::Submit {
            self.submit_link_op(ctx);
        }

        match dialog_actions.rm {
            RmDialogAction::Confirm => self.confirm_rm(ctx),
            RmDialogAction::Cancel | RmDialogAction::None => {}
        }

        match dialog_actions.settings {
            SettingsDialogAction::Apply => self.apply_settings_draft(ctx),
            SettingsDialogAction::Close | SettingsDialogAction::None => {}
        }

        let after_theme = ThemeKey {
            theme: self.state.theme,
            color_scheme: self.state.color_scheme,
            font_size_pt: self.state.font_size_pt,
            resolved_dark: theme::resolve_dark(self.state.theme),
        };
        if self.needs_reload || after_theme != before_theme || self.state.locale != before_locale {
            ctx.request_repaint();
        }
        if let Some(path) = &self.debug_screenshot_to
            && !self.debug_screenshot_requested
        {
            eprintln!(
                "[SYMM_GUI_DEBUG] requesting screenshot to {}",
                path.display()
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            ctx.request_repaint();
            self.debug_screenshot_requested = true;
        }
        self.request_repaint_when_needed(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn test_app() -> SymmApp {
        SymmApp {
            state: AppState::default(),
            snapshot: LinkSnapshot::default(),
            selected_view: None,
            needs_reload: false,
            task: None,
            toast_until: None,
            search_reload_at: None,
            manual_refresh_pending: false,
            applied_theme: None,
            debug_open_settings: false,
            debug_settings_section: None,
            debug_open_link_op: false,
            debug_open_rm: false,
            debug_sample_data: false,
            debug_screenshot_to: None,
            debug_screenshot_requested: false,
        }
    }

    #[test]
    fn remove_partial_failure_keeps_remaining_selection() {
        let mut app = test_app();
        app.state.selected_id = Some(2);
        app.state.checked_ids = [1, 2].into_iter().collect();

        app.finish_remove(Ok(RemoveOutcome {
            log: "已删除链接关系：demo\n失败：other：权限不足".to_string(),
            attempted_ids: HashSet::from([1, 2]),
            remaining_ids: HashSet::from([2]),
            error: Some("部分删除失败：other".to_string()),
        }));

        assert!(app.needs_reload);
        assert_eq!(app.state.selected_id, Some(2));
        assert_eq!(app.state.checked_ids, HashSet::from([2]));
        let toast = app.state.toast.as_deref().expect("toast");
        assert!(toast.contains("demo"));
        assert!(toast.contains("删除失败"));
    }

    #[test]
    fn remove_success_clears_selection() {
        let mut app = test_app();
        app.state.selected_id = Some(1);
        app.state.checked_ids = [1].into_iter().collect();

        app.finish_remove(Ok(RemoveOutcome {
            log: "已删除链接关系：demo".to_string(),
            attempted_ids: HashSet::from([1]),
            remaining_ids: HashSet::new(),
            error: None,
        }));

        assert!(app.needs_reload);
        assert_eq!(app.state.selected_id, None);
        assert!(app.state.checked_ids.is_empty());
    }

    #[test]
    fn remove_partial_failure_clears_deleted_selected_id() {
        let mut app = test_app();
        app.state.selected_id = Some(1);
        app.state.checked_ids = [1, 2].into_iter().collect();

        app.finish_remove(Ok(RemoveOutcome {
            log: "已删除链接关系：demo\n失败：other：权限不足".to_string(),
            attempted_ids: HashSet::from([1, 2]),
            remaining_ids: HashSet::from([2]),
            error: Some("部分删除失败：other".to_string()),
        }));

        assert_eq!(app.state.selected_id, None);
        assert_eq!(app.state.checked_ids, HashSet::from([2]));
    }

    #[test]
    fn remove_success_keeps_unattempted_selected_id() {
        let mut app = test_app();
        app.state.selected_id = Some(9);
        app.state.checked_ids = [1].into_iter().collect();
        app.selected_view = Some(LinkView {
            record: LinkRecord {
                id: 9,
                name: "selected".to_string(),
                link_path: "/tmp/selected-link".to_string(),
                target_path: "/tmp/selected-target".to_string(),
                link_kind: LinkKind::Symlink,
                created_at: 0,
                updated_at: 0,
            },
            index: 9,
            status: LinkStatus::Ok,
            status_error: None,
        });

        app.finish_remove(Ok(RemoveOutcome {
            log: "已删除链接关系：demo".to_string(),
            attempted_ids: HashSet::from([1]),
            remaining_ids: HashSet::new(),
            error: None,
        }));

        assert_eq!(app.state.selected_id, Some(9));
        assert!(app.selected_view.is_some());
        assert!(app.state.checked_ids.is_empty());
    }

    #[test]
    fn link_op_lock_confirmation_required_sets_confirmation_state() {
        let mut app = test_app();
        app.state.link_op_form.operation =
            crate::workflows::link_ops::workflow::LinkOperation::Adopt;
        app.state.link_op_form.link_path = "/tmp/link".to_string();
        app.state.link_op_form.target_path = "/tmp/target".to_string();
        app.state.link_op_form.name = "demo".to_string();
        app.state.link_op_form.lock_policy = crate::gui::state::LinkOpLockPolicy::Unlock;

        app.finish_link_op(Err(GuiLinkOpError::LockConfirmationRequired {
            procs: vec![crate::adapters::lock::ProcInfo {
                pid: 42,
                display: "demo.exe".to_string(),
            }],
        }));

        assert!(app.state.link_op_form.lock_confirmation.is_some());
        let err = app
            .state
            .link_op_form
            .error
            .as_deref()
            .expect("confirmation message");
        assert!(err.contains("42"));
        assert!(err.contains("demo.exe"));
    }

    #[test]
    fn link_op_success_resets_form_and_lock_policy() {
        let mut app = test_app();
        app.state.show_link_op_dialog = true;
        app.state.link_op_form.operation =
            crate::workflows::link_ops::workflow::LinkOperation::Adopt;
        app.state.link_op_form.link_path = "/tmp/link".to_string();
        app.state.link_op_form.target_path = "/tmp/target".to_string();
        app.state.link_op_form.name = "demo".to_string();
        app.state.link_op_form.lock_policy = crate::gui::state::LinkOpLockPolicy::Unlock;
        app.state.link_op_form.error = Some("old error".to_string());
        app.state.link_op_form.lock_confirmation = Some(LinkOpLockConfirmation::new(
            crate::workflows::link_ops::workflow::LinkOperation::Adopt,
            "/tmp/link".to_string(),
            "/tmp/target".to_string(),
            "demo".to_string(),
        ));

        app.finish_link_op(Ok("正在保存记录：/tmp/link\n已接管：/tmp/link".to_string()));

        let form = &app.state.link_op_form;
        assert_eq!(
            form.lock_policy,
            crate::gui::state::LinkOpLockPolicy::Cancel
        );
        assert_eq!(
            form.operation,
            crate::workflows::link_ops::workflow::LinkOperation::Add
        );
        assert!(form.link_path.is_empty());
        assert!(form.target_path.is_empty());
        assert!(form.name.is_empty());
        assert!(form.error.is_none());
        assert!(form.lock_confirmation.is_none());
        assert_eq!(form.status_message.as_deref(), Some("已接管：/tmp/link"));
        assert!(!app.state.show_link_op_dialog);
        assert!(app.needs_reload);
    }

    #[test]
    fn settings_apply_hard_failure_restores_draft() {
        let mut app = test_app();
        let ctx = egui::Context::default();
        let mut draft = SettingsDraft::from_state(&app.state);
        draft.data_dir = "/tmp/failed-settings-dir".to_string();

        app.finish_settings_apply(draft, Err("无法打开数据目录".to_string()), &ctx);

        let restored = app
            .state
            .settings_draft
            .as_ref()
            .expect("draft should be restored");
        assert_eq!(restored.data_dir, "/tmp/failed-settings-dir");
        assert_eq!(app.state.toast.as_deref(), Some("无法打开数据目录"));
    }

    #[test]
    fn settings_apply_save_error_consumes_draft_and_keeps_old_persisted_data_dir() {
        let mut app = test_app();
        let ctx = egui::Context::default();
        app.state.settings_draft = None;
        app.state.data_dir = "/tmp/old-active-dir".to_string();
        app.state.persisted_data_dir = "/tmp/old-persisted-dir".to_string();
        let settings = GuiSettings {
            data_dir: Some("/tmp/new-active-dir".to_string()),
            ..GuiSettings::default()
        };
        let draft = SettingsDraft::from_state(&app.state);

        app.finish_settings_apply(
            draft,
            Ok(SettingsApplyOutcome {
                settings,
                snapshot: None,
                data_dir_changed: true,
                save_error: Some("disk full".to_string()),
            }),
            &ctx,
        );

        assert!(app.state.settings_draft.is_none());
        assert_eq!(app.state.data_dir, "/tmp/new-active-dir");
        assert_eq!(app.state.persisted_data_dir, "/tmp/old-persisted-dir");
        assert!(
            app.state
                .toast
                .as_deref()
                .expect("save error toast")
                .contains("disk full")
        );
    }

    #[test]
    fn reload_keeps_cached_selected_detail_when_selected_id_still_exists() {
        let mut app = test_app();
        app.state.selected_id = Some(9);
        app.selected_view = Some(LinkView {
            record: LinkRecord {
                id: 9,
                name: "cached-detail".to_string(),
                link_path: "/tmp/cached-link".to_string(),
                target_path: "/tmp/cached-target".to_string(),
                link_kind: LinkKind::Symlink,
                created_at: 0,
                updated_at: 0,
            },
            index: 9,
            status: LinkStatus::Ok,
            status_error: None,
        });

        app.apply_reload_result(Ok(ReloadedLinks {
            snapshot: LinkSnapshot::new(Vec::new()),
            all_ids: HashSet::from([9]),
            page_index: 0,
        }));

        assert_eq!(app.state.selected_id, Some(9));
        assert_eq!(
            app.selected_view.as_ref().map(|view| view.name.as_str()),
            Some("cached-detail")
        );
    }

    #[test]
    fn reload_clears_cached_selected_detail_when_selected_id_disappears() {
        let mut app = test_app();
        app.state.selected_id = Some(9);
        app.selected_view = Some(LinkView {
            record: LinkRecord {
                id: 9,
                name: "cached-detail".to_string(),
                link_path: "/tmp/cached-link".to_string(),
                target_path: "/tmp/cached-target".to_string(),
                link_kind: LinkKind::Symlink,
                created_at: 0,
                updated_at: 0,
            },
            index: 9,
            status: LinkStatus::Ok,
            status_error: None,
        });

        app.apply_reload_result(Ok(ReloadedLinks {
            snapshot: LinkSnapshot::new(Vec::new()),
            all_ids: HashSet::new(),
            page_index: 0,
        }));

        assert_eq!(app.state.selected_id, None);
        assert!(app.selected_view.is_none());
    }

    #[test]
    fn selected_detail_refreshes_from_snapshot_on_explicit_click() {
        let mut app = test_app();
        app.state.selected_id = Some(9);
        app.snapshot = LinkSnapshot::new(vec![LinkView {
            record: LinkRecord {
                id: 9,
                name: "fresh-detail".to_string(),
                link_path: "/tmp/fresh-link".to_string(),
                target_path: "/tmp/fresh-target".to_string(),
                link_kind: LinkKind::Symlink,
                created_at: 0,
                updated_at: 0,
            },
            index: 9,
            status: LinkStatus::Ok,
            status_error: None,
        }]);
        app.selected_view = Some(LinkView {
            record: LinkRecord {
                id: 9,
                name: "cached-detail".to_string(),
                link_path: "/tmp/cached-link".to_string(),
                target_path: "/tmp/cached-target".to_string(),
                link_kind: LinkKind::Symlink,
                created_at: 0,
                updated_at: 0,
            },
            index: 9,
            status: LinkStatus::Missing,
            status_error: None,
        });

        app.refresh_selected_view_from_snapshot();

        assert_eq!(
            app.selected_view.as_ref().map(|view| view.name.as_str()),
            Some("fresh-detail")
        );
    }

    #[test]
    fn search_change_defers_reload() {
        let mut app = test_app();

        app.queue_search_reload();

        assert!(!app.needs_reload);
        assert!(app.search_reload_at.is_some());
    }

    #[test]
    fn expired_search_reload_marks_reload_needed() {
        let mut app = test_app();
        let ctx = egui::Context::default();
        app.search_reload_at = Some(Instant::now() - Duration::from_millis(1));

        app.apply_deferred_search_reload(&ctx);

        assert_eq!(app.search_reload_at, None);
        assert!(app.needs_reload);
    }
}
