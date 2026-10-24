//! Design preview of the Septuvo VPN main windows with **sample data**
//! (N4-01, N4-02).
//!
//! Shows the Simple or Advanced main window in each state, from a fake
//! service, so the owner can compare it with `docs/ui/desktop/*.png`. The
//! window title says "SAMPLE DATA". Development only: an example, never part
//! of the `septuvo-vpn` binary. `protected` is a state today's service can
//! never report (no tunnel before G1), and its Local Network Access value is
//! invented too; it exists only to check the design.
//!
//! ```text
//! cargo run --example preview -- [empty|blocking|captive|failed|lost|disconnecting|protected|lockdown|lan-allowed|saved|saved-blocking]
//!                                [--advanced [--settings | --connections]] [--open selector|connections|add|editor]
//!                                [--size 780x760] [--screenshot out.png]
//! ```
//!
//! `saved` and `saved-blocking` have three sample saved connections and a
//! draft (N4-05). `--open` shows a secondary area inside the main window
//! (so one screenshot holds it) or the profile selector's list.
//!
//! ```text
//! ```
//!
//! With `--screenshot`, the window saves itself as a PNG after 40 frames (once
//! windows have faded in)
//! and closes (used in CI, under Xvfb with Mesa's software OpenGL).

use eframe::egui;
use septuvo_cli::watch::Snapshot;
use septuvo_desktop::advanced_window::Page;
use septuvo_desktop::app::{SeptuvoApp, native_options, sizes};
use septuvo_desktop::main_window::Shell;
use septuvo_desktop::prefs::{Bounds, InterfaceMode, Prefs};
use septuvo_desktop::service::Fake;
use septuvo_desktop::state::samples;
use std::path::PathBuf;
use std::process::ExitCode;

struct Preview {
    app: SeptuvoApp,
    screenshot: Option<PathBuf>,
    frames: u32,
    requested: bool,
    done: bool,
}

