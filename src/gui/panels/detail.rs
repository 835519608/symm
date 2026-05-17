use crate::domain::model::LinkView;
use crate::gui::panels::rm_dialog::open_rm_dialog;
use crate::gui::state::AppState;
use crate::gui::theme;
use crate::gui::widgets::{card, card_header, subtle_button};
use egui::{RichText, Ui};

pub fn show_detail_empty(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.35);
        ui.label(
            RichText::new("在左侧选择一条链接以查看详情")
                .size(14.0)
                .color(theme::secondary_text(ui)),
        );
    });
}

pub fn show_detail(ui: &mut Ui, state: &mut AppState, view: &LinkView) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(view.display_name())
                    .strong()
                    .size(18.0)
                    .color(theme::primary_text(ui)),
            );
            ui.label(
                RichText::new(view.status.label_zh())
                    .size(13.0)
                    .color(theme::status_color(view.status)),
            );
        });
        ui.add_space(8.0);

        card(ui, |ui| {
            card_header(ui, "链接详情");
            detail_row(ui, "名称", view.name.clone());
            detail_row(ui, "类型", view.link_kind.label_zh().to_string());
            detail_row(ui, "状态", view.status.label_zh().to_string());
            detail_row(ui, "链接路径", view.link_path.clone());
            detail_row(ui, "目标路径", view.target_path.clone());
            detail_row(ui, "序号", view.index.to_string());
            detail_row(ui, "ID", view.id.to_string());
        });

        ui.add_space(theme::SPACING);
        ui.horizontal(|ui| {
            if subtle_button(ui, "删除此链接…").clicked() {
                let selector = if view.name.is_empty() {
                    view.id.to_string()
                } else {
                    view.name.clone()
                };
                open_rm_dialog(state, selector, view.display_name());
            }
        });
    });
}

fn detail_row(ui: &mut Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(72.0, 20.0),
            egui::Layout::left_to_right(egui::Align::TOP),
            |ui| {
                ui.label(
                    RichText::new(label)
                        .size(12.0)
                        .color(theme::secondary_text(ui)),
                );
            },
        );
        ui.label(
            RichText::new(value)
                .size(13.0)
                .color(theme::primary_text(ui)),
        );
    });
    ui.add_space(2.0);
}
