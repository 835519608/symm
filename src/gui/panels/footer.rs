use crate::gui::state::{AppState, LinkSnapshot};
use crate::gui::theme;
use egui::{RichText, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterAction {
    OpenDataDir,
    None,
}

pub fn show_footer(ui: &mut Ui, state: &AppState, snapshot: &LinkSnapshot) -> FooterAction {
    let mut action = FooterAction::None;
    ui.horizontal(|ui| {
        let (symlink, junction) = snapshot.kind_counts();
        ui.label(
            RichText::new(format!(
                "共 {} · 正常 {} · 软链 {symlink} / 联接 {junction}",
                snapshot.total(),
                snapshot.ok_count(),
            ))
            .size(11.0)
            .color(theme::secondary_text(ui)),
        );
        if let Some(msg) = &state.toast {
            ui.label(RichText::new(msg).size(11.0).color(theme::ACCENT));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(11.0)
                    .color(theme::secondary_text(ui)),
            );
            if let Some(home) = &state.data_home {
                let path = home.display().to_string();
                let resp = ui.link(
                    RichText::new(format!("数据目录 {path}"))
                        .size(11.0)
                        .color(theme::ACCENT),
                );
                if resp.clicked() {
                    action = FooterAction::OpenDataDir;
                }
                resp.on_hover_text("在资源管理器中打开数据目录");
            }
        });
    });
    action
}
