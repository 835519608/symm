use crate::domain::model::LinkView;
use crate::gui::icons::Icon;
use crate::gui::state::{AddConflictPolicy, AddForm, AddLockPolicy, AppState, MainView};
use crate::gui::theme;
use crate::gui::widgets::{
    PathBrowse, button, button_row, card, detail_field, empty_hint, form_page, page_heading,
    path_field, text_field, vertical_when_overflow,
};
use egui::{CollapsingHeader, Ui};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentAction {
    SubmitAdd,
    None,
}

pub fn show_content(ui: &mut Ui, state: &mut AppState, view: Option<&LinkView>) -> ContentAction {
    vertical_when_overflow(ui, "main_content", |ui| match state.main_view {
        MainView::Add => show_add(ui, state),
        MainView::Detail => {
            if let Some(v) = view {
                show_detail(ui, state, v);
            } else {
                show_detail_empty(ui, state);
            }
            ContentAction::None
        }
    })
}

fn show_add(ui: &mut Ui, state: &mut AppState) -> ContentAction {
    let mut action = ContentAction::None;
    let locale = state.locale;
    let t = crate::gui::i18n::GuiTexts::new(locale);
    let p = theme::resolve(state.theme, state.color_scheme);
    let form = &mut state.add_form;
    form_page(ui, |ui| show_add_form(ui, &p, &t, form, &mut action));
    action
}

fn show_add_form(
    ui: &mut Ui,
    p: &theme::UiPalette,
    t: &crate::gui::i18n::GuiTexts,
    form: &mut AddForm,
    action: &mut ContentAction,
) {
    page_heading(ui, p, t.add_heading(), Some(t.add_subtitle()));
    ui.add_space(12.0);

    let browse = PathBrowse {
        label: t.browse(),
        tip: t.browse_tip(),
    };
    if let Some(path) = path_field(ui, p, t.link_path_label(), &mut form.link_path, browse) {
        form.link_path = path.display().to_string();
    }
    ui.add_space(8.0);
    if let Some(path) = path_field(ui, p, t.target_path_label(), &mut form.target_path, browse) {
        form.target_path = path.display().to_string();
    }
    ui.add_space(8.0);
    text_field(
        ui,
        p,
        t.name_optional_label(),
        &mut form.name,
        Some(t.name_hint()),
    );

    ui.add_space(10.0);
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

    ui.add_space(14.0);
    button_row(ui, |ui| {
        if button(ui)
            .icon(Icon::Link)
            .label(t.create_link())
            .tip(t.create_link_tip())
            .show()
            .clicked()
        {
            *action = ContentAction::SubmitAdd;
        }
        if button(ui)
            .icon(Icon::Clear)
            .label(t.clear_form())
            .tip(t.clear_form_tip())
            .show()
            .clicked()
        {
            *form = AddForm::default();
        }
    });

    if let Some(err) = &form.error {
        ui.add_space(8.0);
        ui.colored_label(
            theme::status_color(crate::domain::model::LinkStatus::Missing),
            err,
        );
    }
    if let Some(msg) = &form.status_message {
        ui.add_space(8.0);
        ui.colored_label(
            theme::status_color(crate::domain::model::LinkStatus::Ok),
            msg,
        );
    }
}

fn show_detail_empty(ui: &mut Ui, state: &AppState) {
    let t = state.texts();
    let p = theme::resolve(state.theme, state.color_scheme);
    empty_hint(ui, &p, t.select_link_hint());
}

fn show_detail(ui: &mut Ui, state: &AppState, view: &LinkView) {
    let t = state.texts();
    let p = theme::resolve(state.theme, state.color_scheme);
    let w = ui.available_width();
    ui.set_width(w);
    ui.set_max_width(w);

    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(theme::rich_detail_title(&view.display_name(), p.text));
            ui.label(theme::rich_body(
                t.link_status(view.status),
                theme::status_color(view.status),
            ));
        });
    });
    ui.add_space(12.0);

    card(ui, |ui| {
        detail_field(ui, &p, t.field_name(), &view.name);
        detail_field(ui, &p, t.field_kind(), t.link_kind(view.link_kind));
        detail_field(ui, &p, t.field_status(), t.link_status(view.status));
        detail_field(ui, &p, t.field_link_path(), &view.link_path);
        detail_field(ui, &p, t.field_target_path(), &view.target_path);
        detail_field(ui, &p, t.field_index(), &view.index.to_string());
        detail_field(ui, &p, t.field_id(), &view.id.to_string());
    });
}

pub fn validate_add_form(
    form: &AddForm,
    locale: crate::domain::gui_settings::Locale,
) -> Result<(PathBuf, PathBuf), String> {
    let link = form.link_path.trim();
    let target = form.target_path.trim();
    if link.is_empty() || target.is_empty() {
        return Err(crate::gui::i18n::GuiTexts::new(locale)
            .paths_required()
            .to_string());
    }
    Ok((PathBuf::from(link), PathBuf::from(target)))
}
