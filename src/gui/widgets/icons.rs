//! 矢量图标（不依赖 emoji 字体，避免 Windows 仅挂 CJK 时显示为方框）。

use egui::{Color32, Painter, Pos2, Rect, Stroke, Ui, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Add,
    List,
    Detail,
    Delete,
    Settings,
    /// 主题：跟随系统
    Monitor,
    /// 主题：浅色
    Sun,
    /// 主题：深色
    Moon,
    Refresh,
    ChevronDown,
    ChevronRight,
    Search,
}

pub const ICON_SIZE: f32 = 16.0;

pub fn paint_icon(ui: &Ui, rect: Rect, icon: Icon, color: Color32) {
    let painter = ui.painter();
    paint_with_painter(painter, rect, icon, color);
}

pub fn paint_with_painter(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let side = rect.width().min(rect.height());
    let c = rect.center();
    let r = side * 0.38;
    let stroke = Stroke::new((side * 0.09).clamp(1.2, 2.0), color);

    match icon {
        Icon::Add => {
            painter.line_segment([c + Vec2::new(-r, 0.0), c + Vec2::new(r, 0.0)], stroke);
            painter.line_segment([c + Vec2::new(0.0, -r), c + Vec2::new(0.0, r)], stroke);
        }
        Icon::List => {
            let w = r * 1.1;
            for dy in [-r * 0.55, 0.0, r * 0.55] {
                painter.line_segment([c + Vec2::new(-w, dy), c + Vec2::new(w, dy)], stroke);
            }
        }
        Icon::Detail => {
            painter.circle_stroke(c + Vec2::new(-r * 0.15, -r * 0.1), r * 0.55, stroke);
            painter.line_segment(
                [
                    c + Vec2::new(r * 0.2, r * 0.35),
                    c + Vec2::new(r * 0.95, r * 1.0),
                ],
                stroke,
            );
        }
        Icon::Delete => {
            let top = c + Vec2::new(0.0, -r * 0.85);
            painter.line_segment([top + Vec2::new(-r, 0.0), top + Vec2::new(r, 0.0)], stroke);
            let body =
                Rect::from_center_size(c + Vec2::new(0.0, r * 0.15), Vec2::new(r * 1.5, r * 1.1));
            painter.rect_stroke(body, 2.0, stroke);
            painter.line_segment(
                [
                    c + Vec2::new(-r * 0.45, -r * 0.15),
                    c + Vec2::new(r * 0.45, -r * 0.15),
                ],
                stroke,
            );
        }
        Icon::Settings => {
            painter.circle_stroke(c, r * 0.42, stroke);
            for i in 0..6 {
                let a = std::f32::consts::FRAC_PI_3 * i as f32 - std::f32::consts::FRAC_PI_2;
                let dir = Vec2::angled(a);
                painter.line_segment([c + dir * r * 0.55, c + dir * r * 1.05], stroke);
            }
        }
        Icon::Monitor => {
            let screen = Rect::from_center_size(
                c + Vec2::new(0.0, -r * 0.12),
                Vec2::new(r * 1.35, r * 0.95),
            );
            painter.rect_stroke(screen, 2.0, stroke);
            let foot = c + Vec2::new(0.0, r * 0.72);
            painter.line_segment(
                [
                    foot + Vec2::new(-r * 0.35, 0.0),
                    foot + Vec2::new(r * 0.35, 0.0),
                ],
                stroke,
            );
            painter.line_segment([foot, foot + Vec2::new(0.0, r * 0.22)], stroke);
        }
        Icon::Sun => {
            painter.circle_stroke(c, r * 0.38, stroke);
            for i in 0..8 {
                let a = std::f32::consts::FRAC_PI_4 * i as f32;
                let dir = Vec2::angled(a);
                painter.line_segment([c + dir * r * 0.48, c + dir * r * 0.92], stroke);
            }
        }
        Icon::Moon => {
            painter.circle_stroke(c, r * 0.62, stroke);
            painter.line_segment(
                [
                    c + Vec2::new(-r * 0.15, -r * 0.62),
                    c + Vec2::new(-r * 0.15, r * 0.62),
                ],
                stroke,
            );
        }
        Icon::Refresh => {
            let radius = r * 0.58;
            let steps = 14usize;
            let mut pts = Vec::with_capacity(steps + 1);
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let a = std::f32::consts::FRAC_PI_2 * 1.55 * t + 0.65;
                pts.push(c + Vec2::angled(a) * radius);
            }
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            if let (Some(&tip), Some(&prev)) = (pts.last(), pts.get(pts.len().saturating_sub(2))) {
                let dir = (tip - prev).normalized();
                let perp = Vec2::new(-dir.y, dir.x);
                let wing = r * 0.28;
                painter.line_segment([tip, tip - dir * wing + perp * wing * 0.55], stroke);
                painter.line_segment([tip, tip - dir * wing - perp * wing * 0.55], stroke);
            }
            let _ = color;
        }
        Icon::ChevronDown => paint_chevron(painter, c, r, stroke, true),
        Icon::ChevronRight => paint_chevron(painter, c, r, stroke, false),
        Icon::Search => {
            painter.circle_stroke(c + Vec2::new(-r * 0.12, -r * 0.12), r * 0.52, stroke);
            painter.line_segment(
                [
                    c + Vec2::new(r * 0.22, r * 0.22),
                    c + Vec2::new(r * 0.92, r * 0.92),
                ],
                stroke,
            );
        }
    }
}

fn paint_chevron(painter: &Painter, c: Pos2, r: f32, stroke: Stroke, down: bool) {
    let (a, b, tip) = if down {
        (
            c + Vec2::new(-r * 0.55, -r * 0.2),
            c + Vec2::new(r * 0.55, -r * 0.2),
            c + Vec2::new(0.0, r * 0.55),
        )
    } else {
        (
            c + Vec2::new(-r * 0.2, -r * 0.55),
            c + Vec2::new(-r * 0.2, r * 0.55),
            c + Vec2::new(r * 0.55, 0.0),
        )
    };
    painter.line_segment([a, tip], stroke);
    painter.line_segment([b, tip], stroke);
}
