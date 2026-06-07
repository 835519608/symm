use crate::gui::theme::{self, UiPalette, rich_body};
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
    ui.vertical(|ui| {
        ui.set_min_width(column_width);
        for (idx, &(value, label)) in items.iter().enumerate() {
            if idx > 0 {
                ui.add_space(spacing);
            }
            let selected = *current == value;
            ui.selectable_value(
                current,
                value,
                rich_body(label, if selected { p.text } else { p.text_muted }),
            );
        }
    });
}
