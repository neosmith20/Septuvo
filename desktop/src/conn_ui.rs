//! Drawing the saved-connection areas (N4-05): the profile selector's list,
//! the **Connections** manager, **Add / Import** with the import preview,
//! and the **VPN Configuration** editor (DEFAULT-UI §3.1–§3.3, §4).
//!
//! Layout only: every decision is in [`Connections`]. The same content is
//! a Simple-mode window or an Advanced-mode page. Every control has a
//! visible label and an accessible name; disabled controls say why.

use eframe::egui::{
    self, Align2, Id, Key as KeyCode, Rect, RichText, Sense, Stroke, StrokeKind, TextEdit, Ui,
    Vec2, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::connections::{Connections, EditorMode, ImportStep, Notice, Session};
use crate::form::{Checked, Field};
use crate::icons;
use crate::library::{Entry, Key};
use crate::main_window::{focus_ring, translucent};
use crate::theme::{medium, semibold, token};

/// Selector rows (DEFAULT-UI §3.1: at least 44 px; 6 rows, then scroll).
pub const SELECTOR_ROW: f32 = 44.0;
/// Connections manager rows (DEFAULT-UI §3.2).
pub const MANAGER_ROW: f32 = 72.0;

/// A text button, at least 44 px high. `danger` uses the error colour.
fn button(ui: &mut Ui, label: &str, primary: bool, danger: bool, enabled: bool) -> bool {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), medium(15.0), token::WHITE);
    let size = vec2((galley.size().x + 36.0).max(96.0), 44.0);
    let (rect, resp) = ui.allocate_exact_size(
        size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    let fill = match (enabled, primary, danger) {
        (false, _, _) => token::NAVY,
        (true, _, true) => token::RED,
        (true, true, false) => token::BLUE,
        (true, false, false) if resp.hovered() => token::BORDER,
        (true, false, false) => token::SURFACE,
    };
    ui.painter().rect_filled(rect, 10.0, fill);
    ui.painter().rect_stroke(
        rect,
        10.0,
        Stroke::new(1.0, translucent(token::BORDER, 220)),
        StrokeKind::Inside,
    );
    let colour = if enabled { token::WHITE } else { token::MUTED };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        medium(15.0),
        colour,
    );
    focus_ring(ui, &resp, rect, 10.0);
    enabled && resp.clicked()
}

fn notice(ui: &mut Ui, n: Option<&Notice>) {
    if let Some(n) = n {
        let colour = if n.error { token::RED } else { token::GREEN };
        ui.label(RichText::new(&n.text).size(14.0).color(colour));
        ui.add_space(8.0);
    }
}

fn error_text(ui: &mut Ui, e: Option<&str>) {
    if let Some(e) = e {
        ui.label(RichText::new(e).size(13.0).color(token::RED));
    }
}

/// A labelled single-line field, at least 40 px high.
fn field(ui: &mut Ui, label: &str, text: &mut String, hint: &str, help: Option<&str>) {
    let l = ui.label(RichText::new(label).font(medium(15.0)).color(token::WHITE));
    ui.add(
        TextEdit::singleline(text)
            .hint_text(hint)
            .char_limit(512)
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, 40.0))
            .vertical_align(egui::Align::Center),
    )
    .labelled_by(l.id);
    if let Some(h) = help {
        ui.label(RichText::new(h).size(12.5).color(token::MUTED));
    }
}

/// A labelled multi-line list field.
fn list_field(ui: &mut Ui, label: &str, text: &mut String, hint: &str, help: &str) {
    let l = ui.label(RichText::new(label).font(medium(15.0)).color(token::WHITE));
    ui.add(
        TextEdit::multiline(text)
            .hint_text(hint)
            .desired_rows(2)
            .desired_width(f32::INFINITY),
    )
    .labelled_by(l.id);
    ui.label(RichText::new(help).size(12.5).color(token::MUTED));
}

// --- the profile selector's list (DEFAULT-UI §3.1) ---------------------------

