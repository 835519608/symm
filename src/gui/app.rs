use crate::gui::data::DataStore;
use crate::gui::icon;
use crate::gui::panels::{
    ContentAction, RmDialogAction, SettingsDialogAction, SidebarAction, TopBarAction,
    open_rm_dialog_batch, open_settings, show_content, show_footer, show_rm_dialog,
    show_settings_dialog, show_sidebar, show_top_bar, validate_add_form,
};
use crate::gui::settings_store::{self, from_state};
use crate::gui::state::{AppState, LinkSnapshot, MainView};
use crate::gui::theme;
use eframe::CreationContext;
use std::collections::HashSet;
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
    store: DataStore,
    snapshot: LinkSnapshot,
    needs_reload: bool,
    toast_frames: u32,
    saved_settings: crate::domain::gui_settings::GuiSettings,
}

impl SymmApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        let mut state = AppState::default();
        let saved_settings = settings_store::load_into(&mut state);
        if let Some(ppp) = cc.egui_ctx.native_pixels_per_point().filter(|&p| p > 0.0) {
            cc.egui_ctx.set_pixels_per_point(ppp);
        }
        theme::apply(
            &cc.egui_ctx,
            state.theme,
            state.color_scheme,
            state.font_size_pt,
        );
        let mut app = Self {
            state,
            store: DataStore::new(),
            snapshot: LinkSnapshot::default(),
            needs_reload: true,
            toast_frames: 0,
            saved_settings,
        };
        app.reload_data();
        app
    }

    fn persist_settings_if_changed(&mut self) {
        let current = from_state(&self.state);
        if current == self.saved_settings {
            return;
        }
        if let Ok(()) = settings_store::save_state(&self.state) {
            self.saved_settings = current;
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        theme::apply(
            ctx,
            self.state.theme,
            self.state.color_scheme,
            self.state.font_size_pt,
        );
    }

    fn reload_data(&mut self) {
        match self.store.reload() {
            Ok(snapshot) => {
                self.snapshot = snapshot;
                self.state.db_error = None;
                let valid: HashSet<i64> = self.snapshot.views.iter().map(|v| v.id).collect();
                self.state.checked_ids.retain(|id| valid.contains(id));
                if self.state.selected_id.is_some()
                    && !valid.contains(&self.state.selected_id.unwrap())
                {
                    self.state.selected_id = None;
                }
            }
            Err(err) => {
                self.snapshot = LinkSnapshot::default();
                self.state.db_error = Some(self.state.texts().db_open_failed(&err.to_string()));
            }
        }
        self.needs_reload = false;
    }

    fn toast(&mut self, msg: impl Into<String>, frames: u32) {
        self.state.toast = Some(msg.into());
        self.toast_frames = frames;
    }

    fn tick_toast(&mut self) {
        if self.toast_frames > 0 {
            self.toast_frames -= 1;
            if self.toast_frames == 0 {
                self.state.toast = None;
            }
        }
    }

    fn begin_rm_checked(&mut self) {
        let views: Vec<_> = self
            .state
            .checked_ids
            .iter()
            .filter_map(|id| self.snapshot.views.iter().find(|v| v.id == *id))
            .collect();
        if views.is_empty() {
            self.toast(self.state.texts().select_before_delete(), 240);
            return;
        }
        open_rm_dialog_batch(&mut self.state, &views);
    }

    fn confirm_rm(&mut self) {
        let Some(dialog) = self.state.rm_dialog.clone() else {
            return;
        };
        self.state.busy = true;
        match self.store.remove_links(&dialog.selectors, dialog.mode) {
            Ok(log) => {
                self.state.rm_dialog = None;
                self.needs_reload = true;
                self.state.selected_id = None;
                self.state.checked_ids.clear();
                self.state.main_view = MainView::Detail;
                let msg = log
                    .lines()
                    .next()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| self.state.texts().deleted().to_string());
                self.toast(msg, 360);
            }
            Err(err) => {
                let msg = self.state.texts().delete_failed(&err.to_string());
                self.toast(msg, 420);
            }
        }
        self.state.busy = false;
    }

    fn submit_add(&mut self) {
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
        let lock = form.lock_policy;
        let conflict = form.conflict_policy;
        self.state.busy = true;
        match self.store.add_link(&link, &target, &name, lock, conflict) {
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
                self.state.main_view = MainView::Detail;
                self.toast(t.link_created(), 300);
            }
            Err(err) => form.error = Some(err.to_string()),
        }
        self.state.busy = false;
    }
}

