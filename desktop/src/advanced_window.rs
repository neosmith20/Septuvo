//! The Advanced main window, as `docs/ui/desktop/Advanced_VPN_UI.png`
//! (ADVANCED-UI §2, §4–§6, §11): header with the lockup and the Home,
//! Connections, Settings and Diagnostics tabs; the Home hero panel with the
//! headline, text, main button, connection card and IP card; and the Kill
//! Switch, DNS Protection and Local Network Access tiles. Layout only: every
//! value comes from [`HomeView`]. Sizes are the targets of ADVANCED-UI §4.4.
//!
//! **Hero artwork:** like the Simple window, the globe art waits for the
//! owner's decision N4-D9; a plain ring and shield stand in for it.

use eframe::egui::{
    self, Align2, Color32, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo,
    WidgetType, pos2, vec2,
};

use crate::assets::Textures;
use crate::home::{HomeView, IP_LABEL, Tile, TileKind};
use crate::icons;
use crate::main_window::{
    Action, MAX_CONTENT, Shell, focus_ring, gradient_pill, lockup, paint_shield, translucent,
};
use crate::prefs::InterfaceMode;
use crate::shells;
use crate::state::ButtonAction;
use crate::theme::{self, bold, medium, semibold, token};

/// Initial and minimum Advanced window sizes (ADVANCED-UI §4.2).
pub const INITIAL_SIZE: [f32; 2] = [980.0, 640.0];
/// See [`INITIAL_SIZE`].
pub const MIN_SIZE: [f32; 2] = [760.0, 560.0];

/// The tabs (ADVANCED-UI §5.2), in the approved order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    /// The Home dashboard.
    #[default]
    Home,
    /// Saved connections.
    Connections,
    /// The shared Settings.
    Settings,
    /// Diagnostics.
    Diagnostics,
}

type TabSpec = (&'static str, Page, fn(&egui::Painter, Rect, Color32));

const TABS: [TabSpec; 4] = [
    ("Home", Page::Home, icons::house),
    ("Connections", Page::Connections, icons::globe),
    ("Settings", Page::Settings, icons::gear),
    ("Diagnostics", Page::Diagnostics, icons::monitor),
];

const MARGIN: f32 = 28.0;
/// From this content width the three tiles sit in one row.
pub const ACROSS_FROM: f32 = 900.0;
const PANEL_FILL: Color32 = Color32::from_rgb(0x08, 0x10, 0x1F);

fn panel(ui: &Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 16.0, PANEL_FILL);
    ui.painter().rect_stroke(
        rect,
        16.0,
        Stroke::new(1.0, translucent(token::BORDER, 170)),
        StrokeKind::Inside,
    );
}

fn header(ui: &mut Ui, tex: &Textures, width: f32, page: &mut Page) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 80.0), Sense::hover());
    lockup(ui, tex, pos2(rect.left() + MARGIN, rect.center().y), 0.86);
    // Tabs start after the brand area (about 290 px), 36 px apart.
    let mut x = (rect.left() + 300.0).max(rect.center().x - 220.0);
    for (label, value, icon) in TABS {
        let galley = ui
            .painter()
            .layout_no_wrap(label.to_string(), medium(15.0), token::WHITE);
        let w = galley.size().x.max(44.0) + 16.0;
        let r = Rect::from_min_size(pos2(x, rect.top() + 10.0), vec2(w, 62.0));
        let resp = ui.interact(r, ui.id().with(("tab", label)), Sense::click());
        let selected = *page == value;
        resp.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, label)
        });
        let colour = if selected {
            token::WHITE
        } else if resp.hovered() {
            token::SKY
        } else {
            token::MUTED
        };
        icon(
            ui.painter(),
            Rect::from_center_size(pos2(r.center().x, r.top() + 17.0), Vec2::splat(26.0)),
            if selected { token::ELECTRIC } else { colour },
        );
        ui.painter().text(
            pos2(r.center().x, r.top() + 44.0),
            Align2::CENTER_CENTER,
            label,
            medium(15.0),
            colour,
        );
        if selected {
            let y = r.bottom() - 2.0;
            ui.painter().line_segment(
                [pos2(r.center().x - 22.0, y), pos2(r.center().x + 22.0, y)],
                Stroke::new(3.0, token::ELECTRIC),
            );
        }
        focus_ring(ui, &resp, r, 8.0);
        if resp.clicked() {
            *page = value;
        }
        x = r.right() + 26.0;
    }
}

