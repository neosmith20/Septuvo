//! The Simple main window, exactly as `docs/ui/desktop/Default_VPN_UI.png`
//! (DEFAULT-UI §2, §3): header lockup and Settings cog, profile selector,
//! shield, status heading and lines, status row, main button, divider and
//! the three footer tiles. Layout only: every value comes from
//! [`MainView`]. Sizes are the runtime targets of DEFAULT-UI §2.3.
//!
//! **Hero artwork:** the approved globe and glow art waits for the owner's
//! decision N4-D9. Until then the ring and shield are plain outlines drawn
//! from the brand tokens, with no globe.

use eframe::egui::{
    self, Align, Align2, Color32, Layout, Mesh, Rect, Response, RichText, Sense, Shape, Stroke,
    StrokeKind, Ui, Vec2, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::assets::Textures;
use crate::icons;
use crate::prefs::InterfaceMode;
use crate::state::{ButtonAction, MainView, Shield};
use crate::theme::{self, bold, medium, semibold, token};

/// Below this width the footer tiles stack vertically (DEFAULT-UI §2.3).
pub const STACK_BELOW: f32 = 700.0;
/// Content is never wider than this; larger windows centre it.
pub const MAX_CONTENT: f32 = 1100.0;

/// The secondary windows opened from the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    /// Connections (footer tile).
    Connections,
    /// VPN Configuration (Add / Import tile).
    AddImport,
    /// Help & Diagnostics (footer tile).
    HelpDiagnostics,
    /// Settings (cog).
    Settings,
}

/// What the user did this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Activated the main button.
    Main(ButtonAction),
    /// Opened a secondary window.
    Open(Shell),
    /// Chose an Interface Mode in Settings (never a network action).
    SetMode(InterfaceMode),
    /// Asked the service to set Local Network Access (true: Allowed).
    SetLocalNetwork(bool),
}

pub(crate) fn translucent(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

pub(crate) fn focus_ring(ui: &Ui, resp: &Response, rect: Rect, radius: f32) {
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            radius + 3.0,
            Stroke::new(2.0, token::ELECTRIC),
            StrokeKind::Outside,
        );
    }
}

/// A rounded rectangle filled with a left-to-right gradient.
pub(crate) fn gradient_pill(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    left: Color32,
    right: Color32,
) {
    let r = radius.min(rect.height() / 2.0).min(rect.width() / 2.0);
    let corners = [
        (
            pos2(rect.right() - r, rect.top() + r),
            -std::f32::consts::FRAC_PI_2,
        ),
        (pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (
            pos2(rect.left() + r, rect.bottom() - r),
            std::f32::consts::FRAC_PI_2,
        ),
        (pos2(rect.left() + r, rect.top() + r), std::f32::consts::PI),
    ];
    let mut pts = Vec::new();
    for (c, start) in corners {
        for i in 0..=12 {
            #[allow(clippy::cast_precision_loss)]
            let a = start + std::f32::consts::FRAC_PI_2 * (i as f32 / 12.0);
            pts.push(pos2(c.x + r * a.cos(), c.y + r * a.sin()));
        }
    }
    let colour = |x: f32| {
        let t = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        left.lerp_to_gamma(right, t)
    };
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.center(), colour(rect.center().x));
    for p in &pts {
        mesh.colored_vertex(*p, colour(p.x));
    }
    #[allow(clippy::cast_possible_truncation)]
    let n = pts.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(Shape::mesh(mesh));
    // A thin outline smooths the mesh edge.
    painter.rect_stroke(
        rect,
        r,
        Stroke::new(1.0, colour(rect.center().x)),
        StrokeKind::Inside,
    );
}

fn header(ui: &mut Ui, tex: &Textures, width: f32) -> Rect {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 80.0), Sense::hover());
    lockup(ui, tex, pos2(rect.left() + 24.0, rect.center().y), 1.0);
    rect
}

