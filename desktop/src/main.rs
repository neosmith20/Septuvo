//! `septuvo-vpn`: the Septuvo VPN desktop app (development preview).

use septuvo_desktop::app::{SeptuvoApp, StatusSource, native_options};
use septuvo_desktop::{check_user, current_uid};
use std::process::ExitCode;

const HELP: &str = "Septuvo VPN (development preview, no VPN functionality)

usage: septuvo-vpn [--help | --version]

Runs as your normal user, never as root. The Septuvo service (septuvod)
does the privileged work.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => {}
        Some("--help" | "-h") => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Some("--version" | "-V") => {
            println!("septuvo-vpn {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some(other) => {
            eprintln!(
                "septuvo-vpn: unknown argument `{}`\n\n{HELP}",
                septuvo_desktop::view::plain(other)
            );
            return ExitCode::from(2);
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
    let result = eframe::run_native(
        "Septuvo VPN",
        native_options([920.0, 680.0]),
        Box::new(|_cc| Ok(Box::new(SeptuvoApp::new(StatusSource::Pending)))),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("septuvo-vpn: could not open the window: {e}");
            ExitCode::FAILURE
        }
    }
}