fn main_button(ui: &mut Ui, v: &HomeView, rect: Rect) -> bool {
    let b = &v.main.button;
    let resp = ui.interact(
        rect,
        ui.id().with("advanced-main-button"),
        if b.enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, b.enabled, &b.accessible));
    let radius = 14.0;
    let painter = ui.painter();
    let text_colour = if b.enabled {
        let left = if resp.hovered() {
            token::SKY
        } else {
            token::BLUE
        };
        gradient_pill(painter, rect, radius, left, token::ELECTRIC);
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
    let galley = painter.layout_no_wrap(b.label.to_string(), semibold(22.0), text_colour);
    let (icon, gap) = (26.0, 16.0);
    let left = rect.center().x - (icon + gap + galley.size().x) / 2.0;
    let icon_rect =
        Rect::from_center_size(pos2(left + icon / 2.0, rect.center().y), Vec2::splat(icon));
    if b.action == ButtonAction::Disconnect {
        icons::stop(painter, icon_rect, text_colour);
    } else {
        icons::power(painter, icon_rect, text_colour);
    }
    painter.galley(
        pos2(left + icon + gap, rect.center().y - galley.size().y / 2.0),
        galley,
        text_colour,
    );
    focus_ring(ui, &resp, rect, radius);
    b.enabled && resp.clicked()
}

/// The connection card and the IP row, in one bordered box (as the image).
/// Returns true when the connection card was activated.
fn cards(ui: &mut Ui, v: &HomeView, rect: Rect) -> bool {
    ui.painter().rect_filled(rect, 14.0, token::NAVY);
    ui.painter().rect_stroke(
        rect,
        14.0,
        Stroke::new(1.0, translucent(token::BORDER, 200)),
        StrokeKind::Inside,
    );
    let split = rect.top() + rect.height() * 0.56;
    let card = Rect::from_min_max(rect.min, pos2(rect.right(), split));
    let resp = ui.interact(card, ui.id().with("connection-card"), Sense::click());
    resp.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::Button,
            true,
            format!(
                "Connection: {}, {}",
                v.connection.title, v.connection.detail
            ),
        )
    });
    if resp.hovered() {
        ui.painter()
            .rect_filled(card.shrink(2.0), 12.0, token::SURFACE);
    }
    let painter = ui.painter_at(rect);
    icons::servers(
        &painter,
        Rect::from_center_size(pos2(card.left() + 40.0, card.center().y), Vec2::splat(30.0)),
        token::BLUE,
    );
    let text_left = card.left() + 78.0;
    painter.text(
        pos2(text_left, card.center().y - 11.0),
        Align2::LEFT_CENTER,
        &v.connection.title,
        semibold(18.0),
        token::WHITE,
    );
    painter.text(
        pos2(text_left, card.center().y + 13.0),
        Align2::LEFT_CENTER,
        &v.connection.detail,
        egui::FontId::proportional(14.0),
        token::MUTED,
    );
    icons::chevron_right(
        &painter,
        Rect::from_center_size(
            pos2(card.right() - 26.0, card.center().y),
            Vec2::splat(18.0),
        ),
        token::WHITE,
    );
    focus_ring(ui, &resp, card, 14.0);
    painter.line_segment(
        [
            pos2(rect.left() + 20.0, split),
            pos2(rect.right() - 20.0, split),
        ],
        Stroke::new(1.0, translucent(token::BORDER, 160)),
    );
    let ip = Rect::from_min_max(pos2(rect.left(), split), rect.max);
    icons::globe(
        &painter,
        Rect::from_center_size(pos2(ip.left() + 40.0, ip.center().y), Vec2::splat(26.0)),
        token::BLUE,
    );
    painter.text(
        pos2(text_left, ip.center().y - 10.0),
        Align2::LEFT_CENTER,
        IP_LABEL,
        egui::FontId::proportional(14.0),
        token::MUTED,
    );
    painter.text(
        pos2(text_left, ip.center().y + 11.0),
        Align2::LEFT_CENTER,
        v.ip,
        medium(16.0),
        token::WHITE,
    );
    // The IP row is read as one line by screen readers.
    let ip_resp = ui.interact(
        ip.shrink2(vec2(60.0, 4.0)),
        ui.id().with("ip-row"),
        Sense::hover(),
    );
    ip_resp.widget_info(|| {
        WidgetInfo::labeled(WidgetType::Label, true, format!("{IP_LABEL}: {}", v.ip))
    });
    if v.copy {
        icons::copy(
            &painter,
            Rect::from_center_size(pos2(ip.right() - 28.0, ip.center().y), Vec2::splat(20.0)),
            token::WHITE,
        );
    }
    resp.clicked()
}