impl eframe::App for Preview {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if self.done {
            // Closing after the screenshot: the app's own close handling
            // (disconnect first) is not part of the preview.
            return;
        }
        self.app.ui(ui, frame);
        let Some(path) = self.screenshot.clone() else {
            return;
        };
        self.frames += 1;
        let ctx = ui.ctx().clone();
        let shot = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = shot {
            let rgba: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(egui::Color32::to_srgba_unmultiplied)
                .collect();
            let [w, h] = image.size;
            match std::fs::write(
                &path,
                png(
                    u32::try_from(w).unwrap_or(0),
                    u32::try_from(h).unwrap_or(0),
                    &rgba,
                ),
            ) {
                Ok(()) => println!("wrote {} ({w}x{h})", path.display()),
                Err(e) => eprintln!("preview: cannot write {}: {e}", path.display()),
            }
            self.done = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.frames >= 40 && !self.requested {
            self.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ctx.request_repaint();
    }
}

/// Sample saved connections (documentation addresses only).
const SAMPLE_CONNECTIONS: [&str; 3] = [
    "version = 1\n[server]\nendpoints = [\"203.0.113.10:51820\"]\n[routing]\nmode = \"full-tunnel\"\n[dns]\nresolvers = [\"10.77.0.1\"]\n[connection]\nname = \"Home VPN\"\n",
    "version = 1\n[server]\nendpoints = [\"198.51.100.7:51820\"]\n[routing]\nmode = \"split\"\ntunnel = [\"10.20.0.0/16\"]\n[dns]\nresolvers = [\"10.20.0.53\"]\n[connection]\nname = \"Office\"\n",
    "version = 1\n[server]\nendpoints = [\"192.0.2.44:443\"]\n[routing]\nmode = \"full-tunnel\"\n[dns]\nresolvers = [\"10.9.0.1\"]\n[connection]\nname = \"Travel\"\n",
];

fn fake(state: &str) -> Option<Fake> {
    let svc = |r| Snapshot::Service(r);
    let off = svc(samples::disconnected());
    Some(match state {
        "empty" => Fake::new(off.clone(), off),
        // "disconnecting" shows the blocking state with the button pressed.
        "blocking" | "disconnecting" => Fake::new(svc(samples::blocking()), off),
        "captive" => Fake::new(svc(samples::captive()), off),
        "failed" => Fake::new(svc(samples::kill_switch_missing()), off),
        "lost" => {
            let lost =
                Snapshot::Lost("no Septuvo service is running (looked in /run/septuvo)".into());
            Fake::new(lost.clone(), lost)
        }
        // Lockdown Mode blocking, no connection (N4-04).
        "lockdown" => Fake::new(svc(samples::lockdown()), off),
        // The same, just after the user allowed Local Network Access: the
        // one short note (ADVANCED-UI §11.3).
        "lan-allowed" => {
            let mut r = samples::lockdown();
            for (k, v) in &mut r.fields {
                if k == "local_network" {
                    *v = "allowed".into();
                }
            }
            Fake::new(svc(r), off)
        }
        "saved" => Fake::new(off.clone(), off).with_connections(&SAMPLE_CONNECTIONS),
        "saved-blocking" => {
            Fake::new(svc(samples::blocking()), off).with_connections(&SAMPLE_CONNECTIONS)
        }
        "protected" => Fake::new(
            svc(samples::protected().with("local_network", "allowed")),
            off,
        ),
        _ => return None,
    })
}

fn main() -> ExitCode {
    let mut state = "empty".to_string();
    let mut size = None;
    let mut mode = InterfaceMode::Simple;
    let mut settings = false;
    let mut connections_tab = false;
    let mut open: Option<String> = None;
    let mut screenshot = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--size" => {
                let v = args.next().unwrap_or_default();
                let parsed = v
                    .split_once('x')
                    .and_then(|(w, h)| Some([w.parse().ok()?, h.parse().ok()?]));
                let Some(s) = parsed else {
                    eprintln!("preview: --size needs WIDTHxHEIGHT");
                    return ExitCode::from(2);
                };
                size = Some(s);
            }
            "--advanced" => mode = InterfaceMode::Advanced,
            "--settings" => settings = true,
            "--connections" => connections_tab = true,
            "--open" => open = args.next(),
            "--screenshot" => screenshot = args.next().map(PathBuf::from),
            s => state = s.to_string(),
        }
    }
    let Some(link) = fake(&state) else {
        eprintln!("preview: unknown state `{state}`");
        return ExitCode::from(2);
    };
    let size = size.unwrap_or(sizes(mode).0);
    let mut prefs = Prefs {
        interface_mode: mode,
        ..Prefs::default()
    };
    prefs.set_bounds_for(
        mode,
        Bounds {
            width: size[0],
            height: size[1],
            position: None,
            maximized: false,
        },
    );
    let mut options = native_options(&prefs);
    options.viewport = options
        .viewport
        .with_title(format!("Septuvo VPN (SAMPLE DATA: {state})"))
        .with_resizable(screenshot.is_none());
    let disconnecting = state == "disconnecting";
    let lan_note = state == "lan-allowed";
    let shell = match open.as_deref() {
        Some("connections") => Some(Shell::Connections),
        Some("add") => Some(Shell::AddImport),
        Some("editor") => Some(Shell::Editor),
        _ => None,
    };
    let selector = open.as_deref() == Some("selector");
    let result = eframe::run_native(
        "Septuvo VPN (design preview)",
        options,
        Box::new(move |cc| {
            // Secondary areas inside the main window, so one screenshot
            // holds them.
            cc.egui_ctx.set_embed_viewports(shell.is_some());
            let mut app = SeptuvoApp::new(Box::new(link), prefs, None);
            if settings {
                app.open_page(Page::Settings);
            }
            if connections_tab {
                app.open_page(Page::Connections);
            }
            app.preview_open(shell, selector);
            if disconnecting {
                app.preview_disconnecting();
            }
            if lan_note {
                app.preview_local_network_note();
            }
            Ok(Box::new(Preview {
                app,
                screenshot,
                frames: 0,
                requested: false,
                done: false,
            }))
        }),
    );
    if let Err(e) = result {
        eprintln!("preview: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

// --- A minimal PNG writer (uncompressed deflate), standard library only ---

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    out.extend_from_slice(&u32::try_from(data.len()).unwrap_or(0).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    for row in rgba.chunks(width as usize * 4) {
        raw.push(0); // filter: none
        raw.extend_from_slice(row);
    }
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(i + 1 == blocks.len()));
        let len = u16::try_from(block.len()).unwrap_or(0);
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    let (mut s1, mut s2) = (1u32, 0u32);
    for &byte in &raw {
        s1 = (s1 + u32::from(byte)) % 65_521;
        s2 = (s2 + s1) % 65_521;
    }
    zlib.extend_from_slice(&((s2 << 16) | s1).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, *b"IHDR", &ihdr);
    chunk(&mut out, *b"IDAT", &zlib);
    chunk(&mut out, *b"IEND", &[]);
    out
}
