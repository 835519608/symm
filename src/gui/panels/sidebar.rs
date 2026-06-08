use crate::domain::model::LinkView;
use crate::gui::icons::Icon;
use crate::gui::panels::rm_dialog::open_rm_dialog;
use crate::gui::state::{AppState, LinkSnapshot};
use crate::gui::theme::{self, UiPalette, rich_section, rich_small};
use crate::gui::widgets::{button, right_aligned, search_field, split_row};
use egui::{Ui, WidgetInfo, WidgetType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarAction {
    Refresh,
    DeleteChecked,
    None,
}

pub fn show_sidebar(ui: &mut Ui, state: &mut AppState, snapshot: &LinkSnapshot) -> SidebarAction {
    let mut action = SidebarAction::None;
    let p = theme::resolve(state.theme, state.color_scheme);
    let t = state.texts();

    ui.vertical(|ui| {
        sidebar_header(ui, state, snapshot, &p, &t, &mut action);
        let list_h = ui.available_height();
        if list_h > 1.0 {
            sidebar_list(ui, state, snapshot, &p, &t);
        }
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
) {
    let item_count = state.sidebar_filter.refresh(snapshot, &state.search);
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
        .max_height(ui.available_height())
        .auto_shrink([false, true])
        .show_rows(ui, row_h, item_count, |ui, range| {
            for row in range {
                let Some(index) = state.sidebar_filter.index_at(row) else {
                    continue;
                };
                let Some(view) = snapshot.view_at(index) else {
                    continue;
                };
                let name = snapshot.display_name_at(index).unwrap_or("");
                link_row(ui, state, view, name, p);
            }
        });
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
