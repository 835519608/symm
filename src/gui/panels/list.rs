use crate::domain::model::LinkView;
use crate::gui::state::{AppState, LinkSnapshot, MainView};
use crate::gui::theme;
use crate::gui::widgets::{card, card_header};
use egui::{RichText, Ui};

pub fn show_list(ui: &mut Ui, state: &mut AppState, snapshot: &LinkSnapshot) {
    ui.vertical(|ui| {
        card(ui, |ui| {
            card_header(ui, "全部链接");
            let items = snapshot.filtered(&state.search);
            if items.is_empty() {
                ui.label(
                    RichText::new("暂无记录")
                        .size(12.0)
                        .color(theme::secondary_text(ui)),
                );
                return;
            }

            egui::Grid::new("link_list_grid")
                .num_columns(4)
                .spacing([12.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    header_cell(ui, "名称");
                    header_cell(ui, "状态");
                    header_cell(ui, "链接路径");
                    header_cell(ui, "目标路径");
                    ui.end_row();

                    for view in items {
                        if row(ui, state, view) {
                            state.main_view = MainView::Detail;
                        }
                        ui.end_row();
                    }
                });
        });
    });
}

fn header_cell(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .strong()
            .size(12.0)
            .color(theme::secondary_text(ui)),
    );
}

fn row(ui: &mut Ui, state: &mut AppState, view: &LinkView) -> bool {
    let name = view.display_name();
    let mut clicked = false;
    if ui
        .link(RichText::new(&name).size(13.0).color(theme::ACCENT))
        .clicked()
    {
        state.selected_id = Some(view.id);
        state.expanded_ids.insert(view.id);
        clicked = true;
    }
    ui.label(
        RichText::new(view.status.label_zh())
            .size(12.0)
            .color(theme::status_color(view.status)),
    );
    ui.label(RichText::new(&view.link_path).size(12.0));
    ui.label(RichText::new(&view.target_path).size(12.0));
    clicked
}
