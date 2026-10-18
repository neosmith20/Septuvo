//! Design preview of the Septuvo VPN app with **sample data** (N4-01).
//!
//! Shows the styled status page in each state the lab service can report,
//! so the owner can approve the visual direction before more is built. Every
//! sample is labelled "SAMPLE DATA" on screen. Development only: an example,
//! never part of the `septuvo-vpn` binary.
//!
//! ```text
//! cargo run --example preview -- [disconnected|blocking|captive|lost|pending|about]
//!                                [--size 1100x760] [--screenshot out.png]
//! ```
//!
//! With `--screenshot`, the window saves itself as a PNG after a few frames
//! and closes (used in CI, under Xvfb with Mesa's software OpenGL).

use eframe::egui;
use septuvo_cli::watch::Snapshot;
use septuvo_desktop::app::{SeptuvoApp, StatusSource, native_options};
use septuvo_desktop::pages::Page;
use septuvo_desktop::view::{StatusView, samples};
use std::path::PathBuf;
use std::process::ExitCode;

struct Preview {
    app: SeptuvoApp,
    screenshot: Option<PathBuf>,
    frames: u32,
    requested: bool,
}

impl eframe::App for Preview {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
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
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.frames >= 5 && !self.requested {
            self.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ctx.request_repaint();
    }
}

fn main() -> ExitCode {
    let mut state = "blocking".to_string();
    let mut size = [1100.0, 760.0];
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
    let (page, source) = match state.as_str() {
        "disconnected" => (
            Page::Status,
            sample(&Snapshot::Service(samples::disconnected())),
        ),
        "blocking" => (
            Page::Status,
            sample(&Snapshot::Service(samples::blocking())),
        ),
        "captive" => (Page::Status, sample(&Snapshot::Service(samples::captive()))),
        "lost" => (
            Page::Status,
            sample(&Snapshot::Lost(
                "no Septuvo service is running (looked in /run/septuvo)".into(),
            )),
        ),
        "pending" => (Page::Status, StatusSource::Pending),
        "about" => (Page::About, StatusSource::Pending),
        other => {
            eprintln!("preview: unknown state `{other}`");
            return ExitCode::from(2);
        }
    };
    let result = eframe::run_native(
        "Septuvo VPN (design preview)",
        native_options(size),
        Box::new(move |_cc| {
            let mut app = SeptuvoApp::new(source);
            app.page = page;
            Ok(Box::new(Preview {
                app,
                screenshot,
                frames: 0,
                requested: false,
            }))
        }),
    );
    if let Err(e) = result {
        eprintln!("preview: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn sample(s: &Snapshot) -> StatusSource {
    StatusSource::Sample(StatusView::from_snapshot(s))
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
