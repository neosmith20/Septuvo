//! Design preview of the Septuvo VPN main window with **sample data** (N4-01).
//!
//! Shows the Simple main window in each state, from a fake service, so the
//! owner can compare it with `docs/ui/desktop/Default_VPN_UI.png`. The
//! window title says "SAMPLE DATA". Development only: an example, never part
//! of the `septuvo-vpn` binary. `protected` is a state today's service can
//! never report (no tunnel before G1); it exists only to check the design.
//!
//! ```text
//! cargo run --example preview -- [empty|blocking|captive|failed|lost|disconnecting|protected]
//!                                [--size 780x760] [--screenshot out.png]
//! ```
//!
//! With `--screenshot`, the window saves itself as a PNG after a few frames
//! and closes (used in CI, under Xvfb with Mesa's software OpenGL).

use eframe::egui;
use septuvo_cli::watch::Snapshot;
use septuvo_desktop::app::{SeptuvoApp, native_options};
use septuvo_desktop::prefs::{Bounds, Prefs};
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
        } else if self.frames >= 6 && !self.requested {
            self.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ctx.request_repaint();
    }
}

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
        "protected" => Fake::new(svc(samples::protected()), off),
        _ => return None,
    })
}

fn main() -> ExitCode {
    let mut state = "empty".to_string();
    let mut size = [780.0, 760.0];
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
                size = s;
            }
            "--screenshot" => screenshot = args.next().map(PathBuf::from),
            s => state = s.to_string(),
        }
    }
    let Some(link) = fake(&state) else {
        eprintln!("preview: unknown state `{state}`");
        return ExitCode::from(2);
    };
    let prefs = Prefs {
        bounds: Some(Bounds {
            width: size[0],
            height: size[1],
            position: None,
            maximized: false,
        }),
        ..Prefs::default()
    };
    let mut options = native_options(&prefs);
    options.viewport = options
        .viewport
        .with_title(format!("Septuvo VPN (SAMPLE DATA: {state})"))
        .with_resizable(screenshot.is_none());
    let disconnecting = state == "disconnecting";
    let result = eframe::run_native(
        "Septuvo VPN (design preview)",
        options,
        Box::new(move |_cc| {
            let mut app = SeptuvoApp::new(Box::new(link), prefs, None);
            if disconnecting {
                app.preview_disconnecting();
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
