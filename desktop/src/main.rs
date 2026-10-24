//! `septuvo-vpn`: the Septuvo VPN desktop app (development preview).

use septuvo_desktop::app::{SeptuvoApp, native_options};
use septuvo_desktop::files::Portal;
use septuvo_desktop::prefs::{Prefs, default_path};
use septuvo_desktop::service::Worker;
use septuvo_desktop::state::plain;
use septuvo_desktop::{check_user, current_uid};
use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "Septuvo VPN (development preview, no VPN functionality before the G1 review)

usage: septuvo-vpn [--runtime-dir DIR] | --help | --version

  --runtime-dir DIR   where the Septuvo service's control socket is
                      (default /run/septuvo; used by the lab tests)

Runs as your normal user, never as root. The Septuvo service (septuvod)
does the privileged work.";

fn main() -> ExitCode {
    let mut runtime_dir = PathBuf::from(septuvo_ipc::DEFAULT_RUNTIME_DIR);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                println!("septuvo-vpn {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--runtime-dir" => {
                let Some(dir) = args.next() else {
                    eprintln!("septuvo-vpn: --runtime-dir needs a directory\n\n{HELP}");
                    return ExitCode::from(2);
                };
                runtime_dir = PathBuf::from(dir);
            }
            other => {
                eprintln!("septuvo-vpn: unknown argument `{}`\n\n{HELP}", plain(other));
                return ExitCode::from(2);
            }
        }
    }
    match current_uid().map(check_user) {
        Ok(Ok(())) => {}
        Ok(Err(refused)) => {
            eprintln!("septuvo-vpn: {}", refused.0);
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("septuvo-vpn: cannot tell which user this is ({e}); not starting");
            return ExitCode::from(2);
        }
    }
    let prefs_path = default_path();
    let prefs = prefs_path.as_deref().map(Prefs::load).unwrap_or_default();
    let options = native_options(&prefs);
    let result = eframe::run_native(
        "Septuvo VPN",
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let link = Worker::start(runtime_dir, move || ctx.request_repaint());
            let mut app = SeptuvoApp::new(Box::new(link), prefs, prefs_path);
            let ctx = cc.egui_ctx.clone();
            app.set_files(Box::new(Portal::new(move || ctx.request_repaint())));
            Ok(Box::new(app))
        }),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("septuvo-vpn: could not open the window: {e}");
            ExitCode::FAILURE
        }
    }
}
