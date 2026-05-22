use crate::domain::gui_settings::{ColorScheme, FONT_SIZE_PT_MAX, FONT_SIZE_PT_MIN};
use crate::gui::state::{AppState, SettingsDraft, SettingsSection};
use crate::gui::theme::SIDEBAR_WIDTH_MIN;
use crate::gui::theme::{
    self, SIDEBAR_PANEL_ID, rich_body, rich_body_muted, rich_section, rich_small,
};
use crate::gui::widgets::{
    ModalOptions, ModalSize, PathBrowse, PathFieldHints, button, button_row, labeled_field,
    path_field_with_hints, settings_nav, show_modal,
};
use egui::Ui;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsDialogAction {
    None,
    Apply,
    Close,
}

const SETTINGS_MODAL: ModalSize = ModalSize::fit_content(560.0);
const SETTINGS_NAV_W: f32 = 108.0;

pub fn open_settings(state: &mut AppState) {
    state.settings_draft = Some(SettingsDraft::from_state(state));
}

pub fn show_settings_dialog(ctx: &egui::Context, state: &mut AppState) -> SettingsDialogAction {
    let Some(mut draft) = state.settings_draft.clone() else {
        return SettingsDialogAction::None;
    };

    let t = state.texts();
    let p = theme::resolve(state.theme, state.color_scheme);
    let mut open = true;
    let mut action = SettingsDialogAction::None;
    let modal_id = egui::Id::new("settings_dialog");

    let Some(modal) = show_modal(
        ctx,
        modal_id,
        &p,
        ModalOptions::new(t.settings_title(), SETTINGS_MODAL),
        &mut open,
        |ui| {
            ui.horizontal(|ui| {
                settings_nav(
                    ui,
                    &mut draft.section,
                    &[
                        (SettingsSection::Appearance, t.settings_nav_appearance()),
                        (SettingsSection::About, t.settings_nav_about()),
                    ],
                    SETTINGS_NAV_W,
                );

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(12.0);

                ui.vertical(|ui| {
                    let content_w = (SETTINGS_MODAL.width - SETTINGS_NAV_W - 48.0).max(280.0);
                    ui.set_width(content_w);
                    ui.set_min_width(content_w);
                    match draft.section {
                        SettingsSection::Appearance => {
                            appearance_page(ui, &p, &t, &mut draft);
                        }
                        SettingsSection::About => {
                            about_page(ui, &p, &t);
                        }
                    }
                });
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            button_row(ui, |ui| {
                if button(ui)
                    .label(t.settings_restore_defaults())
                    .tip(t.settings_restore_defaults_tip())
                    .show()
                    .clicked()
                {
                    let d = SettingsDraft::appearance_defaults();
                    draft.color_scheme = d.color_scheme;
                    draft.font_size_pt = d.font_size_pt;
                    draft.sidebar_width = d.sidebar_width;
                    draft.data_dir.clear();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui).label(t.settings_apply()).show().clicked() {
                        action = SettingsDialogAction::Apply;
                    }
                    if button(ui).label(t.settings_close()).show().clicked() {
                        action = SettingsDialogAction::Close;
                    }
                });
            });
        },
    ) else {
        return SettingsDialogAction::None;
    };

    if modal.dismissed_by_backdrop {
        action = SettingsDialogAction::Close;
    }

    state.settings_draft = Some(draft);

    if !open {
        action = SettingsDialogAction::Close;
    }

    match action {
        SettingsDialogAction::Apply => {
            if let Some(d) = state.settings_draft.take() {
                if let Err(msg) = apply_data_dir(&d.data_dir) {
                    state.settings_draft = Some(d);
                    state.toast = Some(msg);
                } else {
                    let sidebar_max = theme::sidebar_max_width(ctx);
                    state.color_scheme = d.color_scheme;
                    state.font_size_pt =
                        crate::domain::gui_settings::sanitize_font_size_pt(d.font_size_pt);
                    state.sidebar_width = d.sidebar_width.clamp(SIDEBAR_WIDTH_MIN, sidebar_max);
                    state.data_dir = d.data_dir.trim().to_string();
                    theme::pin_side_panel_width(ctx, SIDEBAR_PANEL_ID, state.sidebar_width);
                }
            }
        }
        SettingsDialogAction::Close => state.settings_draft = None,
        SettingsDialogAction::None => {}
    }

    action
}

