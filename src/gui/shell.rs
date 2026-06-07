use crate::gui::panels::{
    SidebarAction, TopBarAction, open_add_dialog, open_settings, show_add_dialog, show_content,
    show_footer, show_rm_dialog, show_settings_dialog, show_sidebar, show_top_bar,
};
use crate::gui::state::{AppState, LinkSnapshot};
use crate::gui::theme;

pub use crate::gui::panels::{AddDialogAction, RmDialogAction, SettingsDialogAction};

#[derive(Debug, Default)]
pub struct FrameActions {
    pub refresh_requested: bool,
    pub delete_checked_requested: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DialogActions {
    pub add: AddDialogAction,
    pub rm: RmDialogAction,
    pub settings: SettingsDialogAction,
}

pub fn show_frame(
    ctx: &egui::Context,
    state: &mut AppState,
    snapshot: &LinkSnapshot,
) -> FrameActions {
    let p = theme::resolve(state.theme, state.color_scheme);
    let mut actions = FrameActions::default();

    egui::TopBottomPanel::top(theme::TOP_BAR_PANEL_ID)
        .frame(theme::top_bar_frame(&p))
        .show(ctx, |ui| match show_top_bar(ui, state) {
            TopBarAction::AddLink => open_add_dialog(state),
            TopBarAction::CycleTheme => {
                state.theme = state.theme.next();
            }
            TopBarAction::CycleLocale => {
                state.locale = state.locale.next();
            }
            TopBarAction::OpenSettings => open_settings(state),
            TopBarAction::None => {}
        });

    let footer_resp = egui::TopBottomPanel::bottom(theme::FOOTER_PANEL_ID)
        .frame(theme::footer_frame(&p))
        .show(ctx, |ui| show_footer(ui, state));

    show_sidebar_panel(ctx, state, snapshot, &mut actions, &p);
    show_central_panel(ctx, state, snapshot, &p);
    theme::paint_footer_separator(ctx, footer_resp.response.rect, &p);

    actions
}

pub fn show_dialogs(ctx: &egui::Context, state: &mut AppState) -> DialogActions {
    DialogActions {
        add: show_add_dialog(ctx, state),
        rm: show_rm_dialog(ctx, state),
        settings: show_settings_dialog(ctx, state),
    }
}

fn show_sidebar_panel(
    ctx: &egui::Context,
    state: &mut AppState,
    snapshot: &LinkSnapshot,
    actions: &mut FrameActions,
    p: &theme::UiPalette,
) {
    let sidebar_max = theme::sidebar_max_width(ctx);
    state.sidebar_width = state
        .sidebar_width
        .clamp(theme::SIDEBAR_WIDTH_MIN, sidebar_max);

    let sidebar_resp = egui::SidePanel::left(theme::SIDEBAR_PANEL_ID)
        .resizable(true)
        .default_width(state.sidebar_width)
        .width_range(theme::SIDEBAR_WIDTH_MIN..=sidebar_max)
        .frame(theme::sidebar_frame(p))
        .show(ctx, |ui| match show_sidebar(ui, state, snapshot) {
            SidebarAction::Refresh => {
                actions.refresh_requested = true;
            }
            SidebarAction::DeleteChecked => {
                actions.delete_checked_requested = true;
            }
            SidebarAction::None => {}
        });
    if sidebar_resp.response.dragged() {
        state.sidebar_width = sidebar_resp.response.rect.width();
    }
}

fn show_central_panel(
    ctx: &egui::Context,
    state: &AppState,
    snapshot: &LinkSnapshot,
    p: &theme::UiPalette,
) {
    egui::CentralPanel::default()
        .frame(theme::central_panel_frame(p))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                if state.busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(state.texts().busy());
                    });
                    ui.add_space(theme::gap(ui));
                }
                if let Some(err) = &state.db_error {
                    ui.colored_label(
                        theme::status_color_for_palette(
                            crate::domain::model::LinkStatus::Missing,
                            p,
                        ),
                        err,
                    );
                    ui.add_space(theme::gap(ui));
                }
                if let Some(msg) = &state.toast {
                    ui.label(msg);
                    ui.add_space(theme::gap(ui));
                }
                let selected_id = state.selected_id;
                let view = snapshot.selected_view(selected_id);
                show_content(ui, state, view);
            });
        });
}
