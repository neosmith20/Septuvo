//! `septuvo-server`: local Septuvo Server administration (development
//! preview). See `septuvo-server --help`.

use std::io::Write;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> ExitCode {
    // Non-UTF-8 arguments are replaced rather than causing a panic.
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let outcome = septuvo_server::cli::run(&args, now);
    let _ = std::io::stdout().write_all(outcome.out.as_bytes());
    if !outcome.err.is_empty() {
        let _ = writeln!(
            std::io::stderr(),
            "septuvo-server: {}",
            outcome.err.trim_end()
        );
    }
    ExitCode::from(outcome.code)
}
