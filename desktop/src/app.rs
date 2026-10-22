//! The application: service link, close behaviour, main window and shells.

use eframe::egui::{self, Ui, ViewportCommand};
use std::path::PathBuf;
use std::sync::Arc;

use crate::assets::{ICON, Textures};
use crate::close::{COULD_NOT, Close, Step};
use crate::main_window::{self, Action};
use crate::prefs::{Bounds, Prefs};
use crate::service::{Link, REFRESH};
use crate::shells::Shells;
use crate::state::{ButtonAction, MainView, Pending};
use crate::theme;
use crate::{APP_ID, theme::token};

/// Initial and minimum window sizes (DEFAULT-UI §2.3).
pub const INITIAL_SIZE: [f32; 2] = [780.0, 760.0];
/// See [`INITIAL_SIZE`].
pub const MIN_SIZE: [f32; 2] = [520.0, 580.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    Button,
    Close,
}

/// The Septuvo VPN app.
pub struct SeptuvoApp {
    link: Box<dyn Link>,
    prefs: Prefs,
    prefs_path: Option<PathBuf>,
    close: Close,
    pending: Pending,
    origin: Option<Origin>,
    notice: Option<String>,
    shells: Shells,
    textures: Option<Textures>,
    exit: bool,
    placed: bool,
}

impl SeptuvoApp {
    /// A new app. `prefs_path` is where preferences are saved (`None` in tests).
    #[must_use]
    pub fn new(link: Box<dyn Link>, prefs: Prefs, prefs_path: Option<PathBuf>) -> SeptuvoApp {
        SeptuvoApp {
            link,
            prefs,
            prefs_path,
            close: Close::default(),
            pending: Pending::None,
            origin: None,
            notice: None,
            shells: Shells::default(),
            textures: None,
            exit: false,
            placed: false,
        }
    }

    /// True once the app has decided to exit (tests).
    #[must_use]
    pub fn exit_requested(&self) -> bool {
        self.exit
    }

    /// Shows "Disconnecting…" without asking the service (design preview
    /// only: the preview's fake would otherwise answer at once).
    pub fn preview_disconnecting(&mut self) {
        self.pending = Pending::Disconnecting;
    }

    /// Replaces the service link (tests: the service changes state).
    pub fn set_link(&mut self, link: Box<dyn Link>) {
        self.link = link;
    }

    /// The secondary windows (tests).
    #[must_use]
    pub fn shells(&self) -> &Shells {
        &self.shells
    }

    /// Fonts take effect from the next frame, so nothing is drawn on the first.
    fn setup(&mut self, ctx: &egui::Context) -> bool {
        if self.textures.is_some() {
            return true;
        }
        theme::apply(ctx);
        self.textures = Some(Textures::load(ctx));
        ctx.request_repaint();
        false
    }

    fn save_bounds(&mut self, ctx: &egui::Context) {
        let (inner, outer, maximized) = ctx.input(|i| {
            let v = i.viewport();
            (v.inner_rect, v.outer_rect, v.maximized.unwrap_or(false))
        });
        if let Some(inner) = inner {
            let keep = self.prefs.bounds.filter(|_| maximized);
            self.prefs.bounds = Some(match keep {
                // Maximized: keep the last normal size, remember the state.
                Some(b) => Bounds {
                    maximized: true,
                    ..b
                },
                None => Bounds {
                    width: inner.width(),
                    height: inner.height(),
                    position: outer.map(|o| (o.left(), o.top())),
                    maximized,
                },
            });
        }
        if let Some(path) = &self.prefs_path {
            // A failed save only loses the window position; never block exit.
            let _ = self.prefs.save(path);
        }
    }

    /// Moves a restored window that would be off-screen back to the centre.
    fn keep_on_screen(&mut self, ctx: &egui::Context) {
        if self.placed {
            return;
        }
        let (outer, monitor) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size));
        if let (Some(o), Some(m)) = (outer, monitor) {
            self.placed = true;
            let off = o.left() > m.x - 80.0
                || o.top() > m.y - 80.0
                || o.right() < 80.0
                || o.top() < -10.0;
            if off {
                let pos = egui::pos2(
                    ((m.x - o.width()) / 2.0).max(0.0),
                    ((m.y - o.height()) / 2.0).max(0.0),
                );
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos));
            }
        }
    }

    fn handle_close(&mut self, ctx: &egui::Context, snapshot: &septuvo_cli::watch::Snapshot) {
        if self.exit || !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        match self.close.requested(snapshot) {
            Step::Exit => {
                self.exit = true;
                self.save_bounds(ctx);
            }
            Step::Stay => ctx.send_viewport_cmd(ViewportCommand::CancelClose),
            Step::StayAndDisconnect => {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.notice = None;
                self.pending = Pending::Disconnecting;
                self.origin = Some(Origin::Close);
                self.link.disconnect();
            }
        }
    }

    fn handle_result(&mut self, ctx: &egui::Context) {
        let Some(result) = self.link.result() else {
            return;
        };
        self.pending = Pending::None;
        match self.origin.take() {
            Some(Origin::Close) => {
                if self.close.result(result) == Step::Exit {
                    self.exit = true;
                    self.save_bounds(ctx);
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
            _ => {
                if let Err(why) = result {
                    self.notice = Some(format!(
                        "{COULD_NOT} {why} Open Help & Diagnostics for more information."
                    ));
                }
            }
        }
    }

    /// Draws one frame into `ui` (the whole window). Public for tests.
    pub fn draw(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if !self.setup(&ctx) {
            return;
        }
        self.keep_on_screen(&ctx);
        self.handle_result(&ctx);
        let snapshot = self.link.latest();
        self.handle_close(&ctx, &snapshot);
        let notice = self.close.notice.clone().or_else(|| self.notice.clone());
        let view = MainView::build(&snapshot, self.pending, notice.as_deref());
        let Some(textures) = self.textures.as_ref() else {
            return;
        };
        for action in main_window::draw(ui, &view, textures) {
            match action {
                Action::Main(ButtonAction::Disconnect) if self.pending == Pending::None => {
                    self.close.reset();
                    self.notice = None;
                    self.pending = Pending::Disconnecting;
                    self.origin = Some(Origin::Button);
                    self.link.disconnect();
                }
                Action::Main(_) => {}
                Action::Open(s) => self.shells.open(s),
            }
        }
        self.shells.show(&ctx);
        ctx.request_repaint_after(REFRESH);
    }
}

impl eframe::App for SeptuvoApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::Frame::new().fill(token::MIDNIGHT).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            self.draw(ui);
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(path) = &self.prefs_path {
            let _ = self.prefs.save(path);
        }
    }
}

/// Window options from the saved preferences.
#[must_use]
pub fn native_options(prefs: &Prefs) -> eframe::NativeOptions {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Septuvo VPN")
        .with_app_id(APP_ID)
        .with_inner_size(INITIAL_SIZE)
        .with_min_inner_size(MIN_SIZE)
        .with_icon(Arc::new(egui::IconData {
            rgba: ICON.rgba.to_vec(),
            width: 128,
            height: 128,
        }));
    let mut centered = true;
    if let Some(b) = prefs.bounds {
        viewport = viewport.with_inner_size([b.width.max(MIN_SIZE[0]), b.height.max(MIN_SIZE[1])]);
        if let Some((x, y)) = b.position {
            viewport = viewport.with_position([x, y]);
            centered = false;
        }
        if b.maximized {
            viewport = viewport.with_maximized(true);
        }
    }
    eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        centered,
        ..Default::default()
    }
}
