//! The secondary areas: Connections, VPN Configuration, Help & Diagnostics
//! and Settings.
//!
//! In Simple mode each opens as its own window at its specified size
//! (DEFAULT-UI §3); in Advanced mode Connections, Settings and Diagnostics
//! are pages of the main window, opened from its tabs (ADVANCED-UI §5, §8).
//! Both show the same content ([`page`]). Until later N4 tasks fill them
//! they say plainly that they are not available yet; no area pretends to
//! work. The one working setting is **Interface Mode** (N4-02), which never
//! touches the connection. Closing an area never touches the connection.

use eframe::egui::{
    self, Context, RichText, Sense, Ui, ViewportBuilder, ViewportClass, ViewportId, WidgetInfo,
    WidgetType, vec2,
};

use crate::conn_ui;
use crate::connections::{Connections, Session};
use crate::main_window::{Action, Shell, focus_ring, translucent};
use crate::prefs::InterfaceMode;
use crate::theme::{medium, semibold, token};

/// Title, subtitle, initial size, minimum size and which task fills it.
#[must_use]
pub fn spec(shell: Shell) -> (&'static str, &'static str, [f32; 2], [f32; 2], &'static str) {
    match shell {
        Shell::Connections => (
            "Connections",
            "Manage Your Saved Connections",
            [700.0, 540.0],
            [500.0, 380.0],
            "N4-05",
        ),
        Shell::AddImport => (
            "VPN Configuration",
            "Create a new connection or import one provided to you.",
            [560.0, 360.0],
            [480.0, 330.0],
            "N4-05",
        ),
        Shell::HelpDiagnostics => (
            "Help & Diagnostics",
            "Get Help or Run Tests",
            [740.0, 600.0],
            [540.0, 440.0],
            "N4-08",
        ),
        Shell::Settings => ("Settings", "", [700.0, 640.0], [520.0, 480.0], "N4-06"),
        Shell::Editor => (
            "VPN Configuration",
            "",
            [700.0, 620.0],
            [540.0, 450.0],
            "N4-05",
        ),
    }
}

/// The text shown in an area that is not built yet.
pub const NOT_YET: &str = "This window is not available yet in this development build.";

/// The two-choice Interface Mode selector (DEFAULT-UI §6.2).
fn mode_selector(ui: &mut Ui, mode: InterfaceMode) -> Option<InterfaceMode> {
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (label, value) in [
            ("Simple", InterfaceMode::Simple),
            ("Advanced", InterfaceMode::Advanced),
        ] {
            let selected = mode == value;
            let (rect, resp) = ui.allocate_exact_size(vec2(112.0, 40.0), Sense::click());
            resp.widget_info(|| {
                WidgetInfo::selected(
                    WidgetType::RadioButton,
                    true,
                    selected,
                    format!("Interface Mode: {label}"),
                )
            });
            let fill = if selected {
                token::BLUE
            } else if resp.hovered() {
                token::SURFACE
            } else {
                token::NAVY
            };
            let radius = 10.0;
            ui.painter().rect_filled(rect, radius, fill);
            ui.painter().rect_stroke(
                rect,
                radius,
                egui::Stroke::new(1.0, translucent(token::BORDER, 220)),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                medium(15.0),
                if selected { token::WHITE } else { token::MUTED },
            );
            focus_ring(ui, &resp, rect, radius);
            if resp.clicked() && !selected {
                chosen = Some(value);
            }
        }
    });
    chosen
}

