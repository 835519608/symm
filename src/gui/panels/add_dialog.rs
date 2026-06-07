use crate::domain::gui_settings::Locale;
use crate::gui::icons::Icon;
use crate::gui::state::{AddConflictPolicy, AddForm, AddLockPolicy, AppState};
use crate::gui::theme;
use crate::gui::widgets::{
    ModalOptions, ModalSection, ModalSize, PathBrowse, PathPickMode, button, form_page, path_field,
    show_modal, split_row, text_field,
};
use egui::{CollapsingHeader, Ui};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddDialogAction {
    None,
    Submit,
    Close,
}

const ADD_MODAL: ModalSize = ModalSize::preferred(560.0, 480.0);

pub fn open_add_dialog(state: &mut AppState) {
    state.show_add_dialog = true;
    state.add_form.error = None;
    state.add_form.status_message = None;
}

pub fn show_add_dialog(ctx: &egui::Context, state: &mut AppState) -> AddDialogAction {
    if !state.show_add_dialog {
        return AddDialogAction::None;
    }

    let t = state.texts();
    let p = theme::resolve(state.theme, state.color_scheme);
    let enabled = !state.busy;
    let mut open = true;
    let mut action = AddDialogAction::None;
    let modal_id = egui::Id::new("add_link_dialog");

    let browse = PathBrowse {
        label: t.browse(),
        tip: t.browse_tip(),
        pick: PathPickMode::FileOrFolder,
        #[cfg(not(target_os = "macos"))]
        pick_file: t.browse_pick_file(),
        pick_folder: t.browse_pick_folder(),
        #[cfg(target_os = "macos")]
        pick_unified: t.browse_tip(),
    };

    let Some(modal) = show_modal(
        ctx,
        modal_id,
        &p,
        ModalOptions::new(t.add_heading(), ADD_MODAL).close_enabled(enabled),
        &mut open,
        t.settings_close(),
        |section| match section {
            ModalSection::Main(ui) => {
                ui.add_enabled_ui(enabled, |ui| {
                    form_page(ui, |ui| {
                        show_add_form(ui, &p, &t, &mut state.add_form, browse);
                    });
                });
            }
            ModalSection::FooterCustom(ui) => {
                split_row(
                    ui,
                    |ui| {
                        if button(ui)
                            .icon(Icon::Clear)
                            .label(t.clear_form())
                            .tip(t.clear_form_tip())
                            .enabled(enabled)
                            .show()
                            .clicked()
                        {
                            state.add_form = AddForm::default();
                        }
                    },
                    |ui| {
                        if button(ui)
                            .icon(Icon::Link)
                            .label(t.create_link())
                            .tip(t.create_link_tip())
                            .enabled(enabled)
                            .show()
                            .clicked()
                        {
                            action = AddDialogAction::Submit;
                        }
                    },
                );
            }
        },
    ) else {
        return AddDialogAction::None;
    };

    if modal.dismissed_by_backdrop {
        action = AddDialogAction::Close;
    }

    if !open {
        action = AddDialogAction::Close;
    }

    match action {
        AddDialogAction::Close => state.show_add_dialog = false,
        AddDialogAction::Submit | AddDialogAction::None => {}
    }

    action
}

pub fn validate_add_form(form: &AddForm, locale: Locale) -> Result<(PathBuf, PathBuf), String> {
    let link = form.link_path.trim();
    let target = form.target_path.trim();
    if link.is_empty() || target.is_empty() {
        return Err(crate::gui::i18n::GuiTexts::new(locale)
            .paths_required()
            .to_string());
    }
    Ok((PathBuf::from(link), PathBuf::from(target)))
}