/// The open selector list under `anchor`: every connection and draft, one
/// row each, the selection marked in words and with a check. Returns the
/// chosen connection, or `Some(None)` when it should close unchanged.
#[allow(clippy::option_option)]
#[must_use]
pub fn selector_list(ui: &Ui, anchor: Rect, entries: &[Entry]) -> Option<Option<Key>> {
    let ctx = ui.ctx().clone();
    let mut out = None;
    let rows = entries.len().min(6);
    #[allow(clippy::cast_precision_loss)]
    let height = rows as f32 * SELECTOR_ROW + 8.0;
    let area = egui::Area::new(Id::new("septuvo-selector-list"))
        .order(egui::Order::Foreground)
        .fade_in(false)
        .fixed_pos(pos2(anchor.left(), anchor.bottom() + 6.0))
        .show(&ctx, |ui| {
            let frame = egui::Frame::new()
                .fill(token::NAVY)
                .stroke(Stroke::new(1.0, translucent(token::BLUE, 160)))
                .corner_radius(12.0)
                .inner_margin(4.0);
            frame.show(ui, |ui| {
                ui.set_width(anchor.width() - 8.0);
                egui::ScrollArea::vertical()
                    .max_height(height)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (i, e) in entries.iter().enumerate() {
                            if selector_row(ui, i, e, anchor.width() - 8.0) {
                                out = Some(Some(e.key));
                            }
                        }
                    });
            });
        });
    let (escape, clicked_outside) = ctx.input(|i| {
        (
            i.key_pressed(KeyCode::Escape),
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !area.response.rect.contains(p) && !anchor.contains(p)),
        )
    });
    if out.is_none() && (escape || clicked_outside) {
        out = Some(None);
    }
    out
}

fn selector_row(ui: &mut Ui, i: usize, e: &Entry, width: f32) -> bool {
    let id = Id::new(("septuvo-selector-row", i));
    let (rect, _) = ui.allocate_exact_size(vec2(width, SELECTOR_ROW), Sense::hover());
    let resp = ui.interact(rect, id, Sense::click());
    let draft = matches!(e.key, Key::Draft(_));
    let label = match (e.selected, draft) {
        (true, true) => format!("{}, Setup Required, selected", e.name),
        (true, false) => format!("{}, selected", e.name),
        (false, true) => format!("{}, Setup Required", e.name),
        (false, false) => e.name.clone(),
    };
    resp.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, e.selected, &label));
    // Up and Down move between rows (DEFAULT-UI §9.1).
    if resp.has_focus() {
        let (down, up) = ui.input(|inp| {
            (
                inp.key_pressed(KeyCode::ArrowDown),
                inp.key_pressed(KeyCode::ArrowUp),
            )
        });
        if down {
            ui.memory_mut(|m| m.request_focus(Id::new(("septuvo-selector-row", i + 1))));
        } else if up && i > 0 {
            ui.memory_mut(|m| m.request_focus(Id::new(("septuvo-selector-row", i - 1))));
        }
    }
    if resp.hovered() || resp.has_focus() {
        ui.painter()
            .rect_filled(rect.shrink(2.0), 8.0, token::SURFACE);
    }
    let painter = ui.painter_at(rect);
    if e.selected {
        icons::shield(
            &painter,
            Rect::from_center_size(pos2(rect.left() + 22.0, rect.center().y), Vec2::splat(18.0)),
            token::ELECTRIC,
            true,
        );
    }
    painter.text(
        pos2(rect.left() + 44.0, rect.center().y),
        Align2::LEFT_CENTER,
        &e.name,
        medium(16.0),
        token::WHITE,
    );
    if draft {
        painter.text(
            pos2(rect.right() - 14.0, rect.center().y),
            Align2::RIGHT_CENTER,
            "Setup Required",
            medium(13.0),
            token::YELLOW,
        );
    }
    focus_ring(ui, &resp, rect.shrink(2.0), 8.0);
    resp.clicked()
}

// --- the Connections manager (DEFAULT-UI §3.2) -------------------------------

/// The five actions, in the approved order.
const MENU: [&str; 5] = ["Edit", "Rename", "Export", "Duplicate", "Delete"];