/// The "Septuvo VPN" lockup, left-centred at `at`, `scale` 1.0 = the Simple
/// header (S mark 54 px high). Registered as one accessible image named
/// "Septuvo VPN". Returns its rectangle.
pub(crate) fn lockup(ui: &mut Ui, tex: &Textures, at: egui::Pos2, scale: f32) -> Rect {
    let painter = ui.painter().clone();
    let full = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let mut x = at.x;
    if let Some(s) = &tex.s_mark {
        let h = 54.0 * scale;
        let r = Rect::from_min_size(pos2(x, at.y - h / 2.0), vec2(h * 103.0 / 160.0, h));
        painter.image(s.id(), r, full, Color32::WHITE);
        x = r.right() + 12.0 * scale;
    }
    if let Some(w) = &tex.wordmark {
        let h = 30.0 * scale;
        let r = Rect::from_min_size(pos2(x, at.y - h / 2.0 + scale), vec2(h * 385.0 / 72.0, h));
        painter.image(w.id(), r, full, Color32::WHITE);
        x = r.right() + 9.0 * scale;
    }
    let vpn = painter.text(
        pos2(x, at.y - scale),
        Align2::LEFT_CENTER,
        "VPN",
        bold(29.0 * scale),
        token::BLUE,
    );
    // The lockup is one accessible image named "Septuvo VPN", not three parts.
    let rect = Rect::from_min_max(
        pos2(at.x, at.y - 27.0 * scale),
        pos2(vpn.right(), at.y + 27.0 * scale),
    );
    let resp = ui.interact(rect, ui.id().with("septuvo-vpn-lockup"), Sense::hover());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Image, true, "Septuvo VPN"));
    rect
}

fn selector(ui: &mut Ui, v: &MainView, width: f32) {
    let w = width.clamp(240.0, 580.0);
    let (rect, resp) = ui.allocate_exact_size(
        vec2(w, 66.0),
        if v.selector.enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    resp.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::ComboBox,
            v.selector.enabled,
            format!("Connection: {}", v.selector.text),
        )
    });
    let painter = ui.painter_at(rect.expand(4.0));
    painter.rect_filled(rect, 15.0, token::NAVY);
    painter.rect_stroke(
        rect,
        15.0,
        Stroke::new(1.0, translucent(token::BLUE, 120)),
        StrokeKind::Inside,
    );
    let icon = Rect::from_center_size(pos2(rect.left() + 42.0, rect.center().y), Vec2::splat(30.0));
    icons::servers(&painter, icon, token::BLUE);
    let text_colour = if v.selector.enabled {
        token::WHITE
    } else {
        token::MUTED
    };
    painter.text(
        pos2(rect.left() + 82.0, rect.center().y),
        Align2::LEFT_CENTER,
        &v.selector.text,
        semibold(22.0),
        text_colour,
    );
    icons::chevron_down(
        &painter,
        Rect::from_center_size(
            pos2(rect.right() - 34.0, rect.center().y),
            Vec2::splat(22.0),
        ),
        text_colour,
    );
    focus_ring(ui, &resp, rect, 15.0);
}

fn shield_area(ui: &mut Ui, shield: Shield, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_shield(&ui.painter_at(rect), rect, shield);
}

/// The ring and shield in `rect` (a square). Decoration: the status is
/// always in words too.
pub(crate) fn paint_shield(painter: &egui::Painter, rect: Rect, shield: Shield) {
    let size = rect.width().min(rect.height());
    let c = rect.center();
    let colour = match shield {
        Shield::Verified => token::ELECTRIC,
        Shield::Neutral => token::MUTED,
        Shield::Working | Shield::Warning => token::YELLOW,
        Shield::Error => token::RED,
    };
    // Plain ring and shield (hero artwork pending N4-D9).
    painter.circle_filled(c, size * 0.42, token::NAVY);
    painter.circle_stroke(
        c,
        size * 0.42,
        Stroke::new(size * 0.03, translucent(colour, 210)),
    );
    painter.circle_stroke(c, size * 0.47, Stroke::new(1.5, translucent(colour, 70)));
    icons::shield(
        painter,
        Rect::from_center_size(c, Vec2::splat(size * 0.42)),
        colour,
        shield == Shield::Verified,
    );
}

/// Centred text that wraps within the 20 px side margins.
fn centred_label(ui: &mut Ui, text: RichText) {
    let room = (ui.available_width() - 40.0).max(120.0);
    ui.allocate_ui_with_layout(vec2(room, 0.0), Layout::top_down(Align::Center), |ui| {
        ui.set_max_width(room);
        ui.add(egui::Label::new(text).wrap());
    });
}