/// The content of an area. Returns what the user did in Settings (its one
/// working control: Interface Mode); the connection areas act on `conns`.
pub fn page(
    ui: &mut Ui,
    shell: Shell,
    mode: InterfaceMode,
    conns: &mut Connections,
    session: Session,
) -> Option<Action> {
    let (title, subtitle, _, _, task) = spec(shell);
    let mut action = None;
    ui.label(
        RichText::new(title)
            .font(semibold(24.0))
            .color(token::WHITE),
    );
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).color(token::MUTED));
    }
    ui.add_space(16.0);
    match shell {
        Shell::Connections => {
            conn_ui::connections_page(ui, conns, session);
            return None;
        }
        Shell::AddImport => {
            conn_ui::add_import_page(ui, conns);
            return None;
        }
        Shell::Editor => {
            conn_ui::editor_page(ui, conns);
            return None;
        }
        Shell::Settings | Shell::HelpDiagnostics => {}
    }
    if shell == Shell::Settings {
        ui.label(
            RichText::new("Interface")
                .font(semibold(17.0))
                .color(token::WHITE),
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Interface Mode")
                        .font(medium(16.0))
                        .color(token::WHITE),
                );
                ui.label(
                    RichText::new("Simple or Advanced main window. Changing it never disconnects.")
                        .size(14.0)
                        .color(token::MUTED),
                );
            });
            ui.add_space(16.0);
            if let Some(m) = mode_selector(ui, mode) {
                action = Some(Action::SetMode(m));
            }
        });
        ui.add_space(20.0);
        ui.label(
            RichText::new(format!(
                "The other settings arrive with development task {task}."
            ))
            .color(token::MUTED),
        );
        ui.add_space(24.0);
        ui.separator();
        ui.label(
            RichText::new(format!(
                "Version {} · Development Preview",
                env!("CARGO_PKG_VERSION")
            ))
            .size(13.0)
            .color(token::MUTED),
        );
    } else {
        ui.label(RichText::new(NOT_YET).color(token::WHITE));
        ui.label(
            RichText::new(format!("It arrives with development task {task}.")).color(token::MUTED),
        );
    }
    action
}

/// Which Simple-mode windows are open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shells {
    open: Vec<Shell>,
}

impl Shells {
    /// Opens a window (or keeps it open).
    pub fn open(&mut self, s: Shell) {
        if !self.open.contains(&s) {
            self.open.push(s);
        }
    }

    /// True if the window is open.
    #[must_use]
    pub fn is_open(&self, s: Shell) -> bool {
        self.open.contains(&s)
    }

    /// Closes every window (switching to Advanced mode, where the areas are
    /// tabs).
    pub fn close_all(&mut self) {
        self.open.clear();
    }

    /// Closes one window.
    pub fn close(&mut self, s: Shell) {
        self.open.retain(|x| *x != s);
    }

    /// Shows every open window; closes those the user closed. Returns what
    /// the user did in them. The editor's X goes through
    /// [`Connections::editor_cancel`], so unsaved changes are never lost
    /// silently (DEFAULT-UI §4.1).
    pub fn show(
        &mut self,
        ctx: &Context,
        mode: InterfaceMode,
        conns: &mut Connections,
        session: Session,
    ) -> Vec<Action> {
        let mut closed = Vec::new();
        let mut actions = Vec::new();
        for &shell in &self.open {
            let (title, _, size, min, _) = spec(shell);
            let id = ViewportId::from_hash_of(("septuvo-shell", title));
            let builder = ViewportBuilder::default()
                .with_title(title)
                .with_inner_size(size)
                .with_min_inner_size(min);
            let (close, action) = ctx.show_viewport_immediate(id, builder, |ui, class| {
                let mut body = |ui: &mut Ui| {
                    let action = page(ui, shell, mode, conns, session);
                    if shell == Shell::Editor {
                        // The editor has its own Cancel.
                        return (false, action);
                    }
                    ui.add_space(12.0);
                    (ui.button("Close").clicked(), action)
                };
                if class == ViewportClass::EmbeddedWindow {
                    // Tests and platforms without multiple windows: egui
                    // already wraps the area in a window inside the main one.
                    body(ui)
                } else {
                    let frame = egui::Frame::new().fill(token::MIDNIGHT).inner_margin(24);
                    let scroll = shell != Shell::Editor;
                    let (clicked, action) = frame
                        .show(ui, |ui| {
                            if scroll {
                                egui::ScrollArea::vertical().show(ui, body).inner
                            } else {
                                body(ui)
                            }
                        })
                        .inner;
                    let x = ui.input(|i| i.viewport().close_requested());
                    if x && shell == Shell::Editor {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::CancelClose);
                        conns.editor_cancel();
                    }
                    (clicked || (x && shell != Shell::Editor), action)
                }
            });
            if close {
                closed.push(shell);
            }
            actions.extend(action);
        }
        self.open.retain(|s| !closed.contains(s));
        actions
    }
}
