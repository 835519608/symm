use crate::domain::model::LinkView;
use crate::gui::icons::Icon;
use crate::gui::panels::rm_dialog::open_rm_dialog;
use crate::gui::state::{AppState, LinkSnapshot, PAGE_SIZE_OPTIONS};
use crate::gui::theme::{self, UiPalette, rich_section, rich_small};
use crate::gui::widgets::{
    SelectableTextStyle, button, checkbox_icon, right_aligned, search_field, selectable_text_row,
    split_row,
};
use egui::Ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarAction {
    Refresh,
    DeleteChecked,
    PageChanged,
    Selected(i64),
    None,
}

pub fn show_sidebar(ui: &mut Ui, state: &mut AppState, snapshot: &LinkSnapshot) -> SidebarAction {
    let mut action = SidebarAction::None;
    let p = theme::resolve(state.theme, state.color_scheme);
    let t = state.texts();

    ui.vertical(|ui| {
        sidebar_header(ui, state, snapshot, &p, &t, &mut action);
        let typo = theme::typography_from_ui(ui);
        let match_stats_h = if state.search.trim().is_empty() {
            0.0
        } else {
            typo.small + theme::gap(ui)
        };
        let pagination_h = typo.btn_h + match_stats_h + 2.0 * theme::gap(ui);
        let list_h = (ui.available_height() - pagination_h).max(0.0);
        if list_h > 1.0 {
            sidebar_list(ui, state, snapshot, &p, &t, list_h, &mut action);
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
                ui.label(rich_small(t.refreshed(), p.accent_text));
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
                    .danger()
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

    search_field(ui, &mut state.search, t.search_label(), t.search_hint());
    ui.add_space(theme::gap_lg(ui));
}

fn sidebar_list(
    ui: &mut Ui,
    state: &mut AppState,
    snapshot: &LinkSnapshot,
    p: &UiPalette,
    t: &crate::gui::i18n::GuiTexts,
    max_height: f32,
    action: &mut SidebarAction,
) {
    let item_count = snapshot.views.len();
    if item_count == 0 {
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), max_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.label(rich_small(
                    if state.search.trim().is_empty() {
                        t.no_links()
                    } else {
                        t.no_match()
                    },
                    p.text_muted,
                ));
            },
        );
        return;
    }
    let typo = theme::typography_from_ui(ui);
    let row_h = (typo.field_row_h + 4.0 * typo.scale).max(30.0);
    let row_spacing = ui.spacing().item_spacing.y;
    let content_h = (row_h + row_spacing) * item_count as f32 - row_spacing;
    let scroll_gutter = if content_h > max_height {
        floating_scrollbar_gutter(ui)
    } else {
        0.0
    };
    let row_w = (ui.available_width() - scroll_gutter).max(1.0);
    egui::ScrollArea::vertical()
        .id_salt("sidebar_list")
        .max_height(max_height)
        .auto_shrink([false, false])
        .show_rows(ui, row_h, item_count, |ui, range| {
            ui.set_min_width(row_w);
            for row in range {
                ui.allocate_ui_with_layout(
                    egui::vec2(row_w, row_h),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let Some(view) = snapshot.view_at(row) else {
                            return;
                        };
                        let name = snapshot.display_name_at(row).unwrap_or_default();
                        link_row(ui, state, view, name.as_ref(), p, action);
                    },
                );
            }
        });
}

fn floating_scrollbar_gutter(ui: &Ui) -> f32 {
    let scroll = ui.spacing().scroll;
    (scroll.bar_width + scroll.bar_inner_margin + scroll.bar_outer_margin + 2.0).ceil()
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

fn selectable_name_row(
    ui: &mut Ui,
    selected: bool,
    enabled: bool,
    name: &str,
    p: &UiPalette,
    width: f32,
    height: f32,
) -> egui::Response {
    let typo = theme::typography_from_ui(ui);
    selectable_text_row(
        ui,
        p,
        selected,
        enabled,
        name,
        egui::vec2(width, height),
        SelectableTextStyle::new(p.text, egui::FontId::proportional(typo.body)),
    )
}

fn link_row(
    ui: &mut Ui,
    state: &mut AppState,
    view: &LinkView,
    name: &str,
    p: &UiPalette,
    action: &mut SidebarAction,
) {
    let id = view.id;
    let selected = state.selected_id == Some(id);
    let t = state.texts();

    ui.horizontal_centered(|ui| {
        let mut checked = state.checked_ids.contains(&id);
        let select_label = t.select_link_label(name);
        let checkbox_resp = checkbox_icon(ui, &mut checked, !state.busy, &select_label);
        if checkbox_resp.changed() {
            if checked {
                state.checked_ids.insert(id);
            } else {
                state.checked_ids.remove(&id);
            }
        }
        let typo = theme::typography_from_ui(ui);
        let delete_w = typo.icon_btn.x;
        let text_w = (ui.available_width() - delete_w - ui.spacing().item_spacing.x).max(1.0);
        let mut name_resp =
            selectable_name_row(ui, selected, !state.busy, name, p, text_w, typo.field_row_h);
        if name.chars().count() > 18 {
            name_resp = name_resp.on_hover_text(name);
        }
        if !state.busy && name_resp.clicked() {
            state.selected_id = Some(id);
            *action = SidebarAction::Selected(id);
        }
        right_aligned(ui, |ui| {
            if button(ui)
                .icon(Icon::Trash)
                .tip(t.delete_link_tip())
                .danger()
                .enabled(!state.busy)
                .show()
                .clicked()
            {
                open_rm_dialog(state, view);
            }
        });
    });
}
