//! 模态弹窗：标题 / 主区 / 底栏。
//!
//! 按 egui [`Modal`] 的推荐用法：外层交给 `egui::Modal + Frame`，内部只维护一个
//! vertical content flow。尺寸只在 content root 统一约束，避免 header、body、footer
//! 各自计算宽度后互相撑开。

use crate::gui::theme::{self, UiPalette, rich_section};
use crate::gui::widgets::button::{button, default_min_size};
use egui::{
    self, Area, Color32, Context, Frame, Id, Layout, Margin, Order, Rangef, ScrollArea, Ui, Vec2,
};

#[derive(Clone, Copy, Debug)]
pub struct ModalSize {
    /// 首选内容宽度；实际宽度会按 viewport 收缩。
    pub width: f32,
    /// `None`：高度随内容；`Some(h)`：首选内容高度。
    pub height: Option<f32>,
}

impl ModalSize {
    pub const fn fit_content(width: f32) -> Self {
        Self {
            width,
            height: None,
        }
    }

    pub const fn preferred(width: f32, height: f32) -> Self {
        Self {
            width,
            height: Some(height),
        }
    }

    fn resolve(self, ctx: &Context) -> Self {
        let viewport = ctx.screen_rect().size();
        Self {
            width: self.width.min((viewport.x * 0.92).max(1.0)),
            height: self.height.map(|h| h.min((viewport.y * 0.86).max(1.0))),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ModalBackdrop {
    pub enabled: bool,
    pub click_to_close: bool,
    pub alpha: Option<u8>,
}

impl ModalBackdrop {
    pub const DEFAULT: Self = Self {
        enabled: true,
        click_to_close: true,
        alpha: None,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct ModalOptions<'a> {
    pub title: &'a str,
    pub size: ModalSize,
    pub backdrop: ModalBackdrop,
    pub close_enabled: bool,
}

impl<'a> ModalOptions<'a> {
    pub fn new(title: &'a str, size: ModalSize) -> Self {
        Self {
            title,
            size,
            backdrop: ModalBackdrop::DEFAULT,
            close_enabled: true,
        }
    }

    pub fn close_enabled(mut self, enabled: bool) -> Self {
        self.close_enabled = enabled;
        self
    }
}

pub struct ModalResponse {
    pub dismissed_by_backdrop: bool,
    pub close_clicked: bool,
}

pub enum ModalSection<'a> {
    Main(&'a mut Ui),
    FooterCustom(&'a mut Ui),
}

const MODAL_FRAME_MARGIN: f32 = 16.0;
const MODAL_SECTION_GAP: f32 = 8.0;
const MODAL_FOOTER_GAP: f32 = 8.0;

fn backdrop_color(p: &UiPalette, backdrop: ModalBackdrop) -> Color32 {
    let a = backdrop.alpha.unwrap_or(if p.dark { 140 } else { 90 });
    Color32::from_rgba_premultiplied(15, 23, 42, a)
}

fn window_frame(ctx: &Context, p: &UiPalette) -> Frame {
    Frame::window(&ctx.style())
        .fill(p.surface)
        .stroke(egui::Stroke::new(1.0, p.border))
        .rounding(theme::rounding())
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 10.0),
            blur: 24.0,
            spread: 0.0,
            color: p.shadow,
        })
        .inner_margin(Margin::same(MODAL_FRAME_MARGIN))
}

fn modal_area(id: Id) -> Area {
    Area::new(id)
        .kind(egui::UiKind::Modal)
        .sense(egui::Sense::hover())
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .order(Order::Foreground)
        .interactable(true)
}

fn close_button_size(ui: &Ui, label: &str) -> Vec2 {
    let typo = theme::typography_from_ui(ui);
    default_min_size(&typo, None, label)
}

fn title_height(ui: &Ui) -> f32 {
    let typo = theme::typography_from_ui(ui);
    typo.section
        .max(ui.spacing().interact_size.y * 0.75)
        .max(22.0)
}

fn footer_row_height(ui: &Ui) -> f32 {
    let typo = theme::typography_from_ui(ui);
    typo.btn_h.max(ui.spacing().interact_size.y).max(28.0)
}

fn separator_height() -> f32 {
    MODAL_SECTION_GAP * 2.0 + 1.0
}

fn footer_height(ui: &Ui) -> f32 {
    footer_row_height(ui)
}

fn section_separator(ui: &mut Ui, width: f32, p: &UiPalette) {
    ui.add_space(MODAL_SECTION_GAP);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
    let px = ui.ctx().pixels_per_point().recip();
    ui.painter().hline(
        Rangef::new(rect.left() - px, rect.right() + px),
        rect.center().y,
        egui::Stroke::new(1.0, p.border),
    );
    ui.add_space(MODAL_SECTION_GAP);
}

pub fn fill_ui_width(ui: &mut Ui) {
    let width = ui.available_width().min(ui.max_rect().width());
    ui.set_width(width);
    ui.set_min_width(width);
    ui.set_max_width(width);
}

fn constrain_width(ui: &mut Ui, width: f32) {
    ui.set_width(width);
    ui.set_min_width(width);
    ui.set_max_width(width);
}

fn modal_footer(
    ui: &mut Ui,
    width: f32,
    close_label: &str,
    close_enabled: bool,
    render: &mut impl FnMut(ModalSection<'_>),
) -> bool {
    constrain_width(ui, width);

    let row_h = footer_row_height(ui);
    let close_size = close_button_size(ui, close_label);
    let gap = MODAL_FOOTER_GAP * theme::typography_from_ui(ui).scale;
    let custom_w = (width - close_size.x - gap).max(1.0);
    let mut close_clicked = false;

    ui.allocate_ui_with_layout(
        egui::vec2(width, row_h),
        Layout::left_to_right(egui::Align::Center),
        |ui| {
            constrain_width(ui, width);
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.allocate_ui_with_layout(
                egui::vec2(custom_w, row_h),
                Layout::left_to_right(egui::Align::Center),
                |ui| {
                    constrain_width(ui, custom_w);
                    render(ModalSection::FooterCustom(ui));
                },
            );
            ui.add_space(gap);
            if button(ui)
                .label(close_label)
                .enabled(close_enabled)
                .show_sized(close_size)
                .clicked()
            {
                close_clicked = true;
            }
        },
    );

    close_clicked
}

fn modal_content(
    ui: &mut Ui,
    p: &UiPalette,
    options: ModalOptions<'_>,
    close_label: &str,
    render: &mut impl FnMut(ModalSection<'_>),
) -> bool {
    let width = options.size.width.max(1.0);
    let reserved_h = title_height(ui) + separator_height() * 2.0 + footer_height(ui);
    let main_h = options.size.height.map(|h| (h - reserved_h).max(1.0));
    let mut close_clicked = false;

    ui.vertical(|ui| {
        constrain_width(ui, width);
        ui.label(rich_section(options.title, p.text));
        section_separator(ui, width, p);

        if let Some(main_h) = main_h {
            ui.allocate_ui_with_layout(
                egui::vec2(width, main_h),
                Layout::top_down(egui::Align::LEFT),
                |ui| {
                    constrain_width(ui, width);
                    render(ModalSection::Main(ui));
                },
            );
        } else {
            render(ModalSection::Main(ui));
        }

        section_separator(ui, width, p);
        close_clicked = modal_footer(ui, width, close_label, options.close_enabled, render);
    });

    close_clicked
}

pub fn modal_scroll_vertical<R>(
    ui: &mut Ui,
    id_salt: &'static str,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let viewport = ui.available_rect_before_wrap().intersect(ui.max_rect());
    let height = viewport.height();
    fill_ui_width(ui);
    if height <= 1.0 {
        return add(ui);
    }

    ui.scope(|ui| {
        ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
        ScrollArea::vertical()
            .id_salt(id_salt)
            .max_width(ui.available_width())
            .max_height(height)
            .min_scrolled_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                fill_ui_width(ui);
                add(ui)
            })
            .inner
    })
    .inner
}

fn show_with_modal(
    ctx: &Context,
    modal_id: Id,
    p: &UiPalette,
    options: ModalOptions<'_>,
    close_label: &str,
    mut render: impl FnMut(ModalSection<'_>),
) -> ModalResponse {
    let mut close_clicked = false;
    let egui_resp = egui::Modal::new(modal_id)
        .area(modal_area(modal_id))
        .backdrop_color(backdrop_color(p, options.backdrop))
        .frame(window_frame(ctx, p))
        .show(ctx, |ui| {
            close_clicked = modal_content(ui, p, options, close_label, &mut render);
        });

    ModalResponse {
        dismissed_by_backdrop: options.close_enabled
            && options.backdrop.click_to_close
            && egui_resp.should_close(),
        close_clicked,
    }
}

fn show_without_backdrop(
    ctx: &Context,
    modal_id: Id,
    p: &UiPalette,
    options: ModalOptions<'_>,
    close_label: &str,
    mut render: impl FnMut(ModalSection<'_>),
) -> ModalResponse {
    let close_clicked = modal_area(modal_id)
        .show(ctx, |ui| {
            window_frame(ctx, p)
                .show(ui, |ui| {
                    modal_content(ui, p, options, close_label, &mut render)
                })
                .inner
        })
        .inner;

    ModalResponse {
        dismissed_by_backdrop: false,
        close_clicked,
    }
}

pub fn show_modal(
    ctx: &Context,
    modal_id: Id,
    p: &UiPalette,
    options: ModalOptions<'_>,
    open: &mut bool,
    close_label: &str,
    render: impl FnMut(ModalSection<'_>),
) -> Option<ModalResponse> {
    if !*open {
        return None;
    }

    let options = ModalOptions {
        size: options.size.resolve(ctx),
        ..options
    };
    let resp = if options.backdrop.enabled {
        show_with_modal(ctx, modal_id, p, options, close_label, render)
    } else {
        show_without_backdrop(ctx, modal_id, p, options, close_label, render)
    };

    if resp.dismissed_by_backdrop || resp.close_clicked {
        *open = false;
    }

    Some(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::gui_settings::{ColorScheme, FONT_SIZE_PT_DEFAULT, ThemeMode};

    fn modal_input(frame: usize) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1024.0, 768.0),
            )),
            time: Some(frame as f64 / 60.0),
            ..Default::default()
        }
    }

    fn render_modal_frame(
        ctx: &Context,
        modal_id: Id,
        frame: usize,
    ) -> (egui::Rect, egui::Rect, egui::Rect) {
        let p = theme::resolve(ThemeMode::Light, ColorScheme::Slate);
        let mut open = true;
        let mut main_rect = None;
        let mut footer_rect = None;
        theme::apply(
            ctx,
            ThemeMode::Light,
            ColorScheme::Slate,
            FONT_SIZE_PT_DEFAULT,
        );
        ctx.begin_pass(modal_input(frame));
        show_modal(
            ctx,
            modal_id,
            &p,
            ModalOptions::new("Test", ModalSize::preferred(560.0, 480.0)),
            &mut open,
            "Close",
            |section| match section {
                ModalSection::Main(ui) => {
                    main_rect = Some(ui.max_rect());
                    modal_scroll_vertical(ui, "modal_test_body", |ui| {
                        ui.set_min_height(2_000.0);
                        for idx in 0..120 {
                            ui.label(format!("Line {idx}"));
                        }
                    });
                }
                ModalSection::FooterCustom(ui) => {
                    footer_rect = Some(ui.max_rect());
                    ui.label("Footer");
                }
            },
        )
        .expect("modal should render");
        let _ = ctx.end_pass();
        let area = ctx
            .memory(|mem| mem.area_rect(modal_id))
            .expect("modal area should be stored");
        (area, main_rect.unwrap(), footer_rect.unwrap())
    }

    #[test]
    fn preferred_modal_width_is_stable_across_frames() {
        let ctx = Context::default();
        let modal_id = Id::new("preferred_modal_width_test");
        let (first_area, _, _) = render_modal_frame(&ctx, modal_id, 0);

        for frame in 1..8 {
            let (area, _, _) = render_modal_frame(&ctx, modal_id, frame);
            assert!(
                (area.width() - first_area.width()).abs() <= f32::EPSILON,
                "modal width changed from {first_area:?} to {area:?} on frame {frame}"
            );
        }
    }

    #[test]
    fn preferred_modal_footer_slot_stays_inside_body_width() {
        let ctx = Context::default();
        let modal_id = Id::new("preferred_modal_alignment_test");
        let _ = render_modal_frame(&ctx, modal_id, 0);
        let (_, main, footer) = render_modal_frame(&ctx, modal_id, 1);

        assert!(
            footer.left() >= main.left() && footer.right() <= main.right(),
            "modal footer custom slot should stay inside body width; main={main:?}, footer={footer:?}"
        );
    }
}