/// The Connections manager's list, menus and dialogs.
pub fn connections_page(ui: &mut Ui, c: &mut Connections, session: Session) {
    notice(ui, c.notice.as_ref());
    if !c.lib.loaded && c.lib.drafts.list().is_empty() {
        let text = c
            .lib
            .error
            .clone()
            .unwrap_or_else(|| "Loading saved connections…".into());
        ui.label(RichText::new(text).color(token::MUTED));
        return;
    }
    let entries = c.lib.entries(session.in_use);
    if entries.is_empty() {
        ui.label(
            RichText::new("No saved connections yet.")
                .font(semibold(17.0))
                .color(token::WHITE),
        );
        ui.label(
            RichText::new("Use Add / Import to add or import a VPN configuration.")
                .color(token::MUTED),
        );
        return;
    }
    let width = ui.available_width();
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            for e in &entries {
                manager_row(ui, c, e, width, session);
            }
        });
    rename_dialog(ui, c);
    delete_dialog(ui, c, session);
}

fn manager_row(ui: &mut Ui, c: &mut Connections, e: &Entry, width: f32, session: Session) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, MANAGER_ROW), Sense::hover());
    let row = ui.interact(rect, Id::new(("septuvo-conn-row", e.key)), Sense::click());
    let mut tags: Vec<&str> = Vec::new();
    if e.selected {
        tags.push("Selected");
    }
    if e.in_use {
        tags.push("In Use");
    }
    let second = [e.detail.as_str()]
        .into_iter()
        .chain(tags.iter().copied())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    // One click selects the connection, exactly as the profile selector
    // does (owner decision of 2026-10-25): while connected that switches.
    // The `⋯` button is a separate control on top, so it never selects.
    let highlighted = e.selected;
    row.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::RadioButton,
            true,
            highlighted,
            format!("{}, {second}", e.name),
        )
    });
    if row.clicked() {
        c.select(e.key, session);
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        10.0,
        if highlighted {
            token::SURFACE
        } else {
            token::NAVY
        },
    );
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, translucent(token::BORDER, 160)),
    );
    icons::servers(
        &painter,
        Rect::from_center_size(pos2(rect.left() + 32.0, rect.center().y), Vec2::splat(32.0)),
        token::BLUE,
    );
    painter.text(
        pos2(rect.left() + 64.0, rect.center().y - 11.0),
        Align2::LEFT_CENTER,
        &e.name,
        semibold(16.0),
        token::WHITE,
    );
    painter.text(
        pos2(rect.left() + 64.0, rect.center().y + 13.0),
        Align2::LEFT_CENTER,
        &second,
        medium(13.0),
        if matches!(e.key, Key::Draft(_)) {
            token::YELLOW
        } else {
            token::MUTED
        },
    );
    focus_ring(ui, &row, rect, 10.0);
    let dots = Rect::from_center_size(
        pos2(rect.right() - 30.0, rect.center().y),
        Vec2::splat(44.0),
    );
    let more = ui.interact(dots, Id::new(("septuvo-conn-more", e.key)), Sense::click());
    let more_label = format!("More actions for {}", e.name);
    more.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &more_label));
    if more.hovered() || c.menu == Some(e.key) {
        ui.painter()
            .rect_filled(dots.shrink(4.0), 8.0, token::BORDER);
    }
    for dx in [-8.0, 0.0, 8.0] {
        ui.painter().circle_filled(
            pos2(dots.center().x + dx, dots.center().y),
            2.4,
            token::WHITE,
        );
    }
    focus_ring(ui, &more, dots.shrink(4.0), 8.0);
    if more.clicked() {
        c.menu = if c.menu == Some(e.key) {
            None
        } else {
            Some(e.key)
        };
    }
    if c.menu == Some(e.key) {
        overflow_menu(ui, c, e.key, dots);
    }
}

