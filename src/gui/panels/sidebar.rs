use crate::domain::model::LinkView;
use crate::gui::panels::rm_dialog::open_rm_dialog;
use crate::gui::state::{AppState, LinkSnapshot, MainView};
use crate::gui::theme;
use crate::gui::widgets::{Icon, icon_button, search_field};
use egui::{RichText, Sense, Stroke, Ui, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarAction {
    Refresh,
    DeleteChecked,
    None,
}

pub fn show_sidebar(ui: &mut Ui, state: &mut AppState, snapshot: &LinkSnapshot) -> SidebarAction {
    let mut action = SidebarAction::None;
    let dark = theme::is_dark_ui(ui);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("链接库")
                    .strong()
                    .size(13.0)
                    .color(theme::primary_text(ui)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if icon_button(ui, Icon::Refresh, "重新加载数据库").clicked() {
                    action = SidebarAction::Refresh;
                }
                if ui.small_button("收起").clicked() {
                    state.expanded_ids.clear();
                }
                if ui.small_button("展开").clicked() {
                    state
                        .expanded_ids
                        .extend(snapshot.views.iter().map(|v| v.id));
                }
            });
        });

        if !state.checked_ids.is_empty() {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let n = state.checked_ids.len();
                if ui
                    .button(RichText::new(format!("删除选中 ({n})")).color(theme::ACCENT))
                    .clicked()
                {
                    action = SidebarAction::DeleteChecked;
                }
                if ui.small_button("取消选择").clicked() {
                    state.checked_ids.clear();
                }
            });
        }

        ui.add_space(8.0);
        search_field(ui, &mut state.search, "搜索名称…", ui.available_width());
        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let items = snapshot.filtered_by_name(&state.search);
                if items.is_empty() {
                    let hint = if state.search.trim().is_empty() {
                        "暂无链接。使用顶栏「添加链接」或 CLI：symm add"
                    } else {
                        "无匹配名称"
                    };
                    ui.label(
                        RichText::new(hint)
                            .color(theme::secondary_text(ui))
                            .size(12.0),
                    );
                    return;
                }

                for view in items {
                    render_link_row(ui, state, view, dark);
                    ui.add_space(2.0);
                }
            });
    });

    theme::paint_sidebar_edge(ui);
    action
}

fn render_link_row(ui: &mut Ui, state: &mut AppState, view: &LinkView, dark: bool) {
    let id = view.id;
    let name = view.display_name();
    let selected = state.selected_id == Some(id);
    let mut open = state.expanded_ids.contains(&id);

    ui.push_id(id, |ui| {
        let row_h = 32.0;
        let (row_rect, row_resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());

        if selected {
            ui.painter().rect(
                row_rect,
                theme::control_rounding(),
                theme::ACCENT.gamma_multiply(if dark { 0.22 } else { 0.12 }),
                Stroke::NONE,
            );
        } else if row_resp.hovered() {
            ui.painter().rect(
                row_rect,
                theme::control_rounding(),
                theme::control_hover(dark),
                Stroke::NONE,
            );
        }

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(row_rect), |ui| {
            ui.horizontal(|ui| {
                let mut checked = state.checked_ids.contains(&id);
                let cb = ui.add(egui::Checkbox::without_text(&mut checked));
                if cb.changed() {
                    if checked {
                        state.checked_ids.insert(id);
                    } else {
                        state.checked_ids.remove(&id);
                    }
                }

                let chevron = if open {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                };
                let chev_rect = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                crate::gui::widgets::paint_icon(
                    ui,
                    chev_rect.0.shrink2(Vec2::splat(5.0)),
                    chevron,
                    theme::text_muted(dark),
                );
                if chev_rect.1.clicked() {
                    if open {
                        state.expanded_ids.remove(&id);
                    } else {
                        state.expanded_ids.insert(id);
                    }
                    open = !open;
                }

                let name_resp = ui.selectable_label(
                    false,
                    RichText::new(&name).size(13.0).color(if selected {
                        theme::ACCENT
                    } else {
                        theme::primary_text(ui)
                    }),
                );
                let mut del_clicked = false;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, Icon::Delete, "删除此链接").clicked() {
                        open_rm_dialog(state, view);
                        del_clicked = true;
                    }
                });
                if (name_resp.clicked() || row_resp.clicked())
                    && !chev_rect.1.clicked()
                    && !cb.clicked()
                    && !del_clicked
                {
                    state.selected_id = Some(id);
                    state.main_view = MainView::Detail;
                }
            });
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
