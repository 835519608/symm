use crate::domain::gui_settings::GuiSettings;
use crate::gui::data::{self, DataStore};
use crate::gui::icon;
use crate::gui::panels::{
    AddAction, FooterAction, RmDialogAction, SidebarAction, TopBarAction, open_rm_dialog, show_add,
    show_detail, show_detail_empty, show_footer, show_list, show_rm_dialog, show_settings_window,
    show_sidebar, show_top_bar, validate_add_form,
};
use crate::gui::settings_store::{self, from_state};
use crate::gui::state::{AppState, LinkSnapshot, MainView};
use crate::gui::theme::{self, ThemePreference};
use crate::gui::util::open_path;
use eframe::CreationContext;
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
                if self.state.selected_id.is_some()
                    && self
                        .snapshot
                        .selected_view(self.state.selected_id)
                        .is_none()
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

    fn begin_rm(&mut self) {
        let Some(view) = self.snapshot.selected_view(self.state.selected_id) else {
            self.toast("请先在左侧选择要删除的链接", 240);
            return;
        };
        let selector = if view.name.is_empty() {
            view.id.to_string()
        } else {
            view.name.clone()
        };
        open_rm_dialog(&mut self.state, selector, view.display_name());
    }

    fn confirm_rm(&mut self) {
        let Some(dialog) = self.state.rm_dialog.clone() else {
            return;
        };
        self.state.busy = true;
        match self.store.remove_link(&dialog.selector, dialog.mode) {
            Ok(log) => {
                self.state.rm_dialog = None;
                self.needs_reload = true;
                self.state.selected_id = None;
                self.state.main_view = MainView::List;
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
                self.state.main_view = MainView::List;
                self.toast("链接已创建", 300);
            }
            Err(err) => {
                form.error = Some(err.to_string());
            }
        }
        self.state.busy = false;
    }

    fn handle_top_bar(&mut self, action: TopBarAction) {
        match action {
            TopBarAction::ListAll => {
                self.needs_reload = true;
                self.state.main_view = MainView::List;
                self.toast("已刷新列表", 180);
            }
            TopBarAction::ShowDetail => {
                if self.state.selected_id.is_some() {
                    self.state.main_view = MainView::Detail;
                } else {
                    self.toast("请先在左侧选择一条链接", 240);
                }
            }
            TopBarAction::AddLink => {
                self.state.main_view = MainView::Add;
            }
            TopBarAction::Remove => {
                self.begin_rm();
            }
            TopBarAction::ToggleSettings => {
                self.state.settings_open = !self.state.settings_open;
            }
            TopBarAction::None => {}
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

        show_settings_window(ctx, &mut self.state);
        self.persist_settings_if_changed();

        match show_rm_dialog(ctx, &mut self.state) {
            RmDialogAction::Confirm => self.confirm_rm(),
            RmDialogAction::Cancel | RmDialogAction::None => {}
        }

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            let frame = theme::panel_frame(ui);
            frame.show(ui, |ui| {
                let action = show_top_bar(ui, &self.state);
                self.handle_top_bar(action);
            });
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            let frame = theme::panel_frame(ui).inner_margin(egui::Margin::symmetric(12.0, 4.0));
            let footer_action = frame.show(ui, |ui| show_footer(ui, &self.state, &self.snapshot));
            if footer_action.inner == FooterAction::OpenDataDir
                && let Some(home) = &self.state.data_home
            {
                let _ = open_path(home);
            }
        });

        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(self.state.sidebar_width)
            .width_range(220.0..=480.0)
            .show(ctx, |ui| {
                let frame = theme::panel_frame(ui);
                frame
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        let action = show_sidebar(ui, &mut self.state, &self.snapshot);
                        if action == SidebarAction::Refresh {
                            self.needs_reload = true;
                            self.toast("已刷新", 180);
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::workspace_fill(ui))
                .inner_margin(16.0)
                .show(ui, |ui| {
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
                        MainView::List => {
                            show_list(ui, &mut self.state, &self.snapshot);
                        }
                        MainView::Add => {
                            if show_add(ui, &mut self.state) == AddAction::Submit {
                                self.submit_add();
                            }
                        }
                        MainView::Detail => {
                            if let Some(view) =
                                self.snapshot.selected_view(self.state.selected_id).cloned()
                            {
                                show_detail(ui, &mut self.state, &view);
                            } else {
                                show_detail_empty(ui);
                            }
                        }
                    }
                });
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
