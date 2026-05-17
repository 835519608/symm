use crate::gui::state::{AppState, LinkSnapshot, MainView};
use crate::gui::theme;
use crate::gui::widgets::{card, card_header, stat_card, subtle_button};
use egui::{RichText, Ui};

pub fn show_dashboard(ui: &mut Ui, state: &mut AppState, snapshot: &LinkSnapshot) {
    ui.vertical(|ui| {
        if let Some(err) = &state.db_error {
            ui.colored_label(
                theme::status_color(crate::domain::model::LinkStatus::Missing),
                err,
            );
            ui.add_space(8.0);
        }

        ui.horizontal(|ui| {
            let w = (ui.available_width() - theme::SPACING * 2.0) / 3.0;
            ui.allocate_ui_with_layout(
                egui::vec2(w, 90.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| stat_card(ui, "📊", "链接总数", snapshot.total()),
            );
            ui.allocate_ui_with_layout(
                egui::vec2(w, 90.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| stat_card(ui, "✓", "正常", snapshot.ok_count()),
            );
            let (symlink, junction) = snapshot.kind_counts();
            ui.allocate_ui_with_layout(
                egui::vec2(w, 90.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    stat_card(
                        ui,
                        "✦",
                        "链接类型",
                        format!("软链 {symlink} / 联接 {junction}"),
                    );
                },
            );
        });
        ui.add_space(theme::SPACING);

        ui.horizontal(|ui| {
            let left_w = ui.available_width() * 0.62;
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| quick_start(ui, state, snapshot),
            );
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| common_ops(ui, state),
            );
        });
        ui.add_space(theme::SPACING);

        cli_hint_card(ui);
    });
}

fn quick_start(ui: &mut Ui, state: &mut AppState, snapshot: &LinkSnapshot) {
    card(ui, |ui| {
        card_header(ui, "快速访问");
        let items: Vec<_> = snapshot.views.iter().take(8).collect();
        if items.is_empty() {
            ui.label(
                RichText::new("尚无记录。运行 symm add <link> <target> 创建第一条。")
                    .color(theme::TEXT_SECONDARY)
                    .size(12.0),
            );
            return;
        }
        for view in items {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔗").size(20.0));
                ui.vertical(|ui| {
                    let name = view.display_name();
                    ui.label(RichText::new(&name).strong().size(14.0));
                    ui.label(
                        RichText::new(format!(
                            "{} · {}",
                            view.link_kind.label_zh(),
                            ellipsize_middle(&view.link_path, 42)
                        ))
                        .size(11.0)
                        .color(theme::TEXT_SECONDARY),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "⎘").on_hover_text("选中").clicked() {
                        state.selected_id = Some(view.id);
                        state.main_view = MainView::Detail;
                    }
                });
            });
            ui.add_space(4.0);
        }
    });
}

fn common_ops(ui: &mut Ui, state: &mut AppState) {
    card(ui, |ui| {
        card_header(ui, "常用操作");
        op_link(ui, state, "➕ 添加链接", "CLI: symm add <link> <target>");
        op_link(ui, state, "📋 查看列表", "CLI: symm ls");
        op_link(ui, state, "🔍 查看详情", "CLI: symm show <name|link>");
        op_link(ui, state, "📥 导入", "将已有软链纳管：symm add");
        ui.add_space(8.0);
        ui.label(
            RichText::new("完整能力（占用处理、冲突选项等）请使用终端 CLI。")
                .size(11.0)
                .color(theme::TEXT_SECONDARY),
        );
    });
}

fn op_link(ui: &mut Ui, state: &mut AppState, label: &str, tip: &str) {
    if subtle_button(ui, label).on_hover_text(tip).clicked() {
        state.toast = Some(tip.to_string());
    }
    ui.add_space(2.0);
}

fn cli_hint_card(ui: &mut Ui) {
    card(ui, |ui| {
        card_header(ui, "命令行集成");
        ui.label(
            RichText::new(
                "GUI 首版侧重浏览与导航；写操作与交互式决策仍通过 symm CLI 完成，行为与脚本一致。",
            )
            .size(12.0)
            .color(theme::TEXT_SECONDARY),
        );
        ui.add_space(6.0);
        let frame = egui::Frame::none()
            .fill(theme::CODE_BG)
            .rounding(egui::Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(10.0, 8.0));
        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.monospace("symm add ./link ./target");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if subtle_button(ui, "了解更多").clicked() {
                        // placeholder
                    }
                });
            });
        });
    });
}

fn ellipsize_middle(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(3) / 2;
    let start: String = s.chars().take(keep).collect();
    let end: String = s.chars().skip(char_count - keep).collect();
    format!("{start}...{end}")
}
