//! The application: service link, close behaviour, the Simple and Advanced
//! main windows, Interface Mode switching and the secondary areas.

use eframe::egui::{self, Ui, ViewportCommand};
use std::path::PathBuf;
use std::sync::Arc;

use crate::advanced_window::{self, Page};
use crate::assets::{ICON, Textures};
use crate::close::{COULD_NOT, Close, Step};
use crate::connections::{Connections, Effect, Session};
use crate::drafts::{self, Drafts};
use crate::files::{FakeFiles, Files};
use crate::home::{HomeView, LanNote};
use crate::library::{Key, Library};
use crate::main_window::{self, Action, Shell};
use crate::prefs::{Bounds, InterfaceMode, Prefs};
use crate::service::{Link, REFRESH};
use crate::shells::Shells;
use crate::state::{ButtonAction, Choice, MainView, Pending};
use crate::theme;
use crate::{APP_ID, theme::token};

/// Initial and minimum Simple window sizes (DEFAULT-UI §2.3).
pub const INITIAL_SIZE: [f32; 2] = [780.0, 760.0];
/// See [`INITIAL_SIZE`].
pub const MIN_SIZE: [f32; 2] = [520.0, 580.0];

/// Initial and minimum window size for a mode.
#[must_use]
pub fn sizes(mode: InterfaceMode) -> ([f32; 2], [f32; 2]) {
    match mode {
        InterfaceMode::Simple => (INITIAL_SIZE, MIN_SIZE),
        InterfaceMode::Advanced => (advanced_window::INITIAL_SIZE, advanced_window::MIN_SIZE),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    Button,
    Close,
    /// Disconnecting the connection in use before deleting it (N4-05).
    DeleteFirst,
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
    lan: LanNote,
    lan_wanted: bool,
    shells: Shells,
    page: Page,
    conns: Connections,
    files: Box<dyn Files>,
    textures: Option<Textures>,
    exit: bool,
    placed: bool,
}

impl SeptuvoApp {
    /// A new app. `prefs_path` is where preferences are saved (`None` in tests).
    #[must_use]
    pub fn new(link: Box<dyn Link>, prefs: Prefs, prefs_path: Option<PathBuf>) -> SeptuvoApp {
        let drafts = prefs_path
            .as_deref()
            .and_then(drafts::dir_for)
            .map_or_else(Drafts::in_memory, Drafts::load);
        let conns = Connections::new(Library::new(drafts, prefs.selected_draft));
        SeptuvoApp {
            link,
            prefs,
            prefs_path,
            close: Close::default(),
            pending: Pending::None,
            origin: None,
            notice: None,
            lan: LanNote::None,
            lan_wanted: false,
            shells: Shells::default(),
            page: Page::default(),
            conns,
            files: Box::new(FakeFiles::default()),
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

    /// Shows the note after allowing Local Network Access without asking
    /// the service (design preview only).
    pub fn preview_local_network_note(&mut self) {
        self.lan = LanNote::Allowed;
    }

    /// Opens a secondary area and, for the profile selector, its list
    /// (design preview and tests only).
    pub fn preview_open(&mut self, shell: Option<Shell>, selector: bool) {
        if let Some(s) = shell {
            if s == Shell::Editor {
                self.conns.create_new();
            } else {
                self.shells.open(s);
            }
        }
        self.conns.selector_open = selector;
    }

    /// Uses these file dialogs (the desktop portal in the app; prepared
    /// answers in tests).
    pub fn set_files(&mut self, files: Box<dyn Files>) {
        self.files = files;
    }

    /// The saved-connections controller (tests).
    #[must_use]
    pub fn connections(&self) -> &Connections {
        &self.conns
    }

    /// Replaces the service link (tests: the service changes state).
    pub fn set_link(&mut self, link: Box<dyn Link>) {
        self.link = link;
    }

    /// The Interface Mode in use.
    #[must_use]
    pub fn mode(&self) -> InterfaceMode {
        self.prefs.interface_mode
    }

    /// Selects an Advanced tab (the design preview).
    pub fn open_page(&mut self, page: Page) {
        self.page = page;
    }

    /// The selected Advanced tab (tests).
    #[must_use]
    pub fn page(&self) -> Page {
        self.page
    }

    /// Switches Interface Mode (DEFAULT-UI §6.2): saved at once, the window
    /// takes the mode's size, and **nothing is sent to the service**: the
    /// connection, pending actions and notices carry over unchanged.
    pub fn set_mode(&mut self, ctx: &egui::Context, mode: InterfaceMode) {
        if mode == self.prefs.interface_mode {
            return;
        }
        let old = self.prefs.interface_mode;
        self.remember_bounds(ctx, old);
        self.prefs.interface_mode = mode;
        self.shells.close_all();
        // The choice is made in Settings, so Settings stays in view: the
        // Settings tab in Advanced, the Settings window in Simple.
        self.page = if mode == InterfaceMode::Advanced {
            Page::Settings
        } else {
            Page::Home
        };
        if mode == InterfaceMode::Simple {
            self.shells.open(main_window::Shell::Settings);
        }
        let (initial, min) = sizes(mode);
        let size = self
            .prefs
            .bounds_for(mode)
            .map_or(initial, |b| [b.width.max(min[0]), b.height.max(min[1])]);
        ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(min.into()));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(size.into()));
        if let Some(path) = &self.prefs_path {
            let _ = self.prefs.save(path);
        }
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

    /// Records the window's current bounds for `mode`.
    fn remember_bounds(&mut self, ctx: &egui::Context, mode: InterfaceMode) {
        let (inner, outer, maximized) = ctx.input(|i| {
            let v = i.viewport();
            (v.inner_rect, v.outer_rect, v.maximized.unwrap_or(false))
        });
        let Some(inner) = inner else { return };
        let bounds = match self.prefs.bounds_for(mode).filter(|_| maximized) {
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
        };
        self.prefs.set_bounds_for(mode, bounds);
    }

    fn save_bounds(&mut self, ctx: &egui::Context) {
        self.remember_bounds(ctx, self.prefs.interface_mode);
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
            Some(Origin::DeleteFirst) => {
                self.conns.on_disconnect_result(&result);
            }
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

    /// The service's answer to a Local Network Access change (N4-04): the
    /// value shown always comes from the service; a refusal says why.
    fn handle_setting_result(&mut self) {
        let Some(result) = self.link.setting_result() else {
            return;
        };
        if self.pending == Pending::Setting {
            self.pending = Pending::None;
        }
        self.lan = match result {
            Ok(()) if self.lan_wanted => LanNote::Allowed,
            Ok(()) => LanNote::None,
            Err(why) => LanNote::Refused(why),
        };
    }

    /// Service answers and file dialog results for the connections.
    fn handle_connections(&mut self, session: Session) {
        for reply in self.link.replies() {
            self.conns.on_reply(reply, session);
        }
        for event in self.files.events() {
            self.conns.on_file(event);
        }
        self.conns.tick();
    }

    /// Carries out what the connections controller asked for.
    fn run_effects(&mut self) {
        for effect in self.conns.take_effects() {
            match effect {
                Effect::Op(op) => self.link.send(op),
                Effect::Disconnect => {
                    if self.pending == Pending::None {
                        self.pending = Pending::Disconnecting;
                        self.origin = Some(Origin::DeleteFirst);
                        self.link.disconnect();
                    } else {
                        self.conns
                            .on_disconnect_result(&Err("Septuvo is busy. Try again.".into()));
                    }
                }
                Effect::OpenFile => self.files.open(),
                Effect::SaveFile {
                    file_name,
                    contents,
                } => self.files.save(file_name, contents),
                Effect::RememberDraft(d) => {
                    self.prefs.selected_draft = d;
                    if let Some(path) = &self.prefs_path {
                        let _ = self.prefs.save(path);
                    }
                }
            }
        }
    }

    /// The connection choice for the main views.
    fn choice(&self) -> Choice {
        let lib = &self.conns.lib;
        let selected = lib.selected();
        Choice {
            name: selected.and_then(|k| lib.name(k)),
            detail: match selected {
                Some(Key::Saved(id)) => lib.saved(id).and_then(crate::library::Saved::server),
                _ => None,
            },
            draft: matches!(selected, Some(Key::Draft(_))),
            any: !lib.saved.is_empty() || !lib.drafts.list().is_empty(),
            connecting: self.conns.connecting.clone(),
            switching: self.conns.switching.clone(),
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
        self.handle_setting_result();
        let snapshot = self.link.latest();
        let session = Session::of(&snapshot);
        self.handle_connections(session);
        self.handle_close(&ctx, &snapshot);
        let notice = self
            .close
            .notice
            .clone()
            .or_else(|| self.notice.clone())
            .or_else(|| self.conns.main_notice.clone());
        let choice = self.choice();
        let Some(textures) = self.textures.as_ref() else {
            return;
        };
        let mode = self.prefs.interface_mode;
        let mut actions = match mode {
            InterfaceMode::Simple => {
                let view =
                    MainView::build_with(&snapshot, self.pending, notice.as_deref(), &choice);
                let entries = self.conns.lib.entries(session.in_use);
                let list = (self.conns.selector_open && view.selector.enabled)
                    .then_some(entries.as_slice());
                main_window::draw(ui, &view, textures, list)
            }
            InterfaceMode::Advanced => {
                let view = HomeView::build_with(
                    &snapshot,
                    self.pending,
                    notice.as_deref(),
                    &self.lan,
                    &choice,
                );
                advanced_window::draw(
                    ui,
                    &view,
                    textures,
                    &mut self.page,
                    mode,
                    &mut self.conns,
                    session,
                )
            }
        };
        actions.extend(self.shells.show(&ctx, mode, &mut self.conns, session));
        for action in actions {
            match action {
                Action::Main(ButtonAction::Disconnect) if self.pending == Pending::None => {
                    self.close.reset();
                    self.notice = None;
                    self.pending = Pending::Disconnecting;
                    self.origin = Some(Origin::Button);
                    self.link.disconnect();
                }
                Action::SetLocalNetwork(allowed) if self.pending == Pending::None => {
                    self.pending = Pending::Setting;
                    self.lan = LanNote::None;
                    self.lan_wanted = allowed;
                    self.link.set_local_network(allowed);
                }
                Action::Main(ButtonAction::Connect) if self.pending == Pending::None => {
                    self.notice = None;
                    self.conns.connect();
                }
                Action::Main(_) | Action::SetLocalNetwork(_) => {}
                Action::Open(s) => self.shells.open(s),
                Action::SetMode(m) => self.set_mode(&ctx, m),
                Action::ToggleSelector => self.conns.selector_open = !self.conns.selector_open,
                Action::CloseSelector => self.conns.selector_open = false,
                Action::Select(key) => {
                    self.notice = None;
                    self.conns.select(key, session);
                }
            }
        }
        // The editor is its own window while it is open.
        if self.conns.editor.is_some() {
            self.shells.open(Shell::Editor);
        } else {
            self.shells.close(Shell::Editor);
        }
        self.run_effects();
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
    let (initial, min) = sizes(prefs.interface_mode);
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Septuvo VPN")
        .with_app_id(APP_ID)
        .with_inner_size(initial)
        .with_min_inner_size(min)
        .with_icon(Arc::new(egui::IconData {
            rgba: ICON.rgba.to_vec(),
            width: 128,
            height: 128,
        }));
    let mut centered = true;
    if let Some(b) = prefs.bounds_for(prefs.interface_mode) {
        viewport = viewport.with_inner_size([b.width.max(min[0]), b.height.max(min[1])]);
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
