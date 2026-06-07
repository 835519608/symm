use crate::gui::fonts::icon_font_id;
use crate::gui::icons::Icon;
use crate::gui::theme::{self, rich_body_muted, typography_from_ui};
#[cfg(target_os = "macos")]
use crate::gui::util::pick_path_file_or_folder;
use crate::gui::util::{pick_path_file, pick_path_folder};
use egui::{Align, Layout, RichText, TextEdit, Ui, WidgetInfo, WidgetType};

const FORM_ROW_MIN_W: f32 = 120.0;
const FORM_EDIT_MIN_W: f32 = 80.0;
const FORM_BROWSE_W: f32 = 96.0;

fn browse_button_width(typo: &theme::UiTypography, _browse_label: &str) -> f32 {
    FORM_BROWSE_W * typo.scale
}

/// 表单行：输入区 + 右侧「浏览」槽位宽度。
#[derive(Clone, Copy)]
struct FormRowLayout {
    row_w: f32,
    browse_w: f32,
    gap: f32,
    field_h: f32,
}

fn form_row_layout(ui: &Ui, browse_label: &str) -> FormRowLayout {
    let typo = typography_from_ui(ui);
    let row_w = ui.available_rect_before_wrap().width().max(FORM_ROW_MIN_W);
    FormRowLayout {
        row_w,
        browse_w: browse_button_width(&typo, browse_label),
        gap: 6.0 * typo.scale,
        field_h: typo.field_row_h,
    }
}

fn form_row_horizontal<R>(ui: &mut Ui, layout: FormRowLayout, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(
        egui::vec2(layout.row_w, layout.field_h),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            add(ui)
        },
    )
    .inner
}

fn singleline_text_edit<'a>(value: &'a mut String, width: f32) -> TextEdit<'a> {
    TextEdit::singleline(value)
        .desired_width(width)
        .vertical_align(Align::Center)
}

fn add_text_edit(
    ui: &mut Ui,
    value: &mut String,
    size: egui::Vec2,
    hint: Option<&str>,
    label: &str,
) {
    let mut edit = singleline_text_edit(value, size.x);
    if let Some(h) = hint {
        edit = edit.hint_text(h);
    }
    let resp = ui.add_sized(size, edit);
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, ui.is_enabled(), label));
}

