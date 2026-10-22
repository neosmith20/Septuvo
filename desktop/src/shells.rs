//! The secondary windows, as labelled shells (N4-01).
//!
//! Each opens as its own window at its specified size (DEFAULT-UI §3) and
//! says plainly that it is not available yet; later N4 tasks fill them. No
//! shell pretends to work. Closing one never touches the connection.

use eframe::egui::{self, Context, RichText, ViewportBuilder, ViewportClass, ViewportId};

use crate::main_window::Shell;
use crate::theme::{semibold, token};

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
    }
}

/// The text shown in a shell.
pub const NOT_YET: &str = "This window is not available yet in this development build.";

/// Which shells are open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shells {
    open: Vec<Shell>,
}

impl Shells {
    /// Opens a shell (or keeps it open).
    pub fn open(&mut self, s: Shell) {
        if !self.open.contains(&s) {
            self.open.push(s);
        }
    }

    /// True if the shell is open.
    #[must_use]
    pub fn is_open(&self, s: Shell) -> bool {
        self.open.contains(&s)
    }

    /// Shows every open shell; closes those the user closed.
    pub fn show(&mut self, ctx: &Context) {
        let mut closed = Vec::new();
        for &shell in &self.open {
            let (title, subtitle, size, min, task) = spec(shell);
            let id = ViewportId::from_hash_of(("septuvo-shell", title));
            let builder = ViewportBuilder::default()
                .with_title(title)
                .with_inner_size(size)
                .with_min_inner_size(min);
            let close = ctx.show_viewport_immediate(id, builder, |ui, class| {
                let body = |ui: &mut egui::Ui| {
                    ui.label(RichText::new(title).font(semibold(24.0)));
                    if !subtitle.is_empty() {
                        ui.label(RichText::new(subtitle).color(token::MUTED));
                    }
                    ui.add_space(16.0);
                    ui.label(NOT_YET);
                    ui.label(
                        RichText::new(format!("It arrives with development task {task}."))
                            .color(token::MUTED),
                    );
                    if shell == Shell::Settings {
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
                    }
                    ui.add_space(12.0);
                    ui.button("Close").clicked()
                };
                if class == ViewportClass::EmbeddedWindow {
                    // Tests and platforms without multiple windows.
                    let mut clicked = false;
                    egui::Window::new(title)
                        .collapsible(false)
                        .show(ui.ctx(), |ui| clicked = body(ui));
                    clicked
                } else {
                    let frame = egui::Frame::new().fill(token::MIDNIGHT).inner_margin(24);
                    let clicked = frame.show(ui, body).inner;
                    clicked || ui.input(|i| i.viewport().close_requested())
                }
            });
            if close {
                closed.push(shell);
            }
        }
        self.open.retain(|s| !closed.contains(s));
    }
}
