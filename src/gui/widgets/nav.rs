use egui::Ui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NavAxis {
    Vertical,
    #[allow(dead_code)]
    Horizontal,
}

fn selectable_list_sized<T>(
    ui: &mut Ui,
    current: &mut T,
    items: &[(T, &str)],
    axis: NavAxis,
    spacing: f32,
    column_width: Option<f32>,
) where
    T: PartialEq + Copy,
{
    let add_items = |ui: &mut Ui| {
        if let Some(w) = column_width {
            ui.set_width(w);
            ui.set_min_width(w);
        }
        for (i, &(value, label)) in items.iter().enumerate() {
            if i > 0 {
                ui.add_space(spacing);
            }
            ui.selectable_value(current, value, label);
        }
    };

    match axis {
        NavAxis::Vertical => {
            ui.vertical(add_items);
        }
        NavAxis::Horizontal => {
            ui.horizontal(add_items);
        }
    }
}

/// 设置弹窗侧栏：竖向互斥导航。
pub fn settings_nav<T>(ui: &mut Ui, current: &mut T, items: &[(T, &str)], column_width: f32)
where
    T: PartialEq + Copy,
{
    selectable_list_sized(
        ui,
        current,
        items,
        NavAxis::Vertical,
        4.0,
        Some(column_width),
    );
}