#[allow(clippy::many_single_char_names)] // short geometry names, as in egui
fn main_button(ui: &mut Ui, v: &MainView, width: f32) -> bool {
    let w = width.min(500.0);
    let b = &v.button;
    let (rect, resp) = ui.allocate_exact_size(
        vec2(w, 76.0),
        if b.enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, b.enabled, &b.accessible));
    let painter = ui.painter_at(rect.expand(6.0));
    let radius = 38.0;
    let text_colour = if b.enabled {
        let (l, r) = if resp.hovered() {
            (token::SKY, token::ELECTRIC)
        } else {
            (token::BLUE, token::ELECTRIC)
        };
        gradient_pill(&painter, rect, radius, l, r);
        token::WHITE
    } else {
        painter.rect_filled(rect, radius, token::SURFACE);
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, token::BORDER),
            StrokeKind::Inside,
        );
        token::MUTED
    };
    let galley = painter.layout_no_wrap(b.label.to_string(), semibold(24.0), text_colour);
    let icon = 30.0;
    let gap = 18.0;
    let total = icon + gap + galley.size().x;
    let left = rect.center().x - total / 2.0;
    icons::power(
        &painter,
        Rect::from_center_size(pos2(left + icon / 2.0, rect.center().y), Vec2::splat(icon)),
        text_colour,
    );
    painter.galley(
        pos2(left + icon + gap, rect.center().y - galley.size().y / 2.0),
        galley,
        text_colour,
    );
    focus_ring(ui, &resp, rect, radius);
    b.enabled && resp.clicked()
}

fn tile(
    ui: &mut Ui,
    size: Vec2,
    title: &str,
    subtitle: &str,
    draw_icon: fn(&egui::Painter, Rect, Color32),
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| {
        WidgetInfo::labeled(WidgetType::Button, true, format!("{title}, {subtitle}"))
    });
    let painter = ui.painter_at(rect.expand(4.0));
    let fill = if resp.hovered() {
        token::SURFACE
    } else {
        token::NAVY
    };
    painter.rect_filled(rect, 14.0, fill);
    painter.rect_stroke(
        rect,
        14.0,
        Stroke::new(1.0, translucent(token::BORDER, 200)),
        StrokeKind::Inside,
    );
    // 14 px padding, 34 px pictogram, 12 px gap (DEFAULT-UI §2.3).
    draw_icon(
        &painter,
        Rect::from_center_size(pos2(rect.left() + 31.0, rect.center().y), Vec2::splat(34.0)),
        token::BLUE,
    );
    let text_left = rect.left() + 60.0;
    let room = (rect.right() - 34.0 - text_left).max(40.0);
    // The subtitle wraps onto a second line rather than shrinking or being
    // cut off when the tile is narrow.
    let title_g = painter.layout_no_wrap(title.to_string(), semibold(15.0), token::WHITE);
    let sub_g = painter.layout(
        subtitle.to_string(),
        egui::FontId::proportional(13.0),
        token::MUTED,
        room,
    );
    let gap = 3.0;
    let top = rect.center().y - (title_g.size().y + gap + sub_g.size().y) / 2.0;
    let sub_h = title_g.size().y;
    painter.galley(pos2(text_left, top), title_g, token::WHITE);
    painter.galley(pos2(text_left, top + sub_h + gap), sub_g, token::MUTED);
    icons::chevron_right(
        &painter,
        Rect::from_center_size(
            pos2(rect.right() - 22.0, rect.center().y),
            Vec2::splat(18.0),
        ),
        token::WHITE,
    );
    focus_ring(ui, &resp, rect, 14.0);
    resp.clicked()
}

type TileSpec = (
    &'static str,
    &'static str,
    fn(&egui::Painter, Rect, Color32),
    Shell,
);

/// The footer tiles, in the approved order (never changes).
pub const TILES: [TileSpec; 3] = [
    (
        "Connections",
        "Manage Saved Connections",
        icons::servers,
        Shell::Connections,
    ),
    (
        "Add / Import",
        "VPN Configuration",
        icons::circle_plus,
        Shell::AddImport,
    ),
    (
        "Help & Diagnostics",
        "Get Help or Run Tests",
        icons::circle_question,
        Shell::HelpDiagnostics,
    ),
];

/// The coloured dot and status text, centred together.
fn status_row(ui: &mut Ui, v: &MainView) {
    ui.horizontal(|ui| {
        let text = egui::Label::new(
            RichText::new(&v.status.text)
                .font(medium(16.0))
                .color(token::WHITE),
        );
        let galley_w = ui.ctx().fonts_mut(|f| {
            f.layout_no_wrap(v.status.text.clone(), medium(16.0), token::WHITE)
                .size()
                .x
        });
        ui.add_space(((ui.available_width() - galley_w - 20.0) / 2.0).max(0.0));
        let (dot, _) = ui.allocate_exact_size(vec2(12.0, 16.0), Sense::hover());
        ui.painter()
            .circle_filled(dot.center(), 5.0, theme::tone(v.status.tone));
        ui.add(text);
    });
}

