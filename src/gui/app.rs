use crate::domain::gui_settings::ColorScheme;
use crate::domain::model::{LinkKind, LinkRecord, LinkStatus, LinkView};
use crate::gui::icon;
use crate::gui::panels::{open_rm_dialog_batch, validate_add_form};
use crate::gui::settings_store::{self, from_state};
use crate::gui::shell::{self, AddDialogAction, RmDialogAction, SettingsDialogAction};
use crate::gui::state::{AppState, LinkSnapshot, RmDialog, SettingsSection};
use crate::gui::tasks::{GuiTask, GuiTaskResult, TaskPoll};
use crate::gui::theme;
use crate::gui::theme::ThemePreference;
use crate::workflows::rm::workflow::RemoveMode;
use eframe::CreationContext;
use std::collections::HashSet;
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
    needs_reload: bool,
    task: Option<GuiTask>,
    toast_until: Option<Instant>,
    saved_settings: crate::domain::gui_settings::GuiSettings,
    pending_settings: Option<crate::domain::gui_settings::GuiSettings>,
    settings_save_due: Option<Instant>,
    manual_refresh_pending: bool,
    applied_theme: Option<ThemeKey>,
    debug_open_settings: bool,
    debug_settings_section: Option<SettingsSection>,
    debug_open_add: bool,
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
        },
    ])
}