pub fn field_label(ui: &mut Ui, p: &theme::UiPalette, text: &str) {
    ui.label(rich_body_muted(text, p.text_muted));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathPickMode {
    /// 可选文件或文件夹（macOS 同一对话框；其它平台为浏览菜单）。
    FileOrFolder,
    /// 仅选文件夹（如数据目录）。
    FolderOnly,
}

/// 全宽行：输入框 + 右侧「浏览」。
fn path_input_row(
    ui: &mut Ui,
    value: &mut String,
    browse: PathBrowse<'_>,
    hint: Option<&str>,
    semantic_label: &str,
) -> Option<std::path::PathBuf> {
    let layout = form_row_layout(ui, browse.label);
    let typo = typography_from_ui(ui);
    let mut picked = None;

    form_row_horizontal(ui, layout, |ui| {
        let edit_w =
            (layout.row_w - layout.browse_w - layout.gap).max(FORM_EDIT_MIN_W * typo.scale);
        add_text_edit(
            ui,
            value,
            egui::vec2(edit_w, layout.field_h),
            hint,
            semantic_label,
        );
        ui.add_space(layout.gap);
        let btn_size = egui::vec2(layout.browse_w, layout.field_h);
        let menu = egui::menu::menu_custom_button(
            ui,
            egui::Button::new(browse.label).min_size(btn_size),
            |ui| match browse.pick {
                PathPickMode::FolderOnly => {
                    if ui.button(browse.pick_folder).clicked() {
                        picked = pick_path_folder();
                        ui.close_menu();
                    }
                }
                PathPickMode::FileOrFolder => {
                    #[cfg(target_os = "macos")]
                    {
                        if ui.button(browse.pick_unified).clicked() {
                            picked = pick_path_file_or_folder();
                            ui.close_menu();
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        if ui.button(browse.pick_file).clicked() {
                            picked = pick_path_file();
                            ui.close_menu();
                        }
                        if ui.button(browse.pick_folder).clicked() {
                            picked = pick_path_folder();
                            ui.close_menu();
                        }
                    }
                }
            },
        );
        if !browse.tip.is_empty() {
            menu.response.on_hover_text(browse.tip);
        }
    });

    picked
}

/// 路径控件行：输入框 + 右侧「浏览」，不绘制左侧标签。
pub fn path_control_row(
    ui: &mut Ui,
    value: &mut String,
    browse: PathBrowse<'_>,
    hint: Option<&str>,
    semantic_label: &str,
) -> Option<std::path::PathBuf> {
    path_input_row(ui, value, browse, hint, semantic_label)
}

/// 单行文本；`browse_gutter` 与路径行「浏览」同宽占位，右缘对齐。
fn text_input_row(
    ui: &mut Ui,
    value: &mut String,
    hint: Option<&str>,
    browse_gutter: &str,
    semantic_label: &str,
) {
    let layout = form_row_layout(ui, browse_gutter);
    let typo = typography_from_ui(ui);
    form_row_horizontal(ui, layout, |ui| {
        let edit_w =
            (layout.row_w - layout.browse_w - layout.gap).max(FORM_EDIT_MIN_W * typo.scale);
        add_text_edit(
            ui,
            value,
            egui::vec2(edit_w, layout.field_h),
            hint,
            semantic_label,
        );
        ui.add_space(layout.gap);
        ui.allocate_space(egui::vec2(layout.browse_w, layout.field_h));
    });
}

/// 单行文本：标签 + 输入框（可选与路径行右缘对齐）。
pub fn text_field(
    ui: &mut Ui,
    p: &theme::UiPalette,
    label: &str,
    value: &mut String,
    hint: Option<&str>,
    browse_gutter: Option<&str>,
) {
    field_label(ui, p, label);
    let gutter = browse_gutter.unwrap_or("");
    if browse_gutter.is_some() {
        text_input_row(ui, value, hint, gutter, label);
    } else {
        ui.horizontal(|ui| {
            let width = ui.available_rect_before_wrap().width().max(FORM_ROW_MIN_W);
            add_text_edit(
                ui,
                value,
                egui::vec2(width, typography_from_ui(ui).field_row_h),
                hint,
                label,
            );
        });
    }
}

/// 路径行「浏览」按钮与选择器文案。
#[derive(Clone, Copy)]
pub struct PathBrowse<'a> {
    pub label: &'a str,
    pub tip: &'a str,
    pub pick: PathPickMode,
    pub pick_file: &'a str,
    pub pick_folder: &'a str,
    /// macOS 统一选择器菜单项（其它平台可传空串）。
    #[allow(dead_code)]
    pub pick_unified: &'a str,
}

/// 路径行：标签 + 输入框 + 浏览按钮；选中路径时返回 `Some`。
pub fn path_field(
    ui: &mut Ui,
    p: &theme::UiPalette,
    label: &str,
    value: &mut String,
    browse: PathBrowse<'_>,
) -> Option<std::path::PathBuf> {
    field_label(ui, p, label);
    path_input_row(ui, value, browse, None, label)
}

/// 侧栏搜索框（egui [`TextEdit`] + 图标前缀）。
pub fn search_field(ui: &mut Ui, value: &mut String, label: &str, hint: &str) {
    let typo = typography_from_ui(ui);
    ui.horizontal(|ui| {
        ui.label(RichText::new(Icon::Search.glyph()).font(icon_font_id(typo.icon)));
        add_text_edit(
            ui,
            value,
            egui::vec2(ui.available_width(), typo.field_row_h),
            Some(hint),
            label,
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::gui_settings::{ColorScheme, FONT_SIZE_PT_DEFAULT, ThemeMode};

    #[test]
    fn path_and_text_rows_share_same_right_edge() {
        let ctx = egui::Context::default();
        let p = theme::resolve(ThemeMode::Light, ColorScheme::Slate);
        let mut path = String::new();
        let mut name = String::new();
        let mut path_rect = None;
        let mut text_rect = None;
        let browse = PathBrowse {
            label: "浏览",
            tip: "",
            pick: PathPickMode::FileOrFolder,
            pick_file: "文件",
            pick_folder: "文件夹",
            pick_unified: "选择",
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
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        });
        egui::CentralPanel::default().show(&ctx, |ui| {
            ui.set_max_width(560.0);

            field_label(ui, &p, "链接路径");
            path_input_row(ui, &mut path, browse, None, "链接路径");
            path_rect = Some(ui.min_rect());

            ui.add_space(8.0);
            field_label(ui, &p, "名称");
            text_input_row(ui, &mut name, Some("名称"), browse.label, "名称");
            text_rect = Some(ui.min_rect());
        });
        let _ = ctx.end_pass();

        let path = path_rect.expect("path row rect");
        let text = text_rect.expect("text row rect");
        assert!(
            (path.right() - text.right()).abs() <= 1.0,
            "path and text rows should align right edges; path={path:?}, text={text:?}"
        );
    }
}
