use crate::gui::state::AppState;
use crate::gui::theme;
use egui::{RichText, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBarAction {
    Refresh,
    AddLink,
    ListAll,
    ShowDetail,
    Remove,
    OpenDataDir,
    None,
}

pub fn show_top_bar(ui: &mut Ui, _state: &AppState) -> TopBarAction {
    let mut action = TopBarAction::None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        action = action.or(tool_btn(
            ui,
            "➕ 添加链接",
            "symm add",
            TopBarAction::AddLink,
        ));
        action = action.or(tool_btn(
            ui,
            "📋 列表",
            "刷新链接列表",
            TopBarAction::ListAll,
        ));
        action = action.or(tool_btn(
            ui,
            "🔍 详情",
            "查看选中链接",
            TopBarAction::ShowDetail,
        ));
        action = action.or(tool_btn(ui, "🗑 删除", "删除选中链接", TopBarAction::Remove));
        ui.separator();
        action = action.or(tool_btn(
            ui,
            "↻ 刷新",
            "重新加载数据库",
            TopBarAction::Refresh,
        ));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("⚙").on_hover_text("数据目录").clicked() {
                action = TopBarAction::OpenDataDir;
            }
            ui.label(
                RichText::new("symm")
                    .strong()
                    .color(theme::ACCENT)
                    .size(14.0),
            );
        });
    });
    action
}

fn tool_btn(ui: &mut Ui, label: &str, tip: &str, act: TopBarAction) -> TopBarAction {
    if ui
        .add(egui::Button::new(RichText::new(label).size(13.0)).fill(egui::Color32::TRANSPARENT))
        .on_hover_text(tip)
        .clicked()
    {
        act
    } else {
        TopBarAction::None
    }
}

trait OrAction {
    fn or(self, other: TopBarAction) -> TopBarAction;
}

impl OrAction for TopBarAction {
    fn or(self, other: TopBarAction) -> TopBarAction {
        if self != TopBarAction::None {
            self
        } else {
            other
        }
    }
}
