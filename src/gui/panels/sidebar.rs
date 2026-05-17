use crate::domain::model::LinkKind;
use crate::gui::state::{AppState, LinkSnapshot, MainView};
use crate::gui::theme;
use crate::gui::widgets::subtle_button;
use egui::{RichText, Ui};

pub fn show_sidebar(ui: &mut Ui, state: &mut AppState, snapshot: &LinkSnapshot) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("链接库")
                    .size(12.0)
                    .color(theme::TEXT_SECONDARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if subtle_button(ui, "↻").on_hover_text("刷新").clicked() {
                    state.toast = Some("请使用顶栏「刷新」".to_string());
                }
            });
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.search)
                    .hint_text("搜索…")
                    .desired_width(ui.available_width() - 52.0),
            );
            if ui
                .small_button("⌕")
                .on_hover_text("筛选（待实现）")
                .clicked()
            {
                state.toast = Some("状态筛选将在后续版本提供".to_string());
            }
        });
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let groups = snapshot.groups(&state.search);
                let auto_expand = groups.len() <= 3 && state.expanded_groups.is_empty();
                if groups.is_empty() {
                    ui.label(
                        RichText::new("暂无链接，使用「添加链接」或 CLI：symm add")
                            .color(theme::TEXT_SECONDARY)
                            .size(12.0),
                    );
                    return;
                }

                for (group, items) in &groups {
                    let mut open = state.expanded_groups.contains(group.as_str()) || auto_expand;
                    let id = ui.make_persistent_id(format!("grp-{group}"));
                    ui.push_id(id, |ui| {
                        ui.horizontal(|ui| {
                            let toggle = ui.selectable_label(open, if open { "▾" } else { "▸" });
                            if toggle.clicked() {
                                if open {
                                    state.expanded_groups.remove(group.as_str());
                                } else {
                                    state.expanded_groups.insert(group.clone());
                                }
                                open = !open;
                            }
                            ui.label(
                                RichText::new(format!("📁 {group}"))
                                    .size(12.0)
                                    .color(theme::TEXT_PRIMARY),
                            );
                        });
                        if open {
                            if !state.expanded_groups.contains(group.as_str()) {
                                state.expanded_groups.insert(group.clone());
                            }
                            ui.indent("tree", |ui| {
                                for view in items {
                                    let selected = state.selected_id == Some(view.id);
                                    let icon = kind_icon(view.link_kind);
                                    let name = view.display_name();
                                    let status = view.status.label_zh();
                                    let resp = ui.selectable_label(
                                        selected,
                                        RichText::new(format!("{icon} {name}")).size(13.0).color(
                                            if selected {
                                                theme::ACCENT
                                            } else {
                                                theme::TEXT_PRIMARY
                                            },
                                        ),
                                    );
                                    if resp.clicked() {
                                        state.selected_id = Some(view.id);
                                        state.main_view = MainView::Detail;
                                    }
                                    if resp.hovered() {
                                        resp.on_hover_text(format!(
                                            "{status} · {}",
                                            view.link_path
                                        ));
                                    }
                                }
                            });
                        }
                    });
                }
            });
    });
}

fn kind_icon(kind: LinkKind) -> &'static str {
    match kind {
        LinkKind::Symlink => "🔗",
        LinkKind::Junction => "📂",
    }
}