fn overflow_menu(ui: &Ui, c: &mut Connections, key: Key, anchor: Rect) {
    let ctx = ui.ctx().clone();
    let draft = matches!(key, Key::Draft(_));
    let mut chosen = None;
    let area = egui::Area::new(Id::new(("septuvo-conn-menu", key)))
        .order(egui::Order::Foreground)
        .fade_in(false)
        .fixed_pos(pos2(anchor.right() - 200.0, anchor.bottom() + 2.0))
        .show(&ctx, |ui| {
            egui::Frame::new()
                .fill(token::NAVY)
                .stroke(Stroke::new(1.0, translucent(token::BORDER, 220)))
                .corner_radius(10.0)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ui.set_width(192.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (i, item) in MENU.iter().enumerate() {
                        if i == 4 {
                            ui.separator();
                        }
                        // A draft is not a configuration yet: it cannot be
                        // exported (the reason is in its accessible name).
                        let enabled = !(draft && *item == "Export");
                        if menu_item(ui, item, i == 4, enabled) {
                            chosen = Some(i);
                        }
                    }
                });
        });
    let (escape, outside) = ctx.input(|i| {
        (
            i.key_pressed(KeyCode::Escape),
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| !area.response.rect.contains(p) && !anchor.contains(p)),
        )
    });
    match chosen {
        Some(0) => c.edit(key),
        Some(1) => c.rename(key),
        Some(2) => c.export(key),
        Some(3) => c.duplicate(key),
        Some(_) => c.delete(key),
        None if escape || outside => c.menu = None,
        None => {}
    }
}

fn menu_item(ui: &mut Ui, label: &str, danger: bool, enabled: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(
        vec2(192.0, 42.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let spoken = if enabled {
        label.to_string()
    } else {
        format!("{label} (complete the draft first)")
    };
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &spoken));
    if enabled && (resp.hovered() || resp.has_focus()) {
        ui.painter().rect_filled(rect, 8.0, token::SURFACE);
    }
    let colour = match (enabled, danger) {
        (false, _) => token::MUTED,
        (true, true) => token::RED,
        (true, false) => token::WHITE,
    };
    ui.painter().text(
        pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        medium(15.0),
        colour,
    );
    focus_ring(ui, &resp, rect, 8.0);
    enabled && resp.clicked()
}

fn rename_dialog(ui: &Ui, c: &mut Connections) {
    let Some(r) = &mut c.rename else { return };
    let mut submit = false;
    let mut cancel = false;
    let busy = r.busy;
    let modal = egui::Modal::new(Id::new("septuvo-rename")).show(ui.ctx(), |ui| {
        ui.set_width(380.0);
        ui.label(
            RichText::new("Rename Connection")
                .font(semibold(18.0))
                .color(token::WHITE),
        );
        ui.label(
            RichText::new("Only the name changes. The connection is not affected.")
                .size(13.0)
                .color(token::MUTED),
        );
        ui.add_space(8.0);
        let l = ui.label(RichText::new("Connection Name").font(medium(15.0)));
        let resp = ui
            .add(
                TextEdit::singleline(&mut r.text)
                    .char_limit(64)
                    .hint_text("New name")
                    .desired_width(f32::INFINITY)
                    .min_size(vec2(0.0, 40.0)),
            )
            .labelled_by(l.id);
        if resp.lost_focus() && ui.input(|i| i.key_pressed(KeyCode::Enter)) {
            submit = true;
        }
        error_text(ui, r.error.as_deref());
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            cancel = button(ui, "Cancel", false, false, !busy);
            submit |= button(ui, "Save", true, false, !busy);
        });
    });
    if cancel || (modal.should_close() && !busy) {
        c.rename = None;
    } else if submit {
        c.rename_submit();
    }
}

fn delete_dialog(ui: &Ui, c: &mut Connections, session: Session) {
    let Some(d) = &c.delete else { return };
    let busy = d.busy;
    let in_use = matches!(d.key, Key::Saved(id) if session.in_use == Some(id));
    let mut confirm = false;
    let mut cancel = false;
    let modal = egui::Modal::new(Id::new("septuvo-delete")).show(ui.ctx(), |ui| {
        ui.set_width(400.0);
        ui.label(
            RichText::new(format!("Delete “{}”?", d.name))
                .font(semibold(18.0))
                .color(token::WHITE),
        );
        let mut text = String::from(
            "The saved connection is removed from this device. This cannot be undone. \
             It does not revoke access on the server.",
        );
        if in_use {
            text.push_str(" It is in use: Septuvo disconnects it safely first.");
        }
        ui.label(RichText::new(text).size(14.0).color(token::MUTED));
        error_text(ui, d.error.as_deref());
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            cancel = button(ui, "Cancel", false, false, !busy);
            confirm = button(ui, "Delete Connection", false, true, !busy);
        });
    });
    if cancel || (modal.should_close() && !busy) {
        c.delete = None;
    } else if confirm {
        c.delete_confirm(session);
    }
}

