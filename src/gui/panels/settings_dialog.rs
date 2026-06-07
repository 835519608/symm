use crate::domain::gui_settings::{ColorScheme, FONT_SIZE_PT_MAX, FONT_SIZE_PT_MIN};
use crate::gui::state::{AppState, SettingsDraft, SettingsSection};
use crate::gui::theme::SIDEBAR_WIDTH_MIN;
use crate::gui::theme::{self, rich_body, rich_body_muted, rich_section};
use crate::gui::widgets::{
    ModalOptions, ModalSection, ModalSize, PathBrowse, PathPickMode, button, fill_ui_width,
    form_page, modal_scroll_vertical, path_control_row, settings_content_frame, settings_nav,
    show_modal, split_row,
};
use egui::{Grid, Ui};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsDialogAction {
    None,
    Apply,
    Close,
}

const SETTINGS_MODAL: ModalSize = ModalSize::preferred(600.0, 460.0);
const SETTINGS_NAV_W: f32 = 108.0;
/// 与 [`settings_nav`] 项内 `shrink2(12, 0)` 一致，使画框内文与侧栏文字左右对齐。
const SETTINGS_CONTENT_PAD: f32 = 12.0;
const SETTINGS_FIELD_GAP: f32 = 12.0;
const SETTINGS_FIELD_VALUE_W: f32 = 56.0;

pub fn open_settings(state: &mut AppState) {
    state.settings_draft = Some(SettingsDraft::from_state(state));
}

pub fn show_settings_dialog(ctx: &egui::Context, state: &mut AppState) -> SettingsDialogAction {
    let Some(mut draft) = state.settings_draft.clone() else {
        return SettingsDialogAction::None;
    };

    let t = state.texts();
    let p = theme::resolve(state.theme, state.color_scheme);
    let enabled = !state.busy;
    let sidebar_max = theme::sidebar_max_width(ctx);
    draft.sidebar_width = draft.sidebar_width.clamp(SIDEBAR_WIDTH_MIN, sidebar_max);
    let mut open = true;
    let mut action = SettingsDialogAction::None;
    let modal_id = egui::Id::new("settings_dialog");

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
                    settings_main_body(ui, &p, &t, &mut draft, sidebar_max);
                });
            }
            ModalSection::FooterCustom(ui) => {
                split_row(
                    ui,
                    |ui| {
                        if button(ui)
                            .label(t.settings_restore_defaults())
                            .tip(t.settings_restore_defaults_tip())
                            .enabled(enabled)
                            .show()
                            .clicked()
                        {
                            let d = SettingsDraft::appearance_defaults();
                            draft.color_scheme = d.color_scheme;
                            draft.font_size_pt = d.font_size_pt;
                            draft.sidebar_width = d.sidebar_width;
                            draft.data_dir.clear();
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

    state.settings_draft = Some(draft);

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
    sidebar_max: f32,
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
                    form_page(ui, |ui| match draft.section {
                        SettingsSection::Appearance => {
                            appearance_page(ui, p, t, draft, sidebar_max)
                        }
                        SettingsSection::About => about_page(ui, p, t),
                    });
                });
            })
        });
    });
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
    ui.horizontal(|ui| {
        ui.add_sized(
            egui::vec2(
                settings_slider_width(control_w),
                ui.spacing().interact_size.y,
            ),
            egui::Slider::new(value, range)
                .show_value(false)
                .smart_aim(false),
        );
        ui.label(rich_body(&format!("{:.0}px", value.round()), p.text));
    });
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
    sidebar_max: f32,
) {
    let font_size_hint = t.settings_font_size_hint(FONT_SIZE_PT_MIN, FONT_SIZE_PT_MAX);
    Grid::new("settings_appearance_grid")
        .num_columns(2)
        .spacing(egui::vec2(SETTINGS_FIELD_GAP, 12.0))
        .striped(false)
        .show(ui, |ui| {
            settings_grid_row(ui, p, t.settings_color_scheme(), None, |ui, control_w| {
                egui::ComboBox::from_id_salt("settings_color_scheme")
                    .selected_text(t.color_scheme_label(draft.color_scheme))
                    .width(control_w)
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

            settings_grid_row(ui, p, t.settings_sidebar_width(), None, |ui, control_w| {
                let mut w = draft.sidebar_width;
                settings_slider_value(ui, p, &mut w, SIDEBAR_WIDTH_MIN..=sidebar_max, control_w);
                draft.sidebar_width = w;
            });

            settings_grid_row(
                ui,
                p,
                t.settings_data_dir(),
                Some(t.settings_data_dir_note()),
                |ui, _control_w| {
                    if let Some(path) = path_control_row(
                        ui,
                        &mut draft.data_dir,
                        PathBrowse {
                            label: t.browse(),
                            tip: t.settings_data_dir_browse_tip(),
                            pick: PathPickMode::FolderOnly,
                            pick_file: t.browse_pick_file(),
                            pick_folder: t.browse_pick_folder(),
                            pick_unified: t.browse_pick_folder(),
                        },
                        Some(t.settings_data_dir_hint()),
                        t.settings_data_dir(),
                    ) {
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
