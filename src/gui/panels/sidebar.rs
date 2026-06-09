use crate::domain::model::LinkView;
use crate::gui::icons::Icon;
use crate::gui::panels::rm_dialog::open_rm_dialog;
use crate::gui::state::{AppState, LinkSnapshot, PAGE_SIZE_OPTIONS};
use crate::gui::theme::{self, UiPalette, rich_section, rich_small};
use crate::gui::widgets::{button, right_aligned, search_field, split_row};
use egui::{Ui, WidgetInfo, WidgetType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarAction {
    Refresh,
    DeleteChecked,
    PageChanged,
    None,
}

pub fn show_sidebar(ui: &mut Ui, state: &mut AppState, snapshot: &LinkSnapshot) -> SidebarAction {
    let mut action = SidebarAction::None;
    let p = theme::resolve(state.theme, state.color_scheme);
    let t = state.texts();

    ui.vertical(|ui| {
        sidebar_header(ui, state, snapshot, &p, &t, &mut action);
        let typo = theme::typography_from_ui(ui);
        let pagination_h = typo.btn_h + 2.0 * theme::gap(ui);
        let list_h = (ui.available_height() - pagination_h).max(0.0);
        if list_h > 1.0 {
            sidebar_list(ui, state, snapshot, &p, &t, list_h);
        }
        ui.add_space(theme::gap(ui));
        sidebar_pagination(ui, state, snapshot, &p, &t, &mut action);
    });

    action
}

fn sidebar_header(
    ui: &mut Ui,
    state: &mut AppState,
    snapshot: &LinkSnapshot,
    p: &UiPalette,
    t: &crate::gui::i18n::GuiTexts,
    action: &mut SidebarAction,
) {
    ui.label(rich_section(t.sidebar_title(), p.text));

    let (symlink, junction) = snapshot.kind_counts();
    split_row(
        ui,
        |ui| {
            ui.label(rich_small(
                &t.sidebar_stats(snapshot.total(), symlink, junction),
                p.text_muted,
            ));
        },
        |ui| {
            if button(ui)
                .icon(Icon::Refresh)
                .tip(t.refresh())
                .enabled(!state.busy)
                .show()
                .clicked()
            {
                *action = SidebarAction::Refresh;
            }
            if state.refresh_notice_active() {
                ui.label(rich_small(t.refreshed(), p.accent));
            }
        },
    );
    ui.add_space(theme::gap(ui));

    if !state.checked_ids.is_empty() {
        let n = state.checked_ids.len();
        split_row(
            ui,
            |ui| {
                if button(ui)
                    .icon(Icon::Trash)
                    .label(&t.delete_selected(n))
                    .tip(t.delete_selected_tip())
                    .enabled(!state.busy)
                    .show()
                    .clicked()
                {
                    *action = SidebarAction::DeleteChecked;
                }
            },
            |ui| {
                if button(ui)
                    .label(t.clear_selection())
                    .tip(t.clear_selection())
                    .enabled(!state.busy)
                    .show()
                    .clicked()
                {
                    state.checked_ids.clear();
                }
            },
        );
        ui.add_space(theme::gap_sm(ui));
    }

    ui.add_enabled_ui(!state.busy, |ui| {
        search_field(ui, &mut state.search, t.search_label(), t.search_hint());
    });
    ui.add_space(theme::gap_lg(ui));
}

fn sidebar_list(
    ui: &mut Ui,
    state: &mut AppState,
    snapshot: &LinkSnapshot,
    p: &UiPalette,
    t: &crate::gui::i18n::GuiTexts,
    max_height: f32,
) {
    let item_count = snapshot.views.len();
    if item_count == 0 {
        ui.label(rich_small(
            if state.search.trim().is_empty() {
                t.no_links()
            } else {
                t.no_match()
            },
            p.text_muted,
        ));
        return;
    }
    let typo = theme::typography_from_ui(ui);
    let row_h = (typo.field_row_h + 4.0 * typo.scale).max(30.0);
    egui::ScrollArea::vertical()
        .id_salt("sidebar_list")
        .max_height(max_height)
        .auto_shrink([false, true])
        .show_rows(ui, row_h, item_count, |ui, range| {
            for row in range {
                let Some(view) = snapshot.view_at(row) else {
                    continue;
                };
                let name = snapshot.display_name_at(row).unwrap_or("");
                link_row(ui, state, view, name, p);
            }
        });
}

fn sidebar_pagination(
    ui: &mut Ui,
    state: &mut AppState,
    snapshot: &LinkSnapshot,
    p: &UiPalette,
    t: &crate::gui::i18n::GuiTexts,
    action: &mut SidebarAction,
) {
    let page_count = snapshot.page_count(state.page_size);
    state.page_index = state.page_index.min(page_count.saturating_sub(1));
    let current_page = state.page_index + 1;
    let mut page_size = state.page_size;
    split_row(
        ui,
        |ui| {
            if button(ui)
                .icon(Icon::Previous)
                .tip(t.previous_page())
                .enabled(!state.busy && state.page_index > 0)
                .show()
                .clicked()
            {
                state.page_index = state.page_index.saturating_sub(1);
                *action = SidebarAction::PageChanged;
            }
            ui.label(rich_small(
                &t.page_status(current_page, page_count),
                p.text_muted,
            ));
            if button(ui)
                .icon(Icon::Next)
                .tip(t.next_page())
                .enabled(!state.busy && current_page < page_count)
                .show()
                .clicked()
            {
                state.page_index += 1;
                *action = SidebarAction::PageChanged;
            }
        },
        |ui| {
            egui::ComboBox::from_id_salt("sidebar_page_size")
                .selected_text(format!("{} {}", t.page_size_label(), page_size))
                .show_ui(ui, |ui| {
                    for option in PAGE_SIZE_OPTIONS {
                        ui.selectable_value(
                            &mut page_size,
                            option,
                            format!("{} {option}", t.page_size_label()),
                        );
                    }
                });
        },
    );
    if page_size != state.page_size {
        state.page_size = page_size;
        state.page_index = 0;
        *action = SidebarAction::PageChanged;
    }
    if !state.search.trim().is_empty() {
        ui.label(rich_small(
            &t.sidebar_match_stats(snapshot.matched_total()),
            p.text_muted,
        ));
    }
}

fn link_row(ui: &mut Ui, state: &mut AppState, view: &LinkView, name: &str, p: &UiPalette) {
    let id = view.id;
    let selected = state.selected_id == Some(id);
    let t = state.texts();

    ui.horizontal(|ui| {
        let mut checked = state.checked_ids.contains(&id);
        let checkbox_resp = ui.add_enabled(!state.busy, egui::Checkbox::new(&mut checked, ""));
        checkbox_resp.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::Checkbox,
                !state.busy,
                checked,
                t.select_link_label(name),
            )
        });
        if checkbox_resp.changed() {
            if checked {
                state.checked_ids.insert(id);
            } else {
                state.checked_ids.remove(&id);
            }
        }
        let fg = if selected { p.accent } else { p.text };
        let typo = theme::typography_from_ui(ui);
        let delete_w = typo.icon_btn.x;
        let text_w = (ui.available_width() - delete_w - ui.spacing().item_spacing.x).max(1.0);
        let label = egui::Button::new(theme::rich_body(name, fg))
            .selected(selected)
            .frame(false)
            .truncate();
        let mut name_resp = ui
            .add_enabled_ui(!state.busy, |ui| {
                ui.add_sized(egui::vec2(text_w, typo.field_row_h), label)
            })
            .inner;
        if name.chars().count() > 18 {
            name_resp = name_resp.on_hover_text(name);
        }
        if !state.busy && name_resp.clicked() {
            state.selected_id = Some(id);
        }
        right_aligned(ui, |ui| {
            if button(ui)
                .icon(Icon::Trash)
                .tip(t.delete_link_tip())
                .enabled(!state.busy)
                .show()
                .clicked()
            {
                open_rm_dialog(state, view);
            }
        });
    });
}
