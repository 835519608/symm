use crate::domain::model::LinkView;
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
                RichText::new(format!("确定删除「{}」？", dialog.summary))
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

pub fn open_rm_dialog(state: &mut AppState, view: &LinkView) {
    let selector = selector_for(view);
    state.rm_dialog = Some(RmDialog {
        selectors: vec![selector],
        summary: view.display_name(),
        mode: RemoveMode::DeleteLinkOnly,
    });
}

pub fn open_rm_dialog_batch(state: &mut AppState, views: &[&LinkView]) {
    if views.is_empty() {
        return;
    }
    let selectors: Vec<String> = views.iter().map(|v| selector_for(v)).collect();
    let summary = if views.len() == 1 {
        views[0].display_name()
    } else {
        format!("{} 等 {} 条链接", views[0].display_name(), views.len())
    };
    state.rm_dialog = Some(RmDialog {
        selectors,
        summary,
        mode: RemoveMode::DeleteLinkOnly,
    });
}

fn selector_for(view: &LinkView) -> String {
    if view.name.is_empty() {
        view.id.to_string()
    } else {
        view.name.clone()
    }
}
