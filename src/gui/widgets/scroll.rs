use egui::Ui;

/// 垂直滚动：内容未超出时不出现滚动条（`auto_shrink` 高度随内容收缩）。
pub fn vertical_when_overflow<R>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let max_h = ui.available_height();
    if max_h <= 1.0 {
        return add(ui);
    }
    egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .max_height(max_h)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let w = ui.available_width().max(200.0);
            ui.set_width(w);
            ui.set_min_width(w);
            add(ui)
        })
        .inner
}
