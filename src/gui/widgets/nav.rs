use crate::gui::theme::{self, UiPalette};
use crate::gui::widgets::{SelectableTextStyle, selectable_text_row};
use egui::Ui;

/// 设置弹窗侧栏：使用 egui 原生 selectable 行。
pub fn settings_nav<T>(
    ui: &mut Ui,
    p: &UiPalette,
    current: &mut T,
    items: &[(T, &str)],
    column_width: f32,
) where
    T: PartialEq + Copy,
{
    let spacing = 2.0 * theme::typography_from_ui(ui).scale;
    let row_h = theme::typography_from_ui(ui).field_row_h;
    ui.vertical(|ui| {
        ui.set_min_width(column_width);
        for (idx, &(value, label)) in items.iter().enumerate() {
            if idx > 0 {
                ui.add_space(spacing);
            }
            let selected = *current == value;
            if full_width_nav_item(ui, p, selected, label, column_width, row_h).clicked() {
                *current = value;
            }
        }
    });
}

fn full_width_nav_item(
    ui: &mut Ui,
    p: &UiPalette,
    selected: bool,
    label: &str,
    width: f32,
    height: f32,
) -> egui::Response {
    let typo = theme::typography_from_ui(ui);
    selectable_text_row(
        ui,
        p,
        selected,
        ui.is_enabled(),
        label,
        egui::vec2(width, height),
        SelectableTextStyle::new(p.text_muted, typo.button_font()),
    )
}