/// The three footer tiles: in a row when the window is wide enough, else
/// stacked in the same order (DEFAULT-UI §2.3).
fn footer(ui: &mut Ui, inner: f32, row: bool, actions: &mut Vec<Action>) {
    if row {
        let tile_w = (inner - 2.0 * 12.0) / 3.0;
        ui.allocate_ui_with_layout(
            vec2(inner, 74.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                for (title, sub, icon, shell) in TILES {
                    if tile(ui, vec2(tile_w, 74.0), title, sub, icon) {
                        actions.push(Action::Open(shell));
                    }
                }
            },
        );
    } else {
        for (title, sub, icon, shell) in TILES {
            if tile(ui, vec2(inner, 74.0), title, sub, icon) {
                actions.push(Action::Open(shell));
            }
            ui.add_space(8.0);
        }
    }
}

/// Draws the whole main window and returns what the user did.
pub fn draw(ui: &mut Ui, v: &MainView, tex: &Textures) -> Vec<Action> {
    let mut actions = Vec::new();
    let full = ui.available_width();
    let width = full.min(MAX_CONTENT);
    let margin = 20.0;
    let mut cog_rect = None;
    // Everything but the shield takes about this much height; the shield
    // shrinks to keep the footer tiles in view, down to 160 px, and below
    // that the window scrolls (DEFAULT-UI §2.3 responsive behaviour).
    let fixed = 516.0
        + if v.notice.is_some() { 24.0 } else { 0.0 }
        + if full < STACK_BELOW { 172.0 } else { 0.0 };
    let shield_size = (ui.available_height() - fixed).clamp(160.0, 220.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = 0.0;
                let header_rect = header(ui, tex, width);
                cog_rect = Some(Rect::from_center_size(
                    pos2(header_rect.right() - 40.0, header_rect.center().y),
                    Vec2::splat(44.0),
                ));
                ui.add_space(4.0);
                selector(ui, v, width - 2.0 * margin);
                ui.add_space(10.0);
                shield_area(ui, v.shield, shield_size);
                ui.add_space(4.0);
                // 44 px for short headings, 34 px for long ones, smaller still
                // if it would not fit the window on one line (never below 24).
                let mut heading_size: f32 = if v.heading.chars().count() <= 12 {
                    44.0
                } else {
                    34.0
                };
                let room = width - 2.0 * margin;
                let natural = ui.ctx().fonts_mut(|f| {
                    f.layout_no_wrap(v.heading.to_string(), bold(heading_size), token::WHITE)
                        .size()
                        .x
                });
                if natural > room {
                    heading_size = (heading_size * room / natural).floor().max(24.0);
                }
                centred_label(
                    ui,
                    RichText::new(v.heading)
                        .font(bold(heading_size))
                        .color(token::WHITE),
                );
                ui.add_space(4.0);
                for line in v.lines.iter().take(2) {
                    centred_label(ui, RichText::new(line).size(16.0).color(token::WHITE));
                }
                ui.add_space(6.0);
                status_row(ui, v);
                if let Some(n) = &v.notice {
                    ui.add_space(4.0);
                    centred_label(ui, RichText::new(n).size(15.0).color(token::RED));
                }
                ui.add_space(16.0);
                if main_button(ui, v, width - 2.0 * margin) {
                    actions.push(Action::Main(v.button.action));
                }
                if let (Some(reason), false) = (v.button.reason, v.button.enabled) {
                    ui.add_space(6.0);
                    centred_label(ui, RichText::new(reason).size(14.0).color(token::MUTED));
                }
                ui.add_space(16.0);
                let (line, _) =
                    ui.allocate_exact_size(vec2(width - 2.0 * margin, 1.0), Sense::hover());
                ui.painter().line_segment(
                    [line.left_center(), line.right_center()],
                    Stroke::new(1.0, translucent(token::BORDER, 160)),
                );
                ui.add_space(16.0);
                footer(ui, width - 2.0 * margin, full >= STACK_BELOW, &mut actions);
                ui.add_space(20.0);
            });
        });
    // The Settings cog is created last so keyboard focus reaches it after
    // the footer tiles (DEFAULT-UI §9.1), though it is drawn in the header.
    if let Some(r) = cog_rect {
        let resp = ui.interact(r, ui.id().with("settings-cog"), Sense::click());
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, "Settings"));
        let colour = if resp.hovered() {
            token::ELECTRIC
        } else {
            token::WHITE
        };
        icons::gear(ui.painter(), r.shrink(9.0), colour);
        focus_ring(ui, &resp, r, 10.0);
        if resp.clicked() {
            actions.push(Action::Open(Shell::Settings));
        }
    }
    actions
}
