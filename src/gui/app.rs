use crate::domain::gui_settings::GuiSettings;
use crate::gui::data::{self, DataStore};
use crate::gui::icon;
use crate::gui::panels::{
    AddAction, FooterAction, RmDialogAction, SidebarAction, TopBarAction, open_rm_dialog_batch,
    show_add, show_detail, show_detail_empty, show_footer, show_rm_dialog, show_sidebar,
    show_top_bar, validate_add_form,
};
use crate::gui::settings_store::{self, from_state};
use crate::gui::state::{AppState, LinkSnapshot, MainView};
use crate::gui::theme::{self, ThemePreference};
use crate::gui::util::open_path;
use eframe::CreationContext;
use std::collections::HashSet;
use std::path::PathBuf;

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
    last_theme: ThemePreference,
    saved_settings: GuiSettings,
}

impl SymmApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        let mut state = AppState::default();
        let saved_settings = settings_store::load_into(&mut state);
        theme::apply(&cc.egui_ctx, state.theme);
        let last_theme = state.theme;
        let mut app = Self {
            state,
            store: DataStore::new(),
            snapshot: LinkSnapshot::default(),
            needs_reload: true,
            toast_frames: 0,
            last_theme,
            saved_settings,
        };
        app.state.data_home = data::data_home_display().ok().map(PathBuf::from);
        app.reload_data();
        app
    }

    fn persist_settings_if_changed(&mut self) {
        let current = from_state(&self.state);
        if current == self.saved_settings {
            return;
        }
        match settings_store::save_state(&self.state) {
            Ok(()) => {
                self.saved_settings = current;
            }
            Err(err) => {
                self.toast(format!("设置保存失败：{err}"), 360);
            }
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let pref = self.state.theme;
        if pref == ThemePreference::System {
            theme::apply(ctx, pref);
            return;
        }
        if self.last_theme != pref {
            theme::apply(ctx, pref);
            self.last_theme = pref;
        }
    }

    fn reload_data(&mut self) {
        match self.store.reload() {
            Ok(snapshot) => {
                self.snapshot = snapshot;
                self.state.db_error = None;
                let valid: HashSet<i64> = self.snapshot.views.iter().map(|v| v.id).collect();
                self.state.checked_ids.retain(|id| valid.contains(id));
                self.state.expanded_ids.retain(|id| valid.contains(id));
                if self.state.selected_id.is_some()
                    && !valid.contains(&self.state.selected_id.unwrap())
                {
                    self.state.selected_id = None;
                }
            }
            Err(err) => {
                self.snapshot = LinkSnapshot::default();
                self.state.db_error = Some(format!("无法打开数据库：{err}"));
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
            self.toast("请先勾选要删除的链接", 240);
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
                let msg = if log.is_empty() {
                    "已删除".to_string()
                } else {
                    log.lines().next().unwrap_or("已删除").to_string()
                };
                self.toast(msg, 360);
            }
            Err(err) => {
                self.toast(format!("删除失败：{err}"), 420);
            }
        }
        self.state.busy = false;
    }

    fn submit_add(&mut self) {
        let form = &mut self.state.add_form;
        form.error = None;
        form.status_message = None;
        let Ok((link, target)) = validate_add_form(form) else {
            form.error = Some("请填写链接路径与目标路径".to_string());
            return;
        };
        let name = form.name.trim().to_string();
        let lock = form.lock_policy;
        let conflict = form.conflict_policy;
        self.state.busy = true;
        match self.store.add_link(&link, &target, &name, lock, conflict) {
            Ok(log) => {
                form.error = None;
                form.status_message = Some(if log.is_empty() {
                    "已添加".to_string()
                } else {
                    log.lines().last().unwrap_or("已添加").to_string()
                });
                form.link_path.clear();
                form.target_path.clear();
                form.name.clear();
                self.needs_reload = true;
                self.state.main_view = MainView::Detail;
                self.toast("链接已创建", 300);
            }
            Err(err) => {
                form.error = Some(err.to_string());
            }
        }
        self.state.busy = false;
    }

    fn handle_top_bar(&mut self, action: TopBarAction) {
        if action == TopBarAction::AddLink {
            self.state.main_view = MainView::Add;
        }
    }
}

impl eframe::App for SymmApp {
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

        match show_rm_dialog(ctx, &mut self.state) {
            RmDialogAction::Confirm => self.confirm_rm(),
            RmDialogAction::Cancel | RmDialogAction::None => {}
        }

        let dark = self.state.theme.is_dark();

        egui::TopBottomPanel::top("top_bar")
            .frame(theme::top_bar_frame(dark))
            .show_separator_line(false)
            .show(ctx, |ui| {
                let action = show_top_bar(ui, &self.state);
                if action == TopBarAction::CycleTheme {
                    self.state.theme = self.state.theme.next();
                    theme::apply(ctx, self.state.theme);
                    self.last_theme = self.state.theme;
                } else {
                    self.handle_top_bar(action);
                }
            });

        egui::TopBottomPanel::bottom("footer")
            .frame(theme::footer_frame(dark))
            .show_separator_line(false)
            .show(ctx, |ui| {
                theme::paint_hairline(ui, false);
                let footer_action = show_footer(ui, &self.state, &self.snapshot);
                if footer_action == FooterAction::OpenDataDir
                    && let Some(home) = &self.state.data_home
                {
                    let _ = open_path(home);
                }
            });

        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(self.state.sidebar_width)
            .width_range(240.0..=480.0)
            .frame(theme::sidebar_frame(dark))
            .show_separator_line(false)
            .show(ctx, |ui| {
                let action = show_sidebar(ui, &mut self.state, &self.snapshot);
                match action {
                    SidebarAction::Refresh => {
                        self.needs_reload = true;
                        self.toast("已刷新", 180);
                    }
                    SidebarAction::DeleteChecked => self.begin_rm_checked(),
                    SidebarAction::None => {}
                }
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::workspace_color(dark))
                    .inner_margin(20.0),
            )
            .show(ctx, |ui| {
                if self.state.busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("处理中…");
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
                match self.state.main_view {
                    MainView::Add => {
                        if show_add(ui, &mut self.state) == AddAction::Submit {
                            self.submit_add();
                        }
                    }
                    MainView::Detail => {
                        if let Some(view) =
                            self.snapshot.selected_view(self.state.selected_id).cloned()
                        {
                            show_detail(ui, &view);
                        } else {
                            show_detail_empty(ui);
                        }
                    }
                }
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
