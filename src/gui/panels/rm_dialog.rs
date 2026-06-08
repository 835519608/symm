use crate::domain::model::LinkView;
use crate::gui::i18n::GuiTexts;
use crate::gui::state::{AppState, RmDialog};
use crate::gui::theme::{self, rich_section};
use crate::gui::widgets::{
    ModalOptions, ModalSection, ModalSize, button, right_aligned, show_modal,
};
use crate::workflows::rm::workflow::RemoveMode;

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

    let t = state.texts();
    let p = theme::resolve(state.theme, state.color_scheme);
    let enabled = !state.busy;
    let mut open = true;
    let mut action = RmDialogAction::None;
    let mut mode = dialog.mode;
    let modal_id = egui::Id::new("rm_dialog");

    let Some(modal) = show_modal(
        ctx,
        modal_id,
        &p,
        ModalOptions::new(t.rm_dialog_title(), ModalSize::fit_content(400.0))
            .close_enabled(enabled),
        &mut open,
        t.cancel(),
        |section| match section {
            ModalSection::Main(ui) => {
                ui.add_enabled_ui(enabled, |ui| {
                    ui.add(
                        egui::Label::new(rich_section(
                            &t.rm_confirm_prompt(&dialog.summary),
                            p.text,
                        ))
                        .wrap(),
                    );
                    ui.add_space(theme::gap_lg(ui));
                    ui.radio_value(
                        &mut mode,
                        RemoveMode::DeleteLinkOnly,
                        t.rm_mode_delete_only(),
                    );
                    ui.radio_value(
                        &mut mode,
                        RemoveMode::RestoreTargetToLink,
                        t.rm_mode_restore(),
                    );
                });
            }
            ModalSection::FooterCustom(ui) => {
                right_aligned(ui, |ui| {
                    if button(ui)
                        .icon(crate::gui::icons::Icon::Trash)
                        .label(t.confirm_delete())
                        .enabled(enabled)
                        .show()
                        .clicked()
                    {
                        action = RmDialogAction::Confirm;
                    }
                });
            }
        },
    ) else {
        return RmDialogAction::None;
    };

    if modal.dismissed_by_backdrop {
        action = RmDialogAction::Cancel;
    }

    if let Some(d) = state.rm_dialog.as_mut() {
        d.mode = mode;
    }

    if !open {
        action = RmDialogAction::Cancel;
    }

    match action {
        RmDialogAction::Cancel => state.rm_dialog = None,
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
    state.rm_dialog = Some(RmDialog {
        ids: vec![view.id],
        summary: view.display_name(),
        mode: RemoveMode::DeleteLinkOnly,
    });
}

pub fn open_rm_dialog_batch(state: &mut AppState, views: &[&LinkView]) {
    if views.is_empty() {
        return;
    }
    let t = GuiTexts::new(state.locale);
    let ids: Vec<i64> = views.iter().map(|v| v.id).collect();
    let summary = if views.len() == 1 {
        views[0].display_name()
    } else {
        t.rm_batch_summary(&views[0].display_name(), views.len())
    };
    state.rm_dialog = Some(RmDialog {
        ids,
        summary,
        mode: RemoveMode::DeleteLinkOnly,
    });
}