fn tile(ui: &mut Ui, t: &Tile, rect: Rect, icon: fn(&egui::Painter, Rect, Color32)) -> bool {
    let (sense, enabled) = match t.kind {
        TileKind::Status => (Sense::hover(), true),
        TileKind::Control { enabled, .. } => (
            if enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
            enabled,
        ),
    };
    let resp = ui.interact(rect, ui.id().with(("tile", t.label)), sense);
    let kind = if matches!(t.kind, TileKind::Status) {
        WidgetType::Label
    } else {
        WidgetType::Button
    };
    resp.widget_info(|| WidgetInfo::labeled(kind, enabled, t.accessible()));
    let painter = ui.painter_at(rect);
    let box_rect = Rect::from_min_size(
        pos2(rect.left() + 20.0, rect.top() + 20.0),
        Vec2::splat(64.0),
    );
    painter.rect_filled(box_rect, 14.0, token::NAVY);
    painter.rect_stroke(
        box_rect,
        14.0,
        Stroke::new(1.0, translucent(token::BORDER, 220)),
        StrokeKind::Inside,
    );
    icon(&painter, box_rect.shrink(16.0), token::BLUE);
    let left = box_rect.right() + 18.0;
    let room = (rect.right() - 16.0 - left).max(60.0);
    painter.text(
        pos2(left, rect.top() + 30.0),
        Align2::LEFT_CENTER,
        t.label,
        semibold(16.0),
        token::WHITE,
    );
    painter.text(
        pos2(left, rect.top() + 54.0),
        Align2::LEFT_CENTER,
        t.value,
        semibold(16.0),
        theme::tone(t.tone),
    );
    let text = painter.layout(
        t.text.clone(),
        egui::FontId::proportional(13.5),
        token::MUTED,
        room,
    );
    let text_bottom = rect.top() + 68.0 + text.size().y;
    // The supporting sentence is announced too (a refusal's reason, the
    // note after allowing): status changes must be accessible (§14.1).
    let text_rect = Rect::from_min_size(pos2(left, rect.top() + 68.0), text.size());
    painter.galley(text_rect.min, text, token::MUTED);
    ui.interact(
        text_rect,
        ui.id().with(("tile-text", t.label)),
        Sense::hover(),
    )
    .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &t.text));
    if let TileKind::Control {
        enabled: false,
        reason: Some(why),
    } = t.kind
    {
        let note = painter.layout(
            why.to_string(),
            egui::FontId::proportional(12.0),
            translucent(token::MUTED, 200),
            room,
        );
        painter.galley(pos2(left, text_bottom + 3.0), note, token::MUTED);
    }
    focus_ring(ui, &resp, rect, 12.0);
    enabled && resp.clicked() && matches!(t.kind, TileKind::Control { .. })
}

