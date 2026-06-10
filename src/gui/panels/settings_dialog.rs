use crate::domain::gui_settings::{
    ColorScheme, FONT_SIZE_PT_MAX, FONT_SIZE_PT_MIN, Locale, ThemeMode,
};
use crate::gui::state::{AppState, SettingsDraft, SettingsSection};
use crate::gui::theme::{self, rich_body, rich_body_muted, rich_section};
use crate::gui::widgets::{
    ModalOptions, ModalSection, ModalSize, PathBrowse, PathPickMode, button, fill_ui_width,
    modal_scroll_vertical, path_control_row, selectable_row_rect, settings_content_frame,
    settings_nav, show_modal, split_row, value_slider,
};
use egui::{Grid, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsDialogAction {
    None,
    Apply,
    Close,
}

const SETTINGS_MODAL: ModalSize = ModalSize::preferred(600.0, 460.0);
const SETTINGS_NAV_W: f32 = 148.0;
/// 与 [`settings_nav`] 项内按钮文字内边距一致，使画框内文与侧栏文字左右对齐。
const SETTINGS_CONTENT_PAD: f32 = 12.0;
const SETTINGS_FIELD_GAP: f32 = 12.0;
const SETTINGS_FIELD_VALUE_W: f32 = 56.0;

pub fn open_settings(state: &mut AppState) {
    state.settings_draft = Some(SettingsDraft::from_state(state));
}

pub fn show_settings_dialog(ctx: &egui::Context, state: &mut AppState) -> SettingsDialogAction {
    let t = state.texts();
    let p = theme::resolve(state.theme, state.color_scheme);
    let enabled = !state.busy;
    let data_dir_runtime_override = state.data_dir_runtime_override;
    let mut open = true;
    let mut action = SettingsDialogAction::None;
    let modal_id = egui::Id::new("settings_dialog");

    let Some(draft) = state.settings_draft.as_mut() else {
        return SettingsDialogAction::None;
    };

    let Some(modal) = show_modal(
        ctx,
        modal_id,
        &p,
        ModalOptions::new(t.settings_title(), SETTINGS_MODAL),
        &mut open,
        t.settings_close(),
        |section| match section {
            ModalSection::Main(ui) => {
                ui.add_enabled_ui(enabled, |ui| {
                    settings_main_body(ui, &p, &t, draft, data_dir_runtime_override);
                });
            }
            ModalSection::FooterCustom(ui) => {
                split_row(
                    ui,
                    |ui| {
                        if button(ui)
                            .label(t.settings_restore_defaults())
                            .tip(if data_dir_runtime_override {
                                t.settings_restore_defaults_tip_data_dir_overridden()
                            } else {
                                t.settings_restore_defaults_tip()
                            })
                            .enabled(enabled)
                            .show()
                            .clicked()
                        {
                            draft.restore_defaults(data_dir_runtime_override);
                        }
                    },
                    |ui| {
                        if button(ui)
                            .label(t.settings_apply())
                            .enabled(enabled)
                            .show()
                            .clicked()
                        {
                            action = SettingsDialogAction::Apply;
                        }
                    },
                );
            }
        },
    ) else {
        return SettingsDialogAction::None;
    };

    if modal.dismissed_by_backdrop {
        action = SettingsDialogAction::Close;
    }

    if !open {
        action = SettingsDialogAction::Close;
    }

    match action {
        SettingsDialogAction::Apply => {}
        SettingsDialogAction::Close => state.settings_draft = None,
        SettingsDialogAction::None => {}
    }

    action
}

/// 设置主区：左导航 | 分隔 | 右内容（铺满剩余宽，与 Frame 左右内边距对称）。
fn settings_main_body(
    ui: &mut Ui,
    p: &theme::UiPalette,
    t: &crate::gui::i18n::GuiTexts,
    draft: &mut SettingsDraft,
    data_dir_runtime_override: bool,
) {
    fill_ui_width(ui);
    let pane_h = ui.available_height().max(120.0);

    ui.horizontal(|ui| {
        ui.set_min_height(pane_h);
        ui.set_max_height(pane_h);

        ui.allocate_ui_with_layout(
            egui::vec2(SETTINGS_NAV_W, pane_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                settings_nav(
                    ui,
                    p,
                    &mut draft.section,
                    &[
                        (SettingsSection::Appearance, t.settings_nav_appearance()),
                        (SettingsSection::About, t.settings_nav_about()),
                    ],
                    SETTINGS_NAV_W,
                );
            },
        );
        ui.separator();

        settings_content_pane(ui, |ui| {
            settings_content_frame(ui, SETTINGS_CONTENT_PAD, |ui| {
                modal_scroll_vertical(ui, "settings_dialog_body", |ui| {
                    ui.vertical(|ui| match draft.section {
                        SettingsSection::Appearance => {
                            appearance_page(ui, p, t, draft, data_dir_runtime_override)
                        }
                        SettingsSection::About => about_page(ui, p, t),
                    });
                });
            })
        });
    });
}

fn color_scheme_combo(
    ui: &mut Ui,
    t: &crate::gui::i18n::GuiTexts,
    value: &mut ColorScheme,
    width: f32,
    dark: bool,
) {
    let selected_text = rich_body(
        t.color_scheme_label(*value),
        theme::accent_text_for_scheme(*value, dark),
    );
    egui::ComboBox::from_id_salt("settings_color_scheme")
        .selected_text(selected_text)
        .width(width)
        .show_ui(ui, |ui| {
            ui.set_min_width(width);
            for scheme in ColorScheme::ALL {
                if color_scheme_option(ui, t.color_scheme_label(scheme), scheme, *value, dark)
                    .clicked()
                {
                    *value = scheme;
                    ui.close_menu();
                }
            }
        });
}

fn color_scheme_option(
    ui: &mut Ui,
    label: &str,
    scheme: ColorScheme,
    current: ColorScheme,
    dark: bool,
) -> egui::Response {
    let typo = theme::typography_from_ui(ui);
    let width = ui.available_width().max(120.0);
    let height = typo.field_row_h;
    let selected = scheme == current;
    let accent = theme::accent_for_scheme(scheme, dark);
    let accent_text = theme::accent_text_for_scheme(scheme, dark);
    let (rect, resp) = selectable_row_rect(ui, selected, true, label, width, height);

    if ui.is_rect_visible(rect) {
        let pad = ui.spacing().button_padding.x;
        let swatch_size = (height * 0.42).clamp(10.0, 16.0);
        let swatch_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + pad, rect.center().y - swatch_size * 0.5),
            egui::vec2(swatch_size, swatch_size),
        );
        ui.painter().rect(
            swatch_rect,
            egui::Rounding::same(4.0),
            accent,
            egui::Stroke::new(1.0, accent.gamma_multiply(0.72)),
        );
        ui.painter().text(
            egui::pos2(swatch_rect.right() + pad * 0.75, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            typo.button_font(),
            accent_text,
        );
    }

    resp
}

