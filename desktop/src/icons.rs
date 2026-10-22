//! Line pictograms drawn in code, matching the approved image's outline
//! style (brand blue strokes on dark tiles). Each fits the given square.
//! Drawn, not bitmaps, so they stay sharp at every display scale. They are
//! decoration: every control carries a text label.

// Short geometry names (r, c, p, s, t), as in egui's own drawing code.
#![allow(clippy::many_single_char_names)]

use eframe::egui::{Color32, Painter, Pos2, Rect, Stroke, pos2, vec2};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

fn p(r: Rect, x: f32, y: f32) -> Pos2 {
    pos2(r.left() + r.width() * x, r.top() + r.height() * y)
}

fn arc(center: Pos2, radius: f32, from: f32, to: f32, steps: usize) -> Vec<Pos2> {
    (0..=steps)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let a = from + (to - from) * (i as f32 / steps as f32);
            pos2(center.x + radius * a.cos(), center.y + radius * a.sin())
        })
        .collect()
}

fn line(painter: &Painter, pts: Vec<Pos2>, s: Stroke) {
    painter.add(eframe::egui::Shape::line(pts, s));
}

fn width(r: Rect) -> f32 {
    (r.width() / 14.0).max(1.5)
}

/// A downward chevron (selector).
pub fn chevron_down(painter: &Painter, r: Rect, c: Color32) {
    line(
        painter,
        vec![p(r, 0.2, 0.38), p(r, 0.5, 0.66), p(r, 0.8, 0.38)],
        Stroke::new(width(r), c),
    );
}

/// A right chevron (tiles).
pub fn chevron_right(painter: &Painter, r: Rect, c: Color32) {
    line(
        painter,
        vec![p(r, 0.38, 0.2), p(r, 0.66, 0.5), p(r, 0.38, 0.8)],
        Stroke::new(width(r), c),
    );
}

/// A house (the `Home VPN` example; DEFAULT-UI §3.1).
pub fn house(painter: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(width(r), c);
    line(
        painter,
        vec![p(r, 0.1, 0.48), p(r, 0.5, 0.12), p(r, 0.9, 0.48)],
        s,
    );
    line(
        painter,
        vec![
            p(r, 0.22, 0.4),
            p(r, 0.22, 0.88),
            p(r, 0.78, 0.88),
            p(r, 0.78, 0.4),
        ],
        s,
    );
    line(
        painter,
        vec![
            p(r, 0.42, 0.88),
            p(r, 0.42, 0.62),
            p(r, 0.58, 0.62),
            p(r, 0.58, 0.88),
        ],
        s,
    );
}

/// Two stacked servers (Connections; also the neutral connection icon).
pub fn servers(painter: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(width(r), c);
    for (top, bottom) in [(0.14, 0.44), (0.56, 0.86)] {
        let rect = Rect::from_min_max(p(r, 0.08, top), p(r, 0.92, bottom));
        painter.rect_stroke(rect, r.width() * 0.1, s, eframe::egui::StrokeKind::Middle);
        let y = f32::midpoint(rect.top(), rect.bottom());
        line(
            painter,
            vec![
                pos2(rect.left() + r.width() * 0.16, y),
                pos2(rect.left() + r.width() * 0.36, y),
            ],
            s,
        );
        painter.circle_filled(pos2(rect.right() - r.width() * 0.16, y), width(r) * 0.8, c);
    }
}

/// A plus in a circle (Add / Import).
pub fn circle_plus(painter: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(width(r), c);
    painter.circle_stroke(r.center(), r.width() * 0.44, s);
    line(painter, vec![p(r, 0.5, 0.28), p(r, 0.5, 0.72)], s);
    line(painter, vec![p(r, 0.28, 0.5), p(r, 0.72, 0.5)], s);
}

/// A question mark in a circle (Help & Diagnostics).
pub fn circle_question(painter: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(width(r), c);
    painter.circle_stroke(r.center(), r.width() * 0.44, s);
    let mut hook = arc(
        p(r, 0.5, 0.4),
        r.width() * 0.13,
        PI,
        TAU + FRAC_PI_2 * 0.6,
        14,
    );
    hook.push(p(r, 0.5, 0.6));
    line(painter, hook, s);
    painter.circle_filled(p(r, 0.5, 0.72), width(r) * 0.75, c);
}

/// A gear (Settings).
pub fn gear(painter: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(width(r), c);
    let center = r.center();
    let (outer, inner) = (r.width() * 0.46, r.width() * 0.34);
    let teeth: u8 = 8;
    let mut pts = Vec::new();
    for i in 0..teeth {
        let a = TAU * f32::from(i) / f32::from(teeth);
        let half = TAU / f32::from(teeth) / 4.0;
        for (angle, radius) in [
            (a - half * 1.2, inner),
            (a - half * 0.6, outer),
            (a + half * 0.6, outer),
            (a + half * 1.2, inner),
        ] {
            pts.push(pos2(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            ));
        }
    }
    pts.push(pts[0]);
    line(painter, pts, s);
    painter.circle_stroke(center, r.width() * 0.14, s);
}