fn shield_check(painter: &egui::Painter, r: Rect, c: Color32) {
    icons::shield(painter, r, c, true);
}

type IconFn = fn(&egui::Painter, Rect, Color32);
type Galley = std::sync::Arc<egui::Galley>;

/// The left column's text, laid out once so the panel can be sized to it.
struct Column {
    size: f32,
    headline: Galley,
    lines: Vec<(String, Galley)>,
    notice: Option<(String, Galley)>,
    reason: Option<(&'static str, Galley)>,
}

const CARD_H: f32 = 124.0;

impl Column {
    fn layout(ui: &Ui, v: &HomeView, col_w: f32) -> Column {
        // The headline stays on one line where it can: 46 px, or smaller for
        // long headings (ADVANCED-UI §3.3), never below 28 px.
        let mut size: f32 = 46.0;
        let natural = ui
            .painter()
            .layout_no_wrap(v.headline.to_string(), bold(size), token::WHITE)
            .size()
            .x;
        if natural > col_w {
            size = (size * col_w / natural).floor().max(28.0);
        }
        let text = |t: &str, px: f32, c: Color32| {
            ui.painter()
                .layout(t.to_string(), egui::FontId::proportional(px), c, col_w)
        };
        Column {
            size,
            headline: ui
                .painter()
                .layout(v.headline.to_string(), bold(size), token::WHITE, col_w),
            lines: v
                .main
                .lines
                .iter()
                .take(2)
                .map(|l| (l.clone(), text(l, 16.5, token::WHITE)))
                .collect(),
            notice: v.main.notice.clone().map(|n| {
                let g = text(&n, 14.0, token::RED);
                (n, g)
            }),
            reason: v
                .main
                .button
                .reason
                .filter(|_| !v.main.button.enabled)
                .map(|r| (r, text(r, 13.5, token::MUTED))),
        }
    }

    /// Height of everything in the column, with the panel's padding.
    fn height(&self) -> f32 {
        28.0 + self.headline.size().y
            + 8.0
            + self
                .lines
                .iter()
                .map(|(_, g)| g.size().y + 2.0)
                .sum::<f32>()
            + 30.0
            + self.notice.as_ref().map_or(0.0, |(_, g)| g.size().y + 6.0)
            + 6.0
            + 62.0
            + self.reason.as_ref().map_or(0.0, |(_, g)| g.size().y + 6.0)
            + 14.0
            + CARD_H
            + 26.0
    }
}

/// Text that screen readers can read, at `at`; returns its height.
fn spoken(ui: &Ui, id: &str, at: egui::Pos2, g: Galley, colour: Color32, label: &str) -> f32 {
    let r = Rect::from_min_size(at, g.size());
    ui.painter().galley(r.min, g, colour);
    ui.interact(r, ui.id().with(("spoken", id)), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, label));
    r.height()
}

