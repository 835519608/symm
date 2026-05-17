use crate::gui::state::{AppState, LinkSnapshot, MainView};
use crate::gui::theme;
use egui::{RichText, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarAction {
    Refresh,
    None,
}

pub fn show_sidebar(ui: &mut Ui, state: &mut AppState, snapshot: &LinkSnapshot) -> SidebarAction {
    let mut action = SidebarAction::None;
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("链接库")
                    .size(12.0)
                    .color(theme::secondary_text(ui)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("↻")
                    .on_hover_text("重新加载数据库")
                    .clicked()
                {
                    action = SidebarAction::Refresh;
                }
            });
        });
        ui.add_space(4.0);

        ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .hint_text("搜索名称或路径…")
                .desired_width(ui.available_width()),
        );
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let items = snapshot.filtered(&state.search);
                if items.is_empty() {
                    ui.label(
                        RichText::new("暂无链接。使用顶栏「添加链接」或 CLI：symm add")
                            .color(theme::secondary_text(ui))
                            .size(12.0),
                    );
                    return;
                }

                for view in items {
                    let id = view.id;
                    let name = view.display_name();
                    let selected = state.selected_id == Some(id);
                    let mut open = state.expanded_ids.contains(&id)
                        || (selected && state.main_view == MainView::Detail);

                    ui.push_id(id, |ui| {
                        ui.horizontal(|ui| {
                            let toggle = ui.selectable_label(open, if open { "▾" } else { "▸" });
                            if toggle.clicked() {
                                if open {
                                    state.expanded_ids.remove(&id);
                                } else {
                                    state.expanded_ids.insert(id);
                                }
                                open = !open;
                            }
                            let resp = ui.selectable_label(
                                selected,
                                RichText::new(&name).size(13.0).color(if selected {
                                    theme::ACCENT
                                } else {
                                    theme::primary_text(ui)
                                }),
                            );
                            if resp.clicked() {
                                state.selected_id = Some(id);
                                state.expanded_ids.insert(id);
                                state.main_view = MainView::Detail;
                            }
                        });

                        if open {
                            state.expanded_ids.insert(id);
                            ui.indent("paths", |ui| {
                                path_line(ui, "link", &view.link_path);
                                path_line(ui, "target", &view.target_path);
                            });
                        }
                    });
                }
            });
    });
    action
}

fn path_line(ui: &mut Ui, label: &str, path: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!("{label}: "))
                .size(11.0)
                .color(theme::secondary_text(ui)),
        );
        ui.label(
            RichText::new(path)
                .size(11.0)
                .color(theme::primary_text(ui)),
        );
    });
}
