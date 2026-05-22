use crate::gui::fonts::icon_font_id;
use crate::gui::icons::Icon;
use crate::gui::theme::{self, rich_body, rich_body_muted, typography_from_ui};
use crate::gui::util::pick_path;
use egui::{Align, Button, Layout, RichText, TextEdit, Ui};

fn row_width(ui: &Ui) -> f32 {
    ui.available_width().max(120.0)
}

fn browse_button_width(typo: &theme::UiTypography, browse_label: &str) -> f32 {
    (browse_label.len() as f32 * 7.5 * typo.scale + 34.0 * typo.scale).max(64.0 * typo.scale)
}

pub fn field_label(ui: &mut Ui, p: &theme::UiPalette, text: &str) {
    ui.label(rich_body_muted(text, p.text_muted));
}

/// 表单字段：主色标签 + 控件（设置页等）。
pub fn labeled_field<R>(
    ui: &mut Ui,
    p: &theme::UiPalette,
    label: &str,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.label(rich_body(label, p.text));
    ui.add_space(6.0);
    add(ui)
}

/// 全宽行：输入框 + 右侧「浏览」贴齐行末。
fn path_input_row(
    ui: &mut Ui,
    value: &mut String,
    browse_label: &str,
    browse_tip: &str,
    hint: Option<&str>,
) -> Option<std::path::PathBuf> {
    let typo = typography_from_ui(ui);
    let row_w = row_width(ui);
    let gap = 6.0 * typo.scale;
    let browse_w = browse_button_width(&typo, browse_label);
    let mut picked = None;

    ui.horizontal(|ui| {
        ui.set_width(row_w);
        ui.set_min_width(row_w);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let mut resp =
                ui.add(Button::new(browse_label).min_size(egui::vec2(browse_w, typo.field_row_h)));
            if !browse_tip.is_empty() {
                resp = resp.on_hover_text(browse_tip);
            }
            if resp.clicked()
                && let Some(path) = pick_path()
            {
                picked = Some(path);
            }
            ui.add_space(gap);
            let edit_w = ui.available_width().max(80.0 * typo.scale);
            let mut edit = TextEdit::singleline(value).desired_width(edit_w);
            if let Some(h) = hint {
                edit = edit.hint_text(h);
            }
            ui.add(edit);
        });
    });

    picked
}

/// 全宽单行输入（与路径行同宽）。
fn text_input_row(ui: &mut Ui, value: &mut String, hint: Option<&str>) {
    let row_w = row_width(ui);
    ui.horizontal(|ui| {
        ui.set_width(row_w);
        ui.set_min_width(row_w);
        let mut edit = TextEdit::singleline(value).desired_width(ui.available_width());
        if let Some(h) = hint {
            edit = edit.hint_text(h);
        }
        ui.add(edit);
    });
}

/// 单行文本：标签 + 全宽输入框。
pub fn text_field(
    ui: &mut Ui,
    p: &theme::UiPalette,
    label: &str,
    value: &mut String,
    hint: Option<&str>,
) {
    field_label(ui, p, label);
    text_input_row(ui, value, hint);
}

/// 路径行：标签 + 输入框 + 浏览按钮；选中路径时返回 `Some`。
pub fn path_field(
    ui: &mut Ui,
    p: &theme::UiPalette,
    label: &str,
    value: &mut String,
    browse_label: &str,
    browse_tip: &str,
) -> Option<std::path::PathBuf> {
    path_field_inner(ui, p, label, value, browse_label, browse_tip, None, None)
}

/// 路径行 + 占位提示 + 底部说明（设置页数据目录等）。
pub fn path_field_with_hints(
    ui: &mut Ui,
    p: &theme::UiPalette,
    label: &str,
    value: &mut String,
    browse_label: &str,
    browse_tip: &str,
    hint: &str,
    note: &str,
) -> Option<std::path::PathBuf> {
    path_field_inner(
        ui,
        p,
        label,
        value,
        browse_label,
        browse_tip,
        Some(hint),
        Some(note),
    )
}

fn path_field_inner(
    ui: &mut Ui,
    p: &theme::UiPalette,
    label: &str,
    value: &mut String,
    browse_label: &str,
    browse_tip: &str,
    hint: Option<&str>,
    note: Option<&str>,
) -> Option<std::path::PathBuf> {
    field_label(ui, p, label);
    let picked = path_input_row(ui, value, browse_label, browse_tip, hint);
    if let Some(note) = note {
        ui.label(rich_body_muted(note, p.text_muted));
    }
    picked
}

/// 侧栏搜索框（egui [`TextEdit`] + 图标前缀）。
pub fn search_field(ui: &mut Ui, value: &mut String, hint: &str) {
    let typo = typography_from_ui(ui);
    let row_w = row_width(ui);
    ui.horizontal(|ui| {
        ui.set_width(row_w);
        ui.set_min_width(row_w);
        ui.label(RichText::new(Icon::Search.glyph()).font(icon_font_id(typo.icon)));
        ui.add(
            TextEdit::singleline(value)
                .hint_text(hint)
                .desired_width(ui.available_width()),
        );
    });
}
