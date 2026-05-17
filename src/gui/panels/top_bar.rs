use crate::gui::state::AppState;
use crate::gui::theme;
use egui::{RichText, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBarAction {
    AddLink,
    ListAll,
    ShowDetail,
    Remove,
    ToggleSettings,
    None,
}

pub fn show_top_bar(ui: &mut Ui, state: &AppState) -> TopBarAction {
    let mut action = TopBarAction::None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        action = action.or(tool_btn(
            ui,
            "➕ 添加链接",
            "创建新软链",
            TopBarAction::AddLink,
        ));
        action = action.or(tool_btn(
            ui,
            "📋 列表",
            "查看全部链接表格",
            TopBarAction::ListAll,
        ));
        let detail_tip = if state.selected_id.is_some() {
            "查看当前选中链接详情"
        } else {
            "请先在左侧选择链接"
        };
        action = action.or(tool_btn(
            ui,
            "🔍 详情",
            detail_tip,
            TopBarAction::ShowDetail,
        ));
        action = action.or(tool_btn(
            ui,
            "🗑 删除",
            "删除选中的链接",
            TopBarAction::Remove,
        ));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let settings_label = if state.settings_open {
                "⚙ 设置 ▾"
            } else {
                "⚙ 设置"
            };
            if ui
                .small_button(settings_label)
                .on_hover_text("外观：浅色 / 深色 / 跟随系统")
                .clicked()
            {
                action = TopBarAction::ToggleSettings;
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
