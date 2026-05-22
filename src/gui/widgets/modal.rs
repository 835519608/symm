//! 模态弹窗封装（基于 egui [`Modal`]），遮罩层级由框架保证。
//! 各页面通过 [`ModalOptions`] 配置标题、尺寸、遮罩，无需改本文件。

use crate::gui::theme::{self, UiPalette, rich_section};
use egui::{self, Area, Color32, Context, Frame, Id, Margin, Order, Ui, Vec2};

/// 弹窗尺寸。
#[derive(Clone, Copy, Debug)]
pub struct ModalSize {
    pub width: f32,
    /// `None`：高度随内容；`Some(h)`：最小高度（内容区可自行 `ScrollArea`）。
    pub height: Option<f32>,
}

impl ModalSize {
    pub const fn fit_content(width: f32) -> Self {
        Self {
            width,
            height: None,
        }
    }
}

/// 遮罩行为（各页面可覆盖默认值）。
#[derive(Clone, Copy, Debug)]
pub struct ModalBackdrop {
    /// 是否绘制半透明遮罩。
    pub enabled: bool,
    /// 点击遮罩空白处是否视为关闭。
    pub click_to_close: bool,
    /// 遮罩不透明度 0–255；`None` 时按明暗主题取默认。
    pub alpha: Option<u8>,
}

impl ModalBackdrop {
    pub const DEFAULT: Self = Self {
        enabled: true,
        click_to_close: true,
        alpha: None,
    };
}

/// 打开模态窗时的参数（页面构造，不改组件代码）。
#[derive(Clone, Copy, Debug)]
pub struct ModalOptions<'a> {
    pub title: &'a str,
    pub size: ModalSize,
    pub backdrop: ModalBackdrop,
}

impl<'a> ModalOptions<'a> {
    pub fn new(title: &'a str, size: ModalSize) -> Self {
        Self {
            title,
            size,
            backdrop: ModalBackdrop::DEFAULT,
        }
    }
}

/// [`show_modal`] 的返回值。
pub struct ModalResponse {
    pub dismissed_by_backdrop: bool,
}

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
        .inner_margin(Margin::same(16.0))
}

fn apply_content_size(ui: &mut Ui, size: ModalSize) {
    ui.set_width(size.width);
    ui.set_min_width(size.width);
    ui.set_max_width(size.width);
    if let Some(h) = size.height {
        ui.set_min_height(h);
    }
}

fn modal_body<R>(
    ui: &mut Ui,
    p: &UiPalette,
    options: ModalOptions<'_>,
    add_body: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.label(rich_section(options.title, p.text));
    ui.add_space(10.0);
    apply_content_size(ui, options.size);
    add_body(ui)
}

fn show_with_modal<R>(
    ctx: &Context,
    modal_id: Id,
    p: &UiPalette,
    options: ModalOptions<'_>,
    add_body: impl FnOnce(&mut Ui) -> R,
) -> ModalResponse {
    let color = backdrop_color(p, options.backdrop);
    let egui_resp = egui::Modal::new(modal_id)
        .backdrop_color(color)
        .frame(window_frame(ctx, p))
        .show(ctx, |ui| modal_body(ui, p, options, add_body));

    ModalResponse {
        dismissed_by_backdrop: options.backdrop.click_to_close && egui_resp.should_close(),
    }
}

fn show_without_backdrop<R>(
    ctx: &Context,
    modal_id: Id,
    p: &UiPalette,
    options: ModalOptions<'_>,
    add_body: impl FnOnce(&mut Ui) -> R,
) -> ModalResponse {
    Area::new(modal_id)
        .order(Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .interactable(true)
        .show(ctx, |ui| {
            window_frame(ctx, p)
                .show(ui, |ui| modal_body(ui, p, options, add_body))
                .inner
        });

    ModalResponse {
        dismissed_by_backdrop: false,
    }
}

/// 显示模态窗。`open == false` 时不绘制。
pub fn show_modal<R>(
    ctx: &Context,
    modal_id: Id,
    p: &UiPalette,
    options: ModalOptions<'_>,
    open: &mut bool,
    add_body: impl FnOnce(&mut Ui) -> R,
) -> Option<ModalResponse> {
    if !*open {
        return None;
    }

    let resp = if options.backdrop.enabled {
        show_with_modal(ctx, modal_id, p, options, add_body)
    } else {
        show_without_backdrop(ctx, modal_id, p, options, add_body)
    };

    if resp.dismissed_by_backdrop {
        *open = false;
    }

    Some(resp)
}