/// The power symbol (main button).
pub fn power(painter: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(width(r) * 1.2, c);
    // An open ring with the gap at the top, and a vertical bar.
    line(
        painter,
        arc(
            r.center() + vec2(0.0, r.height() * 0.04),
            r.width() * 0.38,
            -PI * 0.32,
            PI * 1.32,
            28,
        ),
        s,
    );
    line(painter, vec![p(r, 0.5, 0.06), p(r, 0.5, 0.48)], s);
}

/// A shield outline, optionally with a check mark (the large status shield).
pub fn shield(painter: &Painter, r: Rect, c: Color32, check: bool) {
    let s = Stroke::new((r.width() / 16.0).max(2.0), c);
    // Quadratic curve from the side down to the point, sampled.
    let curve = |from: Pos2, ctrl: Pos2, to: Pos2| {
        (1..=12).map(move |i| {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f32 / 12.0;
            let u = 1.0 - t;
            pos2(
                u * u * from.x + 2.0 * u * t * ctrl.x + t * t * to.x,
                u * u * from.y + 2.0 * u * t * ctrl.y + t * t * to.y,
            )
        })
    };
    let mut pts = vec![p(r, 0.5, 0.06), p(r, 0.86, 0.19), p(r, 0.86, 0.46)];
    pts.extend(curve(p(r, 0.86, 0.46), p(r, 0.84, 0.8), p(r, 0.5, 0.94)));
    pts.extend(curve(p(r, 0.5, 0.94), p(r, 0.16, 0.8), p(r, 0.14, 0.46)));
    pts.extend([p(r, 0.14, 0.19), p(r, 0.5, 0.06)]);
    line(painter, pts, s);
    if check {
        line(
            painter,
            vec![p(r, 0.32, 0.5), p(r, 0.45, 0.63), p(r, 0.7, 0.36)],
            Stroke::new(s.width * 1.2, c),
        );
    }
}

/// A globe: circle, equator, two latitudes and a meridian ellipse
/// (Connections tab, the IP row).
pub fn globe(painter: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(width(r), c);
    let center = r.center();
    let radius = r.width() * 0.44;
    painter.circle_stroke(center, radius, s);
    line(painter, vec![p(r, 0.06, 0.5), p(r, 0.94, 0.5)], s);
    for y in [0.3, 0.7] {
        line(painter, vec![p(r, 0.14, y), p(r, 0.86, y)], s);
    }
    let meridian: Vec<Pos2> = (0..=32u8)
        .map(|i| {
            let a = TAU * f32::from(i) / 32.0;
            pos2(
                center.x + radius * 0.45 * a.cos(),
                center.y + radius * a.sin(),
            )
        })
        .collect();
    line(painter, meridian, s);
    line(painter, vec![p(r, 0.5, 0.06), p(r, 0.5, 0.94)], s);
}

/// A monitor with a pulse line (Diagnostics tab).
pub fn monitor(painter: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(width(r), c);
    let screen = Rect::from_min_max(p(r, 0.08, 0.14), p(r, 0.92, 0.72));
    painter.rect_stroke(
        screen,
        r.width() * 0.06,
        s,
        eframe::egui::StrokeKind::Middle,
    );
    line(painter, vec![p(r, 0.5, 0.72), p(r, 0.5, 0.86)], s);
    line(painter, vec![p(r, 0.3, 0.88), p(r, 0.7, 0.88)], s);
    line(
        painter,
        vec![
            p(r, 0.2, 0.46),
            p(r, 0.38, 0.46),
            p(r, 0.46, 0.3),
            p(r, 0.56, 0.58),
            p(r, 0.64, 0.42),
            p(r, 0.8, 0.42),
        ],
        s,
    );
}

/// A filled rounded square: the approved Advanced Disconnect symbol.
pub fn stop(painter: &Painter, r: Rect, c: Color32) {
    let square = Rect::from_center_size(r.center(), r.size() * 0.62);
    painter.rect_filled(square, r.width() * 0.14, c);
}

/// Two overlapping rounded squares (copy to clipboard).
pub fn copy(painter: &Painter, r: Rect, c: Color32) {
    let s = Stroke::new(width(r), c);
    let back = Rect::from_min_max(p(r, 0.34, 0.12), p(r, 0.88, 0.66));
    let front = Rect::from_min_max(p(r, 0.12, 0.34), p(r, 0.66, 0.88));
    painter.rect_stroke(back, r.width() * 0.08, s, eframe::egui::StrokeKind::Middle);
    painter.rect_filled(front, r.width() * 0.08, crate::theme::token::NAVY);
    painter.rect_stroke(front, r.width() * 0.08, s, eframe::egui::StrokeKind::Middle);
}
