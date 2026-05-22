use crate::gui::theme::{self, rich_body, rich_body_muted, rich_heading, rich_small};
use egui::{Frame, Margin, Ui};

/// 表单页：占满主区可用宽度，各行控件右缘对齐。
pub fn form_page<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let w = ui.available_width().max(200.0);
    ui.vertical(|ui| {
        ui.set_width(w);
        ui.set_min_width(w);
        ui.set_max_width(w);
        add(ui)
    })
    .inner
}

/// 主区标题 + 副标题。
pub fn page_heading(ui: &mut Ui, p: &theme::UiPalette, title: &str, subtitle: Option<&str>) {
    ui.label(rich_heading(title, p.text));
    if let Some(sub) = subtitle {
        ui.label(rich_body_muted(sub, p.text_muted));
    }
}

/// 操作按钮行（统一间距）。
pub fn button_row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0 * theme::typography_from_ui(ui).scale;
        add(ui)
    })
    .inner
}

/// 内容卡片（egui [`Frame::group`]）。
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::group(ui.style())
        .inner_margin(Margin::same(12.0))
        .show(ui, add)
        .inner
}

/// 详情只读字段。
pub fn detail_field(ui: &mut Ui, p: &theme::UiPalette, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(rich_body_muted(&format!("{label}:"), p.text_muted));
        ui.label(rich_body(value, p.text));
    });
    ui.add_space(6.0 * theme::typography_from_ui(ui).scale);
}

/// 居中空状态提示。
pub fn empty_hint(ui: &mut Ui, p: &theme::UiPalette, text: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.28);
        card(ui, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(rich_small(text, p.text_muted));
            });
        });
    });
}
