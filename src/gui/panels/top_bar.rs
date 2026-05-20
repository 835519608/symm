use crate::gui::state::AppState;
use crate::gui::theme;
use crate::gui::widgets::{Icon, icon_button, toolbar_button};
use egui::Ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBarAction {
    AddLink,
    CycleTheme,
    None,
}

pub fn show_top_bar(ui: &mut Ui, state: &AppState) -> TopBarAction {
    let mut action = TopBarAction::None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        if toolbar_button(ui, Icon::Add, "添加链接", "创建新软链").clicked() {
            action = TopBarAction::AddLink;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let theme_tip = format!("主题：{}（点击切换到下一项）", state.theme.label());
            if icon_button(ui, state.theme.toggle_icon(), &theme_tip).clicked() {
                action = TopBarAction::CycleTheme;
            }
        });
    });
    theme::paint_hairline(ui, true);
    action
}