impl eframe::App for SymmApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let p = theme::resolve(self.state.theme, self.state.color_scheme);
        p.bg.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        if self.needs_reload {
            self.reload_data();
        }
        self.tick_toast();
        if self.state.busy {
            ctx.request_repaint();
        }
        self.persist_settings_if_changed();

        let p = theme::resolve(self.state.theme, self.state.color_scheme);

        // 1. 顶栏
        egui::TopBottomPanel::top(theme::TOP_BAR_PANEL_ID)
            .frame(theme::top_bar_frame(&p))
            .show(ctx, |ui| {
                let action = show_top_bar(ui, &self.state);
                match action {
                    TopBarAction::AddLink => self.state.main_view = MainView::Add,
                    TopBarAction::CycleTheme => {
                        self.state.theme = self.state.theme.next();
                    }
                    TopBarAction::CycleLocale => {
                        self.state.locale = self.state.locale.next();
                    }
                    TopBarAction::OpenSettings => open_settings(&mut self.state),
                    TopBarAction::None => {}
                }
            });

        // 2. 底栏（仅版本号）
        egui::TopBottomPanel::bottom(theme::FOOTER_PANEL_ID)
            .frame(theme::footer_frame(&p))
            .show(ctx, |ui| show_footer(ui, &self.state));

        // 3. 中间：左栏 + 可拖拽分隔 + 右内容
        let sidebar_max = theme::sidebar_max_width(ctx);
        self.state.sidebar_width = self
            .state
            .sidebar_width
            .clamp(theme::SIDEBAR_WIDTH_MIN, sidebar_max);

        let sidebar_resp = egui::SidePanel::left(theme::SIDEBAR_PANEL_ID)
            .resizable(true)
            .default_width(self.state.sidebar_width)
            .width_range(theme::SIDEBAR_WIDTH_MIN..=sidebar_max)
            .frame(theme::sidebar_frame(&p))
            .show(ctx, |ui| {
                let action = show_sidebar(ui, &mut self.state, &self.snapshot);
                match action {
                    SidebarAction::Refresh => {
                        self.needs_reload = true;
                        self.toast(self.state.texts().refreshed(), 180);
                    }
                    SidebarAction::DeleteChecked => self.begin_rm_checked(),
                    SidebarAction::None => {}
                }
            });
        if sidebar_resp.response.dragged() {
            self.state.sidebar_width = sidebar_resp.response.rect.width();
        }

        egui::CentralPanel::default()
            .frame(theme::central_panel_frame(&p))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    if self.state.busy {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(self.state.texts().busy());
                        });
                        ui.add_space(8.0);
                    }
                    if let Some(err) = &self.state.db_error {
                        ui.colored_label(
                            theme::status_color(crate::domain::model::LinkStatus::Missing),
                            err,
                        );
                        ui.add_space(8.0);
                    }
                    if let Some(msg) = &self.state.toast {
                        ui.label(msg);
                        ui.add_space(8.0);
                    }
                    let body_h = ui.available_height();
                    if body_h > 1.0 {
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), body_h),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                let selected_id = self.state.selected_id;
                                let view = self.snapshot.selected_view(selected_id);
                                if show_content(ui, &mut self.state, view)
                                    == ContentAction::SubmitAdd
                                {
                                    self.submit_add();
                                }
                            },
                        );
                    }
                });
            });

        match show_rm_dialog(ctx, &mut self.state) {
            RmDialogAction::Confirm => self.confirm_rm(),
            RmDialogAction::Cancel | RmDialogAction::None => {}
        }

        if show_settings_dialog(ctx, &mut self.state) == SettingsDialogAction::Apply {
            self.store.invalidate();
            self.needs_reload = true;
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
