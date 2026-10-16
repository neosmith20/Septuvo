//! `septuvo-config check device|server FILE`: validate a configuration file.
//!
//! Exit status: 0 valid, 1 invalid, 2 usage error.

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: septuvo-config check device FILE\n       septuvo-config check server FILE\n\n\
Checks a Septuvo configuration file and prints a summary. Nothing is applied.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parts: Vec<&str> = args.iter().map(String::as_str).collect();
    let (kind, file) = match parts.as_slice() {
        ["check", kind @ ("device" | "server"), file] => (*kind, Path::new(*file)),
        ["--help" | "-h" | "help"] => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let result = if kind == "device" {
        septuvo_config::load_device(file).map(|c| c.summary())
    } else {
        septuvo_config::load_server(file).map(|c| c.summary())
    };
    match result {
        Ok(summary) => {
            println!("{}: valid {kind} configuration\n{summary}", file.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: {e}", file.display());
            ExitCode::from(1)
        }
    }
}