/// The Advanced Connections page's **Add / Import** button, in `rect`.
/// Returns true when activated.
pub fn add_import_button(ui: &mut Ui, rect: Rect) -> bool {
    let resp = ui.interact(rect, Id::new("septuvo-advanced-add-import"), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, "Add / Import"));
    ui.painter().rect_filled(
        rect,
        10.0,
        if resp.hovered() {
            token::SKY
        } else {
            token::BLUE
        },
    );
    icons::circle_plus(
        &ui.painter_at(rect),
        Rect::from_center_size(pos2(rect.left() + 24.0, rect.center().y), Vec2::splat(20.0)),
        token::WHITE,
    );
    ui.painter().text(
        pos2(rect.left() + 42.0, rect.center().y),
        Align2::LEFT_CENTER,
        "Add / Import",
        medium(15.0),
        token::WHITE,
    );
    focus_ring(ui, &resp, rect, 10.0);
    resp.clicked()
}

// --- Add / Import (DEFAULT-UI §3.3, §4.3) ------------------------------------

/// The VPN Configuration choice dialog and the import preview.
pub fn add_import_page(ui: &mut Ui, c: &mut Connections) {
    notice(ui, c.notice.as_ref());
    match &mut c.import {
        ImportStep::Choice => {
            ui.spacing_mut().item_spacing.y = 12.0;
            if wide_button(ui, "Create New Configuration") {
                c.notice = None;
                c.create_new();
            }
            if wide_button(ui, "Import Configuration") {
                c.import();
            }
        }
        ImportStep::Choosing => {
            ui.label(RichText::new("Choose the file in the file chooser…").color(token::MUTED));
        }
        ImportStep::Preview(_) => preview_page(ui, c),
    }
}

fn wide_button(ui: &mut Ui, label: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    ui.painter().rect_filled(
        rect,
        12.0,
        if resp.hovered() {
            token::BORDER
        } else {
            token::SURFACE
        },
    );
    ui.painter().text(
        pos2(rect.left() + 18.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        semibold(16.0),
        token::WHITE,
    );
    icons::chevron_right(
        &ui.painter_at(rect),
        Rect::from_center_size(
            pos2(rect.right() - 24.0, rect.center().y),
            Vec2::splat(18.0),
        ),
        token::MUTED,
    );
    focus_ring(ui, &resp, rect, 12.0);
    resp.clicked()
}

fn preview_page(ui: &mut Ui, c: &mut Connections) {
    let ImportStep::Preview(p) = &mut c.import else {
        return;
    };
    let busy = p.busy;
    ui.label(
        RichText::new("Import Preview")
            .font(semibold(18.0))
            .color(token::WHITE),
    );
    ui.label(RichText::new(format!("File: {}", p.file_name)).color(token::MUTED));
    ui.add_space(6.0);
    let l = ui.label(RichText::new("Connection Name").font(medium(15.0)));
    ui.add(
        TextEdit::singleline(&mut p.name)
            .char_limit(64)
            .hint_text("Home VPN")
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, 40.0)),
    )
    .labelled_by(l.id);
    error_text(ui, p.error.as_deref());
    let mode = if p.form.full_tunnel {
        "Full Tunnel"
    } else {
        "Split Tunnel"
    };
    for (k, v) in [
        ("Server Address", p.form.server.clone()),
        ("Routing Mode", mode.to_string()),
        ("DNS", p.form.dns_summary()),
    ] {
        ui.label(RichText::new(format!("{k}: {v}")).color(token::WHITE));
    }
    match &p.incomplete {
        None => ui
            .label(RichText::new("Complete: it can be selected after import.").color(token::GREEN)),
        Some(why) => {
            ui.label(RichText::new(format!("Draft · Setup Required: {why}")).color(token::YELLOW))
        }
    };
    ui.label(
        RichText::new(
            "Importing a file does not prove that a server is trusted, and does not connect.",
        )
        .size(13.0)
        .color(token::MUTED),
    );
    ui.add_space(10.0);
    let mut cancel = false;
    let mut confirm = false;
    ui.horizontal(|ui| {
        cancel = button(ui, "Cancel", false, false, !busy);
        confirm = button(ui, "Import Configuration", true, false, !busy);
    });
    if cancel {
        c.import = ImportStep::Choice;
    } else if confirm {
        c.import_confirm();
    }
}