fn show_add_form(
    ui: &mut Ui,
    p: &theme::UiPalette,
    t: &crate::gui::i18n::GuiTexts,
    form: &mut AddForm,
    browse: PathBrowse<'_>,
) {
    if let Some(path) = path_field(ui, p, t.link_path_label(), &mut form.link_path, browse) {
        form.link_path = path.display().to_string();
    }
    ui.add_space(theme::gap(ui));
    if let Some(path) = path_field(ui, p, t.target_path_label(), &mut form.target_path, browse) {
        form.target_path = path.display().to_string();
    }
    ui.add_space(theme::gap(ui));
    text_field(
        ui,
        p,
        t.name_optional_label(),
        &mut form.name,
        Some(t.name_hint()),
        Some(browse.label),
    );

    ui.add_space(theme::gap(ui));
    CollapsingHeader::new(t.advanced_options())
        .id_salt("add_advanced")
        .show(ui, |ui| {
            ui.radio_value(
                &mut form.lock_policy,
                AddLockPolicy::Unlock,
                t.lock_unlock(),
            );
            ui.radio_value(
                &mut form.lock_policy,
                AddLockPolicy::Cancel,
                t.lock_cancel(),
            );
            ui.radio_value(
                &mut form.conflict_policy,
                AddConflictPolicy::KeepLink,
                t.conflict_keep_link(),
            );
            ui.radio_value(
                &mut form.conflict_policy,
                AddConflictPolicy::KeepTarget,
                t.conflict_keep_target(),
            );
        });

    if let Some(err) = &form.error {
        ui.add_space(theme::gap(ui));
        ui.add(
            egui::Label::new(egui::RichText::new(err.as_str()).color(
                theme::status_color_for_palette(crate::domain::model::LinkStatus::Missing, p),
            ))
            .wrap(),
        );
    }
    if let Some(msg) = &form.status_message {
        ui.add_space(theme::gap(ui));
        ui.add(
            egui::Label::new(egui::RichText::new(msg.as_str()).color(
                theme::status_color_for_palette(crate::domain::model::LinkStatus::Ok, p),
            ))
            .wrap(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::gui_settings::{ColorScheme, FONT_SIZE_PT_DEFAULT, ThemeMode};
    use crate::gui::i18n::GuiTexts;

    #[test]
    fn add_dialog_form_spacing_matches_modal_body() {
        let ctx = egui::Context::default();
        let p = theme::resolve(ThemeMode::Light, ColorScheme::Slate);
        let t = GuiTexts::new(Locale::ZhCn);
        let mut open = true;
        let mut form = AddForm::default();
        let mut main_rect = None;
        let mut form_rect = None;
        let browse = PathBrowse {
            label: t.browse(),
            tip: t.browse_tip(),
            pick: PathPickMode::FileOrFolder,
            #[cfg(not(target_os = "macos"))]
            pick_file: t.browse_pick_file(),
            pick_folder: t.browse_pick_folder(),
            #[cfg(target_os = "macos")]
            pick_unified: t.browse_tip(),
        };

        theme::apply(
            &ctx,
            ThemeMode::Light,
            ColorScheme::Slate,
            FONT_SIZE_PT_DEFAULT,
        );
        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            ..Default::default()
        });
        show_modal(
            &ctx,
            egui::Id::new("add_dialog_spacing_test"),
            &p,
            ModalOptions::new(t.add_heading(), ADD_MODAL),
            &mut open,
            t.settings_close(),
            |section| match section {
                ModalSection::Main(ui) => {
                    main_rect = Some(ui.max_rect());
                    form_page(ui, |ui| {
                        show_add_form(ui, &p, &t, &mut form, browse);
                        form_rect = Some(ui.min_rect());
                    });
                }
                ModalSection::FooterCustom(ui) => {
                    ui.label("footer");
                }
            },
        )
        .expect("add dialog should render");
        let _ = ctx.end_pass();
        let area = ctx
            .memory(|mem| mem.area_rect(egui::Id::new("add_dialog_spacing_test")))
            .expect("modal area should be stored");
        let main = main_rect.expect("main rect");
        let form = form_rect.expect("form rect");

        assert!(
            (form.left() - main.left()).abs() <= 1.0,
            "add form should use modal body left edge; area={area:?}, main={main:?}, form={form:?}"
        );
        assert!(
            area.right() - form.right() <= 36.0,
            "add form should not leave a large right gap; area={area:?}, main={main:?}, form={form:?}"
        );
    }
}