fn settings_slider_width(control_w: f32) -> f32 {
    (control_w - SETTINGS_FIELD_VALUE_W - SETTINGS_FIELD_GAP).max(80.0)
}

fn settings_slider_value(
    ui: &mut Ui,
    p: &theme::UiPalette,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    control_w: f32,
) {
    let value_text = format!("{:.0}px", value.round());
    value_slider(
        ui,
        p,
        value,
        range,
        settings_slider_width(control_w),
        &value_text,
    );
}

fn settings_content_pane<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.vertical(|ui| {
        fill_ui_width(ui);
        ui.set_min_height(ui.available_height());
        add(ui)
    })
    .inner
}

fn settings_grid_row<R>(
    ui: &mut Ui,
    p: &theme::UiPalette,
    label: &str,
    tip: Option<&str>,
    add_control: impl FnOnce(&mut Ui, f32) -> R,
) -> R {
    let resp = ui.label(rich_body(label, p.text));
    if let Some(tip) = tip.filter(|tip| !tip.is_empty()) {
        resp.on_hover_text(tip);
    }
    let control_w = ui.available_width().max(120.0);
    let inner = add_control(ui, control_w);
    ui.end_row();
    inner
}

fn appearance_page(
    ui: &mut Ui,
    p: &theme::UiPalette,
    t: &crate::gui::i18n::GuiTexts,
    draft: &mut SettingsDraft,
    data_dir_runtime_override: bool,
) {
    let font_size_hint = t.settings_font_size_hint(FONT_SIZE_PT_MIN, FONT_SIZE_PT_MAX);
    let data_dir_note = if data_dir_runtime_override {
        t.settings_data_dir_env_override_note()
    } else {
        t.settings_data_dir_note()
    };
    Grid::new("settings_appearance_grid")
        .num_columns(2)
        .spacing(egui::vec2(SETTINGS_FIELD_GAP, 12.0))
        .striped(false)
        .show(ui, |ui| {
            settings_grid_row(ui, p, t.settings_theme(), None, |ui, control_w| {
                egui::ComboBox::from_id_salt("settings_theme")
                    .selected_text(t.theme_mode_label(draft.theme))
                    .width(control_w)
                    .show_ui(ui, |ui| {
                        for mode in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
                            ui.selectable_value(&mut draft.theme, mode, t.theme_mode_label(mode));
                        }
                    });
            });

            settings_grid_row(ui, p, t.settings_locale(), None, |ui, control_w| {
                egui::ComboBox::from_id_salt("settings_locale")
                    .selected_text(draft.locale.toggle_label())
                    .width(control_w)
                    .show_ui(ui, |ui| {
                        for locale in [Locale::ZhCn, Locale::En] {
                            ui.selectable_value(&mut draft.locale, locale, locale.toggle_label());
                        }
                    });
            });

            settings_grid_row(ui, p, t.settings_color_scheme(), None, |ui, control_w| {
                color_scheme_combo(ui, t, &mut draft.color_scheme, control_w, p.dark);
            });

            settings_grid_row(
                ui,
                p,
                t.settings_font_size(),
                Some(&font_size_hint),
                |ui, control_w| {
                    let mut size = draft.font_size_pt;
                    settings_slider_value(
                        ui,
                        p,
                        &mut size,
                        FONT_SIZE_PT_MIN..=FONT_SIZE_PT_MAX,
                        control_w,
                    );
                    draft.font_size_pt = size;
                },
            );

            settings_grid_row(
                ui,
                p,
                t.settings_data_dir(),
                Some(data_dir_note),
                |ui, _control_w| {
                    let mut picked = None;
                    ui.add_enabled_ui(!data_dir_runtime_override, |ui| {
                        picked = path_control_row(
                            ui,
                            &mut draft.data_dir,
                            PathBrowse {
                                label: t.browse(),
                                tip: t.settings_data_dir_browse_tip(),
                                pick: PathPickMode::FolderOnly,
                                #[cfg(not(target_os = "macos"))]
                                pick_file: t.browse_pick_file(),
                                #[cfg(not(target_os = "macos"))]
                                pick_file_title: t.browse_pick_file_title(),
                                pick_folder: t.browse_pick_folder(),
                                pick_folder_title: t.browse_pick_folder_title(),
                                #[cfg(target_os = "macos")]
                                pick_unified: t.browse_pick_folder(),
                                #[cfg(target_os = "macos")]
                                pick_unified_title: t.browse_pick_folder_title(),
                                #[cfg(target_os = "macos")]
                                pick_unified_prompt: t.browse_pick_unified_prompt(),
                            },
                            Some(t.settings_data_dir_hint()),
                            t.settings_data_dir(),
                        );
                    });
                    if let Some(path) = picked {
                        draft.data_dir = path.display().to_string();
                    }
                },
            );
        });
}

fn about_page(ui: &mut Ui, p: &theme::UiPalette, t: &crate::gui::i18n::GuiTexts) {
    ui.vertical_centered(|ui| {
        ui.label(rich_section(t.settings_about_heading(), p.text));
        ui.add_space(theme::gap(ui));
        ui.label(rich_body(t.settings_about_tagline(), p.text_muted));
        ui.add_space(theme::gap_lg(ui));
        ui.label(rich_body_muted(
            &format!(
                "{} {}",
                t.settings_version_label(),
                env!("CARGO_PKG_VERSION")
            ),
            p.text_muted,
        ));
    });
}