// --- the editor (DEFAULT-UI §4.2) --------------------------------------------

/// The configuration editor's content.
pub fn editor_page(ui: &mut Ui, c: &mut Connections) {
    let mut checked = c.check();
    // Errors appear once something was typed or saving was tried, not on
    // a blank form (DEFAULT-UI §4.1: next to the field, in plain words).
    if c.editor
        .as_ref()
        .is_some_and(|e| !e.attempted && !e.changed())
    {
        checked.errors.clear();
    }
    let Some(e) = &mut c.editor else { return };
    ui.label(RichText::new(e.mode.subtitle()).color(token::MUTED));
    ui.add_space(8.0);
    if e.loading {
        ui.label(RichText::new("Loading the saved configuration…").color(token::MUTED));
        return;
    }
    let escape = ui.input(|i| i.key_pressed(KeyCode::Escape));
    egui::ScrollArea::vertical()
        .max_height((ui.available_height() - 80.0).max(120.0))
        .show(ui, |ui| editor_fields(ui, e, &checked));
    if let Some(err) = &e.error {
        ui.label(RichText::new(err).size(13.0).color(token::RED));
    }
    let busy = e.busy;
    let saved = matches!(e.mode, EditorMode::Edit(_));
    let mut cancel = escape && !e.confirm_discard;
    let (mut save, mut draft) = (false, false);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        cancel |= button(ui, "Cancel", false, false, !busy);
        if !saved {
            draft = button(ui, "Save Draft", false, false, !busy && checked.name_ok);
        }
        save = button(
            ui,
            "Save Configuration",
            true,
            false,
            !busy && checked.complete,
        );
    });
    if !checked.complete && !busy {
        ui.label(
            RichText::new("Save Configuration needs a complete configuration.")
                .size(12.5)
                .color(token::MUTED),
        );
    }
    let confirm = e.confirm_discard;
    if cancel {
        c.editor_cancel();
    } else if draft {
        c.save_draft();
    } else if save {
        c.save_configuration();
    }
    if confirm {
        discard_dialog(ui, c);
    }
}

fn editor_fields(ui: &mut Ui, e: &mut crate::connections::Editor, checked: &Checked) {
    ui.spacing_mut().item_spacing.y = 6.0;
    field(ui, "Connection Name", &mut e.form.name, "Home VPN", None);
    error_text(ui, checked.error(Field::Name));
    ui.add_space(6.0);
    field(
        ui,
        "Server Address",
        &mut e.form.server,
        "203.0.113.10:51820",
        Some("Enter the server address and port provided to you."),
    );
    error_text(ui, checked.error(Field::Server));
    ui.add_space(10.0);
    let label = format!(
        "Advanced Configuration, {}",
        if e.advanced { "expanded" } else { "collapsed" }
    );
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
    ui.painter().rect_filled(rect, 10.0, token::NAVY);
    ui.painter().text(
        pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        "Advanced Configuration",
        semibold(15.0),
        token::WHITE,
    );
    let chevron = Rect::from_center_size(
        pos2(rect.right() - 22.0, rect.center().y),
        Vec2::splat(16.0),
    );
    if e.advanced {
        icons::chevron_down(&ui.painter_at(rect), chevron, token::MUTED);
    } else {
        icons::chevron_right(&ui.painter_at(rect), chevron, token::MUTED);
    }
    focus_ring(ui, &resp, rect, 10.0);
    if resp.clicked() {
        e.advanced = !e.advanced;
    }
    if e.advanced {
        advanced_fields(ui, e, checked);
    } else if checked.errors.iter().any(|(f, _)| f.advanced()) {
        ui.label(
            RichText::new("Some settings in Advanced Configuration need attention.")
                .size(13.0)
                .color(token::YELLOW),
        );
    }
}