impl SymmApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        let mut state = AppState::default();
        let saved_settings = settings_store::load_into(&mut state);
        if let Some(ppp) = cc.egui_ctx.native_pixels_per_point().filter(|&p| p > 0.0) {
            cc.egui_ctx.set_pixels_per_point(ppp);
        }
        theme::install_fonts(&cc.egui_ctx);
        let mut app = Self {
            state,
            snapshot: LinkSnapshot::default(),
            needs_reload: true,
            task: None,
            toast_until: None,
            saved_settings,
            pending_settings: None,
            settings_save_due: None,
            manual_refresh_pending: false,
            applied_theme: None,
            debug_open_settings: std::env::var_os("SYMM_DEBUG_OPEN_SETTINGS").is_some(),
            debug_settings_section: debug_settings_section(),
            debug_open_add: std::env::var_os("SYMM_DEBUG_OPEN_ADD").is_some(),
            debug_open_rm: std::env::var_os("SYMM_DEBUG_OPEN_RM").is_some(),
            debug_sample_data: std::env::var_os("SYMM_DEBUG_SAMPLE_DATA").is_some(),
            debug_screenshot_to: std::env::var_os("SYMM_DEBUG_SCREENSHOT_TO").map(PathBuf::from),
            debug_screenshot_requested: false,
        };
        app.apply_theme(&cc.egui_ctx);
        if app.debug_sample_data {
            app.snapshot = debug_sample_snapshot();
            app.state.selected_id = Some(1);
            app.needs_reload = false;
        }
        app
    }

    fn persist_settings_if_changed(&mut self, ctx: &egui::Context) {
        let current = from_state(&self.state);
        if current == self.saved_settings {
            self.pending_settings = None;
            self.settings_save_due = None;
            return;
        }

        let now = Instant::now();
        if self.pending_settings.as_ref() != Some(&current) {
            self.pending_settings = Some(current);
            self.settings_save_due = Some(now + Duration::from_millis(500));
            ctx.request_repaint_after(Duration::from_millis(500));
            return;
        }

        let Some(due) = self.settings_save_due else {
            return;
        };
        if now < due {
            ctx.request_repaint_after(due.saturating_duration_since(now));
            return;
        }

        if let Some(pending) = self.pending_settings.take() {
            match settings_store::save(&pending) {
                Ok(()) => {
                    self.saved_settings = pending;
                    self.settings_save_due = None;
                }
                Err(err) => {
                    self.pending_settings = Some(pending);
                    self.settings_save_due = Some(now + Duration::from_secs(5));
                    self.toast(
                        self.state.texts().settings_save_failed(&err.to_string()),
                        4200,
                    );
                    ctx.request_repaint_after(Duration::from_secs(5));
                }
            }
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let key = ThemeKey {
            theme: self.state.theme,
            color_scheme: self.state.color_scheme,
            font_size_pt: self.state.font_size_pt,
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
        self.spawn_task(ctx, || {
            GuiTaskResult::Reload(crate::gui::data::reload().map_err(|err| err.to_string()))
        });
    }

    fn apply_reload_result(&mut self, result: Result<LinkSnapshot, String>) {
        match result {
            Ok(snapshot) => {
                self.snapshot = snapshot;
                self.state.sidebar_filter.clear();
                self.state.db_error = None;
                if self.manual_refresh_pending {
                    self.state.refresh_notice_until =
                        Some(Instant::now() + Duration::from_millis(1800));
                }
                let valid: HashSet<i64> = self.snapshot.views.iter().map(|v| v.id).collect();
                self.state.checked_ids.retain(|id| valid.contains(id));
                if let Some(selected_id) = self.state.selected_id
                    && !valid.contains(&selected_id)
                {
                    self.state.selected_id = None;
                }
            }
            Err(err) => {
                self.snapshot = LinkSnapshot::default();
                self.state.sidebar_filter.clear();
                self.state.selected_id = None;
                self.state.checked_ids.clear();
                self.state.rm_dialog = None;
                self.state.db_error = Some(self.state.texts().db_open_failed(&err));
            }
        }
        self.manual_refresh_pending = false;
    }

    fn apply_settings_draft(&mut self, ctx: &egui::Context) {
        if self.task.is_some() {
            return;
        }
        let Some(draft) = self.state.settings_draft.take() else {
            return;
        };

        if let Err(msg) = settings_store::apply_data_dir(&draft.data_dir) {
            self.state.settings_draft = Some(draft);
            self.toast(msg, 4200);
            return;
        }

        let data_dir_changed = self.state.data_dir != draft.data_dir.trim();
        let sidebar_max = theme::sidebar_max_width(ctx);
        self.state.color_scheme = draft.color_scheme;
        self.state.font_size_pt =
            crate::domain::gui_settings::sanitize_font_size_pt(draft.font_size_pt);
        self.state.sidebar_width = draft
            .sidebar_width
            .clamp(theme::SIDEBAR_WIDTH_MIN, sidebar_max);
        self.state.data_dir = draft.data_dir.trim().to_string();
        self.state.persisted_data_dir = self.state.data_dir.clone();
        self.state.data_dir_runtime_override = false;
        theme::pin_side_panel_width(ctx, theme::SIDEBAR_PANEL_ID, self.state.sidebar_width);

        let current = from_state(&self.state);
        match settings_store::save(&current) {
            Ok(()) => {
                self.saved_settings = current;
                self.pending_settings = None;
                self.settings_save_due = None;
            }
            Err(err) => {
                self.toast(
                    self.state.texts().settings_save_failed(&err.to_string()),
                    4200,
                );
            }
        }
        self.needs_reload = true;
        if data_dir_changed {
            self.state.selected_id = None;
            self.state.checked_ids.clear();
            self.state.rm_dialog = None;
        }
    }

    fn poll_task(&mut self, ctx: &egui::Context) {
        let Some(task) = &self.task else {
            return;
        };

        let result = match task.poll() {
            TaskPoll::Ready(result) => result,
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
        self.handle_task_result(result);
        ctx.request_repaint();
    }

    fn handle_task_result(&mut self, result: GuiTaskResult) {
        match result {
            GuiTaskResult::Reload(result) => self.apply_reload_result(result),
            GuiTaskResult::Add(result) => self.finish_add(result),
            GuiTaskResult::Remove(result) => self.finish_remove(result),
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

    fn request_repaint_when_needed(&self, ctx: &egui::Context) {
        if self.state.busy {
            ctx.request_repaint();
            return;
        }
        let now = Instant::now();
        let next_deadline = [self.toast_until, self.state.refresh_notice_until]
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
        let views: Vec<_> = self
            .state
            .checked_ids
            .iter()
            .filter_map(|id| self.snapshot.view_by_id(*id))
            .collect();
        if views.is_empty() {
            self.toast(self.state.texts().select_before_delete(), 2400);
            return;
        }
        open_rm_dialog_batch(&mut self.state, &views);
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
        self.spawn_task(ctx, move || {
            GuiTaskResult::Remove(
                crate::gui::data::remove_links(&ids, mode).map_err(|err| err.to_string()),
            )
        });
    }

    fn finish_remove(&mut self, result: Result<String, String>) {
        match result {
            Ok(log) => {
                self.state.rm_dialog = None;
                self.needs_reload = true;
                self.state.selected_id = None;
                self.state.checked_ids.clear();
                let msg = log
                    .lines()
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| self.state.texts().deleted().to_string());
                self.toast(msg, 3600);
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

    fn submit_add(&mut self, ctx: &egui::Context) {
        if self.task.is_some() {
            return;
        }
        let locale = self.state.locale;
        let t = self.state.texts();
        let form = &mut self.state.add_form;
        form.error = None;
        form.status_message = None;
        let Ok((link, target)) = validate_add_form(form, locale) else {
            form.error = Some(t.paths_required().to_owned());
            return;
        };
        let name = form.name.trim().to_string();
        let operation = form.operation;
        let lock = form.lock_policy;
        self.spawn_task(ctx, move || {
            GuiTaskResult::Add(
                crate::gui::data::add_link(operation, &link, &target, &name, lock)
                    .map_err(|err| err.to_string()),
            )
        });
    }

    fn finish_add(&mut self, result: Result<String, String>) {
        let t = self.state.texts();
        let form = &mut self.state.add_form;
        match result {
            Ok(log) => {
                form.status_message = Some(
                    log.lines()
                        .last()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| t.added().to_string()),
                );
                form.link_path.clear();
                form.target_path.clear();
                form.name.clear();
                self.needs_reload = true;
                self.state.show_add_dialog = false;
                self.toast(t.link_created(), 3000);
            }
            Err(err) => form.error = Some(err),
        }
    }
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
        if self.debug_open_add && !self.state.show_add_dialog {
            crate::gui::panels::open_add_dialog(&mut self.state);
        }
        if self.debug_open_rm && self.state.rm_dialog.is_none() {
            self.state.rm_dialog = Some(RmDialog {
                ids: vec![],
                summary: "debug-link".to_string(),
                mode: RemoveMode::DeleteLinkOnly,
            });
        }
        self.expire_notices();
        self.persist_settings_if_changed(ctx);

        let before_theme = ThemeKey {
            theme: self.state.theme,
            color_scheme: self.state.color_scheme,
            font_size_pt: self.state.font_size_pt,
        };
        let before_locale = self.state.locale;
        let frame_actions = shell::show_frame(ctx, &mut self.state, &self.snapshot);
        if frame_actions.refresh_requested {
            self.needs_reload = true;
            self.state.refresh_notice_until = None;
            self.manual_refresh_pending = true;
        }
        if frame_actions.delete_checked_requested {
            self.begin_rm_checked();
        }

        let dialog_actions = shell::show_dialogs(ctx, &mut self.state);
        if dialog_actions.add == AddDialogAction::Submit {
            self.submit_add(ctx);
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