fn appearance_page(
    ui: &mut Ui,
    p: &theme::UiPalette,
    t: &crate::gui::i18n::GuiTexts,
    draft: &mut SettingsDraft,
) {
    labeled_field(ui, p, t.settings_color_scheme(), |ui| {
        egui::ComboBox::from_id_salt("settings_color_scheme")
            .selected_text(t.color_scheme_label(draft.color_scheme))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for scheme in ColorScheme::ALL {
                    ui.selectable_value(
                        &mut draft.color_scheme,
                        scheme,
                        t.color_scheme_label(scheme),
                    );
                }
            });
    });

    ui.add_space(14.0);

    labeled_field(ui, p, t.settings_font_size(), |ui| {
        ui.horizontal(|ui| {
            let mut size = draft.font_size_pt;
            ui.add(
                egui::Slider::new(&mut size, FONT_SIZE_PT_MIN..=FONT_SIZE_PT_MAX)
                    .show_value(false)
                    .smart_aim(false),
            );
            draft.font_size_pt = size;
            ui.label(rich_body(
                &format!("{:.0}px", draft.font_size_pt.round()),
                p.text,
            ));
        });
        ui.label(rich_small(
            &t.settings_font_size_hint(FONT_SIZE_PT_MIN, FONT_SIZE_PT_MAX),
            p.text_muted,
        ));
    });

    ui.add_space(14.0);

    labeled_field(ui, p, t.settings_sidebar_width(), |ui| {
        ui.horizontal(|ui| {
            let mut w = draft.sidebar_width;
            ui.add(
                egui::Slider::new(&mut w, SIDEBAR_WIDTH_MIN..=480.0)
                    .show_value(false)
                    .smart_aim(false),
            );
            draft.sidebar_width = w;
            ui.label(rich_body(
                &format!("{:.0}px", draft.sidebar_width.round()),
                p.text,
            ));
        });
    });

    ui.add_space(14.0);

    if let Some(path) = path_field_with_hints(
        ui,
        p,
        t.settings_data_dir(),
        &mut draft.data_dir,
        PathBrowse {
            label: t.browse(),
            tip: t.settings_data_dir_browse_tip(),
        },
        PathFieldHints {
            hint: t.settings_data_dir_hint(),
            note: t.settings_data_dir_note(),
        },
    ) {
        draft.data_dir = path.display().to_string();
    }
}

fn about_page(ui: &mut Ui, p: &theme::UiPalette, t: &crate::gui::i18n::GuiTexts) {
    ui.label(rich_section(t.settings_about_heading(), p.text));
    ui.add_space(8.0);
    ui.label(rich_body(t.settings_about_tagline(), p.text_muted));
    ui.add_space(12.0);
    ui.label(rich_body_muted(
        &format!(
            "{} {}",
            t.settings_version_label(),
            env!("CARGO_PKG_VERSION")
        ),
        p.text_muted,
    ));
}

/// 应用设置中的数据目录；成功返回 `Ok(())`。
pub fn apply_data_dir(data_dir: &str) -> Result<PathBuf, String> {
    let trimmed = data_dir.trim();
    if trimmed.is_empty() {
        crate::gui::env::sync_symm_home("");
        return crate::adapters::paths::runtime_paths::data_home().map_err(|e| e.to_string());
    }
    let path = PathBuf::from(trimmed);
    crate::gui::env::ensure_data_dir(&path).map_err(|e| e.to_string())?;
    crate::gui::env::sync_symm_home(trimmed);
    Ok(path)
}