/// Headline, text, status row, notice, main button, reason and cards.
#[allow(clippy::many_single_char_names)] // short geometry names, as in egui
fn draw_column(
    ui: &mut Ui,
    v: &HomeView,
    c: Column,
    at: egui::Pos2,
    col_w: f32,
    actions: &mut Vec<Action>,
    page: &mut Page,
) {
    let left = at.x;
    let mut y = at.y;
    if v.headline == "You’re Connected" {
        // "Connected" in electric blue, as the approved image.
        let first = ui
            .painter()
            .layout_no_wrap("You’re ".into(), bold(c.size), token::WHITE);
        let w = first.size().x;
        ui.painter().galley(pos2(left, y), first, token::WHITE);
        let second = ui
            .painter()
            .layout_no_wrap("Connected".into(), bold(c.size), token::ELECTRIC);
        ui.painter()
            .galley(pos2(left + w, y), second, token::ELECTRIC);
        let r = Rect::from_min_size(pos2(left, y), c.headline.size());
        ui.interact(r, ui.id().with("headline"), Sense::hover())
            .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, v.headline));
        y += r.height();
    } else {
        y += spoken(
            ui,
            "headline",
            pos2(left, y),
            c.headline,
            token::WHITE,
            v.headline,
        );
    }
    y += 8.0;
    for (line, g) in c.lines {
        y += spoken(ui, &line, pos2(left, y), g, token::WHITE, &line) + 2.0;
    }
    // Status row: dot and words, never colour alone.
    let status =
        ui.painter()
            .layout_no_wrap(v.main.status.text.clone(), medium(15.0), token::WHITE);
    ui.painter().circle_filled(
        pos2(left + 5.0, y + 12.0),
        5.0,
        theme::tone(v.main.status.tone),
    );
    spoken(
        ui,
        "status-row",
        pos2(left + 18.0, y + 3.0),
        status,
        token::WHITE,
        &v.main.status.text,
    );
    y += 30.0;
    if let Some((n, g)) = c.notice {
        y += spoken(ui, "notice", pos2(left, y), g, token::RED, &n) + 6.0;
    }
    y += 6.0;
    let w = col_w.min(360.0);
    if main_button(ui, v, Rect::from_min_size(pos2(left, y), vec2(w, 62.0))) {
        actions.push(Action::Main(v.main.button.action));
    }
    y += 62.0;
    if let Some((reason, g)) = c.reason {
        y += spoken(ui, "reason", pos2(left, y + 6.0), g, token::MUTED, reason) + 6.0;
    }
    y += 14.0;
    if cards(ui, v, Rect::from_min_size(pos2(left, y), vec2(w, CARD_H))) {
        *page = Page::Connections;
    }
}

/// The Kill Switch, DNS Protection and Local Network Access tiles: in one
/// row, or stacked. Clicking Local Network Access (the only control) asks
/// the service for the other value.
#[allow(clippy::too_many_arguments)]
fn draw_tiles(
    ui: &mut Ui,
    v: &HomeView,
    left: f32,
    top: f32,
    width: f32,
    across: bool,
    tile_h: f32,
    actions: &mut Vec<Action>,
) {
    // Only a control tile reports a click, and Local Network Access is the
    // only control.
    let mut clicked = |hit: bool| {
        if hit {
            actions.push(Action::SetLocalNetwork(v.local_network.value != "Allowed"));
        }
    };
    let specs: [(&Tile, IconFn); 3] = [
        (&v.kill_switch, shield_check),
        (&v.dns, icons::servers),
        (&v.local_network, icons::house),
    ];
    if across {
        let row = Rect::from_min_size(pos2(left, top), vec2(width, tile_h));
        panel(ui, row);
        let w = width / 3.0;
        for (i, (t, icon)) in specs.into_iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let r = Rect::from_min_size(pos2(left + w * i as f32, top), vec2(w, tile_h));
            if i > 0 {
                ui.painter().line_segment(
                    [
                        pos2(r.left(), r.top() + 18.0),
                        pos2(r.left(), r.bottom() - 18.0),
                    ],
                    Stroke::new(1.0, translucent(token::BORDER, 160)),
                );
            }
            let hit = tile(ui, t, r, icon);
            clicked(hit);
        }
    } else {
        for (i, (t, icon)) in specs.into_iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let r = Rect::from_min_size(
                pos2(left, top + (tile_h + 10.0) * i as f32),
                vec2(width, tile_h),
            );
            panel(ui, r);
            let hit = tile(ui, t, r, icon);
            clicked(hit);
        }
    }
}

