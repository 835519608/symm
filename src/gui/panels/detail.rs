use crate::domain::model::LinkView;
use crate::gui::theme;
use crate::gui::widgets::{card, card_header};
use egui::{RichText, Ui};

pub fn show_detail_empty(ui: &mut Ui) {
    let dark = theme::is_dark_ui(ui);
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.32);
        let size = egui::vec2(56.0, 56.0);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter().rect(
            rect,
            egui::Rounding::same(16.0),
            theme::control_hover(dark),
            egui::Stroke::NONE,
        );
        crate::gui::widgets::paint_icon(
            ui,
            rect.shrink2(egui::Vec2::splat(14.0)),
            crate::gui::widgets::Icon::Detail,
            theme::text_muted(dark),
        );
        ui.add_space(14.0);
        ui.label(
            RichText::new("在左侧选择一条链接以查看详情")
                .size(15.0)
                .color(theme::secondary_text(ui)),
        );
    });
}

pub fn show_detail(ui: &mut Ui, view: &LinkView) {
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
