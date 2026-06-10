use crate::gui::theme::{self, UiPalette, rich_body};
use egui::{Align2, Color32, FontId, Rect, Response, Sense, Stroke, Ui, WidgetInfo, WidgetType};

pub struct SelectableTextStyle {
    pub idle_text: Color32,
    pub font_id: FontId,
}

impl SelectableTextStyle {
    pub fn new(idle_text: Color32, font_id: FontId) -> Self {
        Self { idle_text, font_id }
    }
}

pub fn checkbox_icon(ui: &mut Ui, checked: &mut bool, enabled: bool, label: &str) -> Response {
    let selected = *checked;
    let semantic_enabled = enabled && ui.is_enabled();
    let resp = if selected && semantic_enabled {
        let p = theme::palette_from_ui(ui);
        ui.scope(|ui| {
            apply_checked_checkbox_visuals(ui, p);
            ui.add_enabled(enabled, egui::Checkbox::without_text(checked))
        })
        .inner
    } else {
        ui.add_enabled(enabled, egui::Checkbox::without_text(checked))
    };
    resp.widget_info(|| {
        WidgetInfo::selected(WidgetType::Checkbox, semantic_enabled, selected, label)
    });
    resp
}

fn apply_checked_checkbox_visuals(ui: &mut Ui, p: UiPalette) {
    let widgets = &mut ui.style_mut().visuals.widgets;

    widgets.inactive.bg_fill = p.accent_soft;
    widgets.inactive.weak_bg_fill = p.accent_soft;
    widgets.inactive.bg_stroke = Stroke::new(1.0, p.accent_text);
    widgets.inactive.fg_stroke = Stroke::new(1.4, p.accent_text);

    widgets.hovered.bg_fill = p.accent_active;
    widgets.hovered.weak_bg_fill = p.accent_active;
    widgets.hovered.bg_stroke = Stroke::new(1.2, p.accent_text);
    widgets.hovered.fg_stroke = Stroke::new(1.4, p.accent_text);

    widgets.active.bg_fill = p.accent_active;
    widgets.active.weak_bg_fill = p.accent_active;
    widgets.active.bg_stroke = Stroke::new(1.2, p.accent_text);
    widgets.active.fg_stroke = Stroke::new(1.4, p.accent_text);
}

pub fn radio_value<T: PartialEq + Copy>(
    ui: &mut Ui,
    p: &UiPalette,
    current: &mut T,
    value: T,
    label: &str,
) -> Response {
    let selected = *current == value;
    let color = if selected { p.accent_text } else { p.text };
    let resp = ui.add(egui::RadioButton::new(selected, rich_body(label, color)));
    if resp.clicked() && !selected {
        *current = value;
    }
    resp
}

pub fn selectable_row_rect(
    ui: &mut Ui,
    selected: bool,
    enabled: bool,
    label: &str,
    width: f32,
    height: f32,
) -> (Rect, Response) {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), sense);
    resp.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::SelectableLabel,
            enabled && ui.is_enabled(),
            selected,
            label,
        )
    });

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&resp, selected);
        if selected || resp.hovered() || resp.highlighted() || resp.has_focus() {
            ui.painter().rect(
                rect.expand(visuals.expansion),
                visuals.rounding,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
            );
        }
    }

    (rect, resp)
}

pub fn selectable_text_row(
    ui: &mut Ui,
    p: &UiPalette,
    selected: bool,
    enabled: bool,
    label: &str,
    size: egui::Vec2,
    text: SelectableTextStyle,
) -> Response {
    let (rect, resp) = selectable_row_rect(ui, selected, enabled, label, size.x, size.y);

    if ui.is_rect_visible(rect) {
        let pad = ui.spacing().button_padding.x;
        let text_color = if enabled {
            if selected || resp.hovered() || resp.has_focus() {
                p.accent_text
            } else {
                text.idle_text
            }
        } else {
            p.text_muted
        };
        ui.painter()
            .with_clip_rect(rect.shrink2(egui::vec2(pad, 0.0)))
            .text(
                egui::pos2(rect.left() + pad, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                text.font_id,
                text_color,
            );
    }

    resp
}
