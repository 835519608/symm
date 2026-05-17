use crate::gui::state::{AddConflictPolicy, AddForm, AddLockPolicy, AppState};
use crate::gui::theme;
use crate::gui::util::{pick_file, pick_folder};
use crate::gui::widgets::{card, card_header, primary_button, subtle_button};
use egui::{RichText, Ui};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddAction {
    Submit,
    None,
}

pub fn show_add(ui: &mut Ui, state: &mut AppState) -> AddAction {
    let mut action = AddAction::None;
    let form = &mut state.add_form;
    ui.vertical(|ui| {
        card(ui, |ui| {
            card_header(ui, "添加链接");
            ui.label(
                RichText::new("创建软链并写入数据库（与 CLI symm add 相同逻辑）")
                    .size(12.0)
                    .color(theme::secondary_text(ui)),
            );
            ui.add_space(10.0);

            path_field(ui, "链接路径 (link)", &mut form.link_path);
            ui.add_space(8.0);
            path_field(ui, "目标路径 (target)", &mut form.target_path);
            ui.add_space(8.0);
            ui.label(
                RichText::new("名称（可选）")
                    .size(12.0)
                    .color(theme::secondary_text(ui)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut form.name)
                    .hint_text("留空则使用链接文件名")
                    .desired_width(ui.available_width()),
            );

            ui.add_space(10.0);
            ui.collapsing("高级选项", |ui| {
                ui.label(
                    RichText::new("链接位置被占用时")
                        .size(12.0)
                        .color(theme::secondary_text(ui)),
                );
                ui.radio_value(
                    &mut form.lock_policy,
                    AddLockPolicy::Unlock,
                    "尝试结束占用进程并继续",
                );
                ui.radio_value(
                    &mut form.lock_policy,
                    AddLockPolicy::Cancel,
                    "取消（不结束进程）",
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("链接与目标路径都已存在时")
                        .size(12.0)
                        .color(theme::secondary_text(ui)),
                );
                ui.radio_value(
                    &mut form.conflict_policy,
                    AddConflictPolicy::KeepLink,
                    "保留链接侧（移走目标侧内容）",
                );
                ui.radio_value(
                    &mut form.conflict_policy,
                    AddConflictPolicy::KeepTarget,
                    "保留目标侧（移走链接侧内容）",
                );
            });

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if primary_button(ui, "创建链接").clicked() {
                    action = AddAction::Submit;
                }
                if subtle_button(ui, "清空表单").clicked() {
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
                ui.add_space(6.0);
                ui.colored_label(
                    theme::status_color(crate::domain::model::LinkStatus::Ok),
                    msg,
                );
            }
        });
    });
    action
}

fn path_field(ui: &mut Ui, label: &str, value: &mut String) {
    ui.label(
        RichText::new(label)
            .size(12.0)
            .color(theme::secondary_text(ui)),
    );
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(value).desired_width(ui.available_width() - 168.0));
        if subtle_button(ui, "文件…").clicked()
            && let Some(path) = pick_file()
        {
            *value = path.display().to_string();
        }
        if subtle_button(ui, "文件夹…").clicked()
            && let Some(path) = pick_folder()
        {
            *value = path.display().to_string();
        }
    });
}

pub fn validate_add_form(form: &AddForm) -> Result<(PathBuf, PathBuf), String> {
    let link = form.link_path.trim();
    let target = form.target_path.trim();
    if link.is_empty() || target.is_empty() {
        return Err("请填写链接路径与目标路径".to_string());
    }
    Ok((PathBuf::from(link), PathBuf::from(target)))
}
