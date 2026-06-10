use crate::gui::icons::Icon;
use crate::gui::theme::{self, rich_body, rich_body_muted};
use crate::gui::widgets::button;
use egui::{Align, Frame, Layout, Margin, Ui};

const CARD_INNER_MARGIN: f32 = 12.0;

/// 右对齐区域：用于工具栏、底栏、弹窗 footer 的右侧动作。
pub fn right_aligned<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.with_layout(Layout::right_to_left(Align::Center), add)
        .inner
}

/// 左右分区行：左侧内容自然排列，右侧动作贴右。
pub fn split_row<L, R>(
    ui: &mut Ui,
    add_left: impl FnOnce(&mut Ui) -> L,
    add_right: impl FnOnce(&mut Ui) -> R,
) -> (L, R) {
    ui.horizontal(|ui| {
        let left = add_left(ui);
        let right = right_aligned(ui, add_right);
        (left, right)
    })
    .inner
}

/// 内容卡片（egui [`Frame::group`]）。
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let width = ui.available_width();
    let height = ui.available_height();
    Frame::group(ui.style())
        .inner_margin(Margin::same(CARD_INNER_MARGIN))
        .show(ui, |ui| {
            ui.set_min_width((width - 2.0 * CARD_INNER_MARGIN).max(1.0));
            ui.set_min_height((height - 2.0 * CARD_INNER_MARGIN).max(1.0));
            add(ui)
        })
        .inner
}

/// 设置页等内容区：只提供内边距，不绘制左右边框。
pub fn settings_content_frame<R>(ui: &mut Ui, pad: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::none()
        .inner_margin(Margin::same(pad))
        .show(ui, add)
        .inner
}

/// 详情只读字段。
pub fn detail_field(ui: &mut Ui, p: &theme::UiPalette, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(rich_body_muted(&format!("{label}:"), p.text_muted));
        let resp = ui.add(egui::Label::new(rich_body(value, p.text)).wrap());
        if value.chars().count() > 24 {
            resp.on_hover_text(value);
        }
    });
    ui.add_space(theme::gap_sm(ui));
}

/// 路径详情字段：值可选择，右侧动作可直接复制完整路径。
pub fn detail_path_field(
    ui: &mut Ui,
    p: &theme::UiPalette,
    label: &str,
    value: &str,
    copy_tip: &str,
) {
    split_row(
        ui,
        |ui| {
            ui.label(rich_body_muted(&format!("{label}:"), p.text_muted));
        },
        |ui| {
            if button(ui).icon(Icon::Copy).tip(copy_tip).show().clicked() {
                ui.ctx().copy_text(value.to_string());
            }
        },
    );
    ui.add(
        egui::Label::new(rich_body(value, p.text))
            .selectable(true)
            .wrap(),
    );
    ui.add_space(theme::gap_sm(ui));
}
