use crate::gui::data::{self, DataStore};
use crate::gui::panels::{
    TopBarAction, show_dashboard, show_detail, show_footer, show_sidebar, show_top_bar,
};
use crate::gui::state::{AppState, LinkSnapshot, MainView};
use crate::gui::theme;
use eframe::CreationContext;
use std::path::PathBuf;

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("symm"),
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
}

impl SymmApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut app = Self {
            state: AppState::default(),
            store: DataStore::new(),
            snapshot: LinkSnapshot::default(),
            needs_reload: true,
            toast_frames: 0,
        };
        app.state.data_home = data::data_home_display().ok().map(PathBuf::from);
        app.reload_data();
        app
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
                    self.state.main_view = MainView::Dashboard;
                }
            }
            Err(err) => {
                self.snapshot = LinkSnapshot::default();
                self.state.db_error = Some(format!("无法打开数据库：{err}"));
            }
        }
        self.needs_reload = false;
    }

    fn handle_top_bar(&mut self, action: TopBarAction) {
        match action {
            TopBarAction::Refresh | TopBarAction::ListAll => {
                self.needs_reload = true;
                self.state.toast = Some("已刷新".to_string());
                self.toast_frames = 180;
            }
            TopBarAction::ShowDetail => {
                if self.state.selected_id.is_some() {
                    self.state.main_view = MainView::Detail;
                } else {
                    self.state.toast = Some("请先在左侧选择一条链接".to_string());
                    self.toast_frames = 240;
                }
            }
            TopBarAction::AddLink => {
                self.state.toast = Some("请使用终端：symm add <link> <target>".to_string());
                self.toast_frames = 300;
            }
            TopBarAction::Remove => {
                if let Some(view) = self.snapshot.selected_view(self.state.selected_id) {
                    self.state.toast = Some(format!("请使用终端：symm rm {}", view.display_name()));
                } else {
                    self.state.toast = Some("请先选择要删除的链接".to_string());
                }
                self.toast_frames = 300;
            }
            TopBarAction::OpenDataDir => {
                if let Ok(home) = data::data_home_display() {
                    let _ = open::that(&home);
                    self.state.toast = Some(format!("已尝试打开 {home}"));
                } else {
                    self.state.toast = Some("无法解析数据目录".to_string());
                }
                self.toast_frames = 240;
            }
            TopBarAction::None => {}
        }
    }

    fn tick_toast(&mut self) {
        if self.toast_frames > 0 {
            self.toast_frames -= 1;
            if self.toast_frames == 0 {
                self.state.toast = None;
            }
        }
    }
}

impl eframe::App for SymmApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.needs_reload {
            self.reload_data();
        }
        self.tick_toast();

        egui::TopBottomPanel::top("top_bar")
            .frame(theme::panel_frame())
            .show(ctx, |ui| {
                let action = show_top_bar(ui, &self.state);
                self.handle_top_bar(action);
            });

        egui::TopBottomPanel::bottom("footer")
            .frame(theme::panel_frame().inner_margin(egui::Margin::symmetric(12.0, 4.0)))
            .show(ctx, |ui| show_footer(ui, &self.state));

        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(self.state.sidebar_width)
            .width_range(200.0..=420.0)
            .frame(theme::panel_frame().inner_margin(egui::Margin::symmetric(10.0, 8.0)))
            .show(ctx, |ui| show_sidebar(ui, &mut self.state, &self.snapshot));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_WORKSPACE)
                    .inner_margin(16.0),
            )
            .show(ctx, |ui| match self.state.main_view {
                MainView::Dashboard => {
                    show_dashboard(ui, &mut self.state, &self.snapshot);
                }
                MainView::Detail => {
                    if let Some(view) = self.snapshot.selected_view(self.state.selected_id).cloned()
                    {
                        show_detail(ui, &mut self.state, &view);
                    } else {
                        self.state.main_view = MainView::Dashboard;
                        show_dashboard(ui, &mut self.state, &self.snapshot);
                    }
                }
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

// 轻量打开目录：无额外依赖时用平台命令
mod open {
    use crate::domain::error::SymmError;

    pub fn that(path: &str) -> Result<(), SymmError> {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(path)
                .spawn()
                .map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(path)
                .spawn()
                .map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            std::process::Command::new("xdg-open")
                .arg(path)
                .spawn()
                .map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
        }
        Ok(())
    }
}