fn advanced_fields(ui: &mut Ui, e: &mut crate::connections::Editor, checked: &Checked) {
    ui.add_space(6.0);
    ui.label(
        RichText::new("Routing Mode")
            .font(medium(15.0))
            .color(token::WHITE),
    );
    ui.horizontal(|ui| {
        for (label, full) in [("Full Tunnel", true), ("Split Tunnel", false)] {
            let selected = e.form.full_tunnel == full;
            if ui.add(egui::RadioButton::new(selected, label)).clicked() {
                e.form.full_tunnel = full;
            }
        }
    });
    ui.label(
        RichText::new(if e.form.full_tunnel {
            "Full Tunnel: all supported traffic goes through the VPN."
        } else {
            "Split Tunnel: only the networks listed below go through the VPN. Other traffic is not protected."
        })
        .size(12.5)
        .color(token::MUTED),
    );
    error_text(ui, checked.error(Field::Mode));
    for (label, field_kind, text, hint, help) in [
        (
            "Networks Through VPN",
            Field::Tunnel,
            &mut e.form.tunnel,
            "10.0.0.0/8",
            "Networks reached through the VPN, one per line, for example 10.0.0.0/8.",
        ),
        (
            "Direct Network Access",
            Field::Direct,
            &mut e.form.direct,
            "192.168.1.30/32",
            "Traffic to these networks bypasses the VPN. Keep this list short.",
        ),
        (
            "Blocked Networks",
            Field::Block,
            &mut e.form.block,
            "203.0.113.0/24",
            "Traffic to these networks is always blocked.",
        ),
    ] {
        ui.add_space(6.0);
        list_field(ui, label, text, hint, help);
        error_text(ui, checked.error(field_kind));
    }
    ui.add_space(6.0);
    ui.label(RichText::new("DNS").font(medium(15.0)).color(token::WHITE));
    ui.label(
        RichText::new(e.form.dns_summary())
            .size(13.0)
            .color(token::MUTED),
    );
    error_text(ui, checked.error(Field::Dns));
    for (label, kind, text, hint, help) in [
        (
            "Interface Name",
            Field::Interface,
            &mut e.form.interface,
            "septuvo0",
            "1 to 15 characters. Leave empty for septuvo0.",
        ),
        (
            "MTU",
            Field::Mtu,
            &mut e.form.mtu,
            "1420",
            "1280 to 9000. Leave empty for 1420.",
        ),
        (
            "Log Retention (Days)",
            Field::Retention,
            &mut e.form.retention,
            "7",
            "0 to 365 days. Leave empty for 7. Logs never contain keys or secrets.",
        ),
    ] {
        ui.add_space(6.0);
        field(ui, label, text, hint, Some(help));
        error_text(ui, checked.error(kind));
    }
    error_text(ui, checked.error(Field::Other));
}

fn discard_dialog(ui: &Ui, c: &mut Connections) {
    let mut keep = false;
    let mut discard = false;
    let modal = egui::Modal::new(Id::new("septuvo-discard")).show(ui.ctx(), |ui| {
        ui.set_width(360.0);
        ui.label(
            RichText::new("Discard changes?")
                .font(semibold(18.0))
                .color(token::WHITE),
        );
        ui.label(
            RichText::new("Your changes to this configuration have not been saved.")
                .color(token::MUTED),
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            keep = button(ui, "Keep Editing", false, false, true);
            discard = button(ui, "Discard Changes", false, true, true);
        });
    });
    if discard {
        c.editor_discard();
    } else if (keep || modal.should_close())
        && let Some(e) = &mut c.editor
    {
        e.confirm_discard = false;
    }
}