/// Draws the Home page into `width` × `height`.
fn home(
    ui: &mut Ui,
    v: &HomeView,
    width: f32,
    height: f32,
    actions: &mut Vec<Action>,
    page: &mut Page,
) {
    // Three across from 900 px; narrower windows stack them so every label
    // stays readable (ADVANCED-UI §4.5).
    let across = width >= ACROSS_FROM;
    let tile_h = if across { 136.0 } else { 116.0 };
    let row_total = if across {
        tile_h
    } else {
        3.0 * tile_h + 2.0 * 10.0
    };
    let gap = 18.0;
    let hero_w = width - 2.0 * MARGIN;
    let col_w = (hero_w * 0.5).clamp(330.0, 480.0);
    let column = Column::layout(ui, v, col_w);
    // The panel always holds the whole column.
    let panel_h = (height - row_total - gap - 12.0).max(column.height());
    let (area, _) = ui.allocate_exact_size(vec2(width, panel_h + gap + row_total), Sense::hover());
    let hero = Rect::from_min_size(
        pos2(area.left() + MARGIN, area.top()),
        vec2(hero_w, panel_h),
    );
    panel(ui, hero);
    let left = hero.left() + 36.0;
    draw_column(
        ui,
        v,
        column,
        pos2(left, hero.top() + 28.0),
        col_w,
        actions,
        page,
    );

    // Right side: the hero placeholder (decoration; the status is in words).
    let art_left = (left + col_w.min(360.0) + 24.0).max(hero.center().x);
    let art = Rect::from_min_max(
        pos2(art_left, hero.top() + 16.0),
        pos2(hero.right() - 16.0, hero.bottom() - 16.0),
    );
    if art.width() > 120.0 {
        let s = art.width().min(art.height()).min(320.0);
        paint_shield(
            &ui.painter_at(art),
            Rect::from_center_size(art.center(), Vec2::splat(s)),
            v.main.shield,
        );
    }
    draw_tiles(
        ui,
        v,
        hero.left(),
        hero.bottom() + gap,
        hero.width(),
        across,
        tile_h,
        actions,
    );
}

/// Draws the Advanced main window and returns what the user did. `page`
/// is the selected tab.
pub fn draw(
    ui: &mut Ui,
    v: &HomeView,
    tex: &Textures,
    page: &mut Page,
    mode: InterfaceMode,
    conns: &mut crate::connections::Connections,
    session: crate::connections::Session,
) -> Vec<Action> {
    let mut actions = Vec::new();
    let full = ui.available_width();
    let width = full.min(MAX_CONTENT);
    let height = ui.available_height();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = 0.0;
                header(ui, tex, width, page);
                ui.add_space(8.0);
                match *page {
                    Page::Home => home(ui, v, width, height - 88.0, &mut actions, page),
                    other => {
                        let shell = match other {
                            Page::Connections => Shell::Connections,
                            Page::Settings => Shell::Settings,
                            _ => Shell::HelpDiagnostics,
                        };
                        let (area, _) = ui.allocate_exact_size(
                            vec2(width, (height - 100.0).max(380.0)),
                            Sense::hover(),
                        );
                        let inner = Rect::from_min_max(
                            pos2(area.left() + MARGIN, area.top()),
                            pos2(area.right() - MARGIN, area.bottom()),
                        );
                        panel(ui, inner);
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(inner.shrink(28.0))
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        child.style_mut().spacing.item_spacing.y = 6.0;
                        if other == Page::Diagnostics {
                            // The tab is named Diagnostics (ADVANCED-UI §5.2).
                            child.label(
                                RichText::new("Diagnostics")
                                    .font(semibold(24.0))
                                    .color(token::WHITE),
                            );
                            child.add_space(16.0);
                            child.label(RichText::new(shells::NOT_YET).color(token::WHITE));
                            child.label(
                                RichText::new("It arrives with development task N4-08.")
                                    .color(token::MUTED),
                            );
                        } else if let Some(a) =
                            shells::page(&mut child, shell, mode, conns, session)
                        {
                            actions.push(a);
                        }
                    }
                }
                ui.add_space(16.0);
            });
        });
    actions
}
