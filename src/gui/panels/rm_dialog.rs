use crate::gui::state::{AppState, RmDialog};
use crate::gui::theme;
use crate::gui::widgets::{primary_button, subtle_button};
use crate::workflows::rm::workflow::RemoveMode;
use egui::RichText;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RmDialogAction {
    Confirm,
    Cancel,
    None,
}

pub fn show_rm_dialog(ctx: &egui::Context, state: &mut AppState) -> RmDialogAction {
    let Some(dialog) = state.rm_dialog.clone() else {
        return RmDialogAction::None;
    };

    let mut open = true;
    let mut action = RmDialogAction::None;
    let mut mode = dialog.mode;

    egui::Window::new("删除链接")
        .collapsible(false)
        .resizable(false)
        .default_width(400.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(
                RichText::new(format!("确定删除「{}」？", dialog.display_name))
                    .strong()
                    .color(theme::primary_text(ui)),
            );
            ui.add_space(8.0);
            ui.radio_value(
                &mut mode,
                RemoveMode::DeleteLinkOnly,
                "只删除软链与数据库记录",
            );
            ui.radio_value(
                &mut mode,
                RemoveMode::RestoreTargetToLink,
                "删除软链，并把目标移回链接位置",
            );
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if primary_button(ui, "确认删除").clicked() {
                    action = RmDialogAction::Confirm;
                }
                if subtle_button(ui, "取消").clicked() {
                    action = RmDialogAction::Cancel;
                }
            });
        });

    if let Some(d) = state.rm_dialog.as_mut() {
        d.mode = mode;
    }

    if !open {
        action = RmDialogAction::Cancel;
    }

    match action {
        RmDialogAction::Cancel => {
            state.rm_dialog = None;
        }
        RmDialogAction::Confirm => {
            if let Some(d) = state.rm_dialog.as_mut() {
                d.mode = mode;
            }
        }
        RmDialogAction::None => {}
    }

    action
}

pub fn open_rm_dialog(state: &mut AppState, selector: String, display_name: String) {
    state.rm_dialog = Some(RmDialog {
        selector,
        display_name,
        mode: RemoveMode::DeleteLinkOnly,
    });
}
