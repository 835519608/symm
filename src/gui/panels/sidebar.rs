use crate::domain::model::LinkView;
use crate::gui::icons::Icon;
use crate::gui::panels::rm_dialog::open_rm_dialog;
use crate::gui::state::{AppState, LinkSnapshot, MainView};
use crate::gui::theme::{self, UiPalette, rich_section, rich_small};
use crate::gui::widgets::{button, search_field, vertical_when_overflow};
use egui::Ui;

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
            vertical_when_overflow(ui, "sidebar_list", |ui| {
                sidebar_list(ui, state, snapshot, &p, &t);
            });
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
    ui.horizontal(|ui| {
        ui.label(rich_section(t.sidebar_title(), p.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if button(ui)
                .icon(Icon::Refresh)
                .tip(t.refresh())
                .show()
                .clicked()
            {
                *action = SidebarAction::Refresh;
            }
        });
    });

    let (symlink, junction) = snapshot.kind_counts();
    ui.label(rich_small(
        &t.sidebar_stats(snapshot.total(), symlink, junction),
        p.text_muted,
    ));
    ui.add_space(8.0 * theme::typography_from_ui(ui).scale);

    if !state.checked_ids.is_empty() {
        ui.horizontal(|ui| {
            let n = state.checked_ids.len();
            if button(ui)
                .icon(Icon::Trash)
                .label(&t.delete_selected(n))
                .tip(t.delete_selected_tip())
                .show()
                .clicked()
            {
                *action = SidebarAction::DeleteChecked;
            }
            ui.add_space(6.0 * theme::typography_from_ui(ui).scale);
            if button(ui)
                .label(t.clear_selection())
                .tip(t.clear_selection())
                .show()
                .clicked()
            {
                state.checked_ids.clear();
            }
        });
        ui.add_space(6.0 * theme::typography_from_ui(ui).scale);
    }

    search_field(ui, &mut state.search, t.search_hint());
    ui.add_space(10.0 * theme::typography_from_ui(ui).scale);
}

fn sidebar_list(
    ui: &mut Ui,
    state: &mut AppState,
    snapshot: &LinkSnapshot,
    p: &UiPalette,
    t: &crate::gui::i18n::GuiTexts,
) {
    let items = snapshot.filtered_by_name(&state.search);
    if items.is_empty() {
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
    for view in items {
        link_row(ui, state, view, p);
        ui.add_space(4.0 * theme::typography_from_ui(ui).scale);
    }
}

fn link_row(ui: &mut Ui, state: &mut AppState, view: &LinkView, p: &UiPalette) {
    let id = view.id;
    let selected = state.selected_id == Some(id);
    let t = state.texts();

    ui.horizontal(|ui| {
        let mut checked = state.checked_ids.contains(&id);
        if ui.checkbox(&mut checked, "").changed() {
            if checked {
                state.checked_ids.insert(id);
            } else {
                state.checked_ids.remove(&id);
            }
        }
        let name = view.display_name();
        let fg = if selected { p.accent } else { p.text };
        if ui
            .selectable_label(selected, theme::rich_body(&name, fg))
            .clicked()
        {
            state.selected_id = Some(id);
            state.main_view = MainView::Detail;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if button(ui)
                .icon(Icon::Trash)
                .tip(t.delete_link_tip())
                .show()
                .clicked()
            {
                open_rm_dialog(state, view);
            }
        });
    });

    if selected {
        ui.add_space(4.0 * theme::typography_from_ui(ui).scale);
        ui.indent(format!("paths_{id}"), |ui| {
            ui.label(rich_small(
                &format!("link: {}", view.link_path),
                p.text_muted,
            ));
            ui.label(rich_small(
                &format!("target: {}", view.target_path),
                p.text_muted,
            ));
        });
    }
}
