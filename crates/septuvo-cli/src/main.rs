//! `septuvo` command-line tool — N1-01 skeleton.
//!
//! Development preview only: it has no VPN, network or cryptographic
//! functionality. Commands such as `status` and `policy check` arrive in N1-04.

use std::io::{self, Write};
use std::process::ExitCode;

const USAGE: &str = "\
septuvo - Septuvo VPN command-line tool (development preview)

This build has no VPN functionality: it cannot connect, carry traffic
or protect anything yet.

Usage:
  septuvo help       Show this help
  septuvo version    Show the version
";

/// Exit code for a successful command.
const EXIT_OK: u8 = 0;
/// Exit code for a usage error (unknown command or extra arguments).
const EXIT_USAGE: u8 = 2;

/// Runs the command line `args` (without the program name), writing normal
/// output to `out` and errors to `err`. Returns the process exit code.
fn run(args: &[String], out: &mut impl Write, err: &mut impl Write) -> io::Result<u8> {
    match args {
        [] => {
            out.write_all(USAGE.as_bytes())?;
            Ok(EXIT_OK)
        }
        [cmd] if cmd == "help" || cmd == "--help" || cmd == "-h" => {
            out.write_all(USAGE.as_bytes())?;
            Ok(EXIT_OK)
        }
        [cmd] if cmd == "version" || cmd == "--version" || cmd == "-V" => {
            writeln!(out, "septuvo {}", env!("CARGO_PKG_VERSION"))?;
            Ok(EXIT_OK)
        }
        [cmd, ..] => {
            // Echo the argument only after removing control characters, so a
            // hostile argument cannot inject terminal escape sequences.
            let shown: String = cmd.chars().filter(|c| !c.is_control()).take(64).collect();
            writeln!(err, "septuvo: unknown command or arguments: '{shown}'")?;
            writeln!(err, "Run 'septuvo help' for usage.")?;
            Ok(EXIT_USAGE)
        }
    }
}

fn main() -> ExitCode {
    // Non-UTF-8 arguments are replaced rather than causing a panic.
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let code = run(&args, &mut io::stdout().lock(), &mut io::stderr().lock()).unwrap_or(1);
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(args: &[&str]) -> (u8, String, String) {
        let args: Vec<String> = args.iter().map(ToString::to_string).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(&args, &mut out, &mut err).expect("writing to a Vec cannot fail");
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn no_arguments_prints_usage() {
        let (code, out, err) = call(&[]);
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("Usage:"));
        assert!(err.is_empty());
    }

    #[test]
    fn help_variants_print_usage() {
        for arg in ["help", "--help", "-h"] {
            let (code, out, _) = call(&[arg]);
            assert_eq!(code, EXIT_OK, "{arg}");
            assert!(out.contains("Usage:"), "{arg}");
        }
    }

    #[test]
    fn usage_never_claims_vpn_functionality() {
        let (_, out, _) = call(&["help"]);
        assert!(out.contains("no VPN functionality"));
        let lowered = out.to_lowercase();
        for word in ["connected", "secure", "encrypted", "protected"] {
            assert!(!lowered.contains(word), "usage must not claim '{word}'");
        }
    }

    #[test]
    fn version_variants_print_version() {
        for arg in ["version", "--version", "-V"] {
            let (code, out, _) = call(&[arg]);
            assert_eq!(code, EXIT_OK, "{arg}");
            assert_eq!(out, format!("septuvo {}\n", env!("CARGO_PKG_VERSION")));
        }
    }

    #[test]
    fn unknown_command_is_a_usage_error() {
        let (code, out, err) = call(&["connect"]);
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty());
        assert!(err.contains("unknown command"));
    }

    #[test]
    fn extra_arguments_are_rejected() {
        let (code, _, _) = call(&["version", "extra"]);
        assert_eq!(code, EXIT_USAGE);
    }

    #[test]
    fn hostile_argument_is_not_echoed_with_control_characters() {
        let (code, _, err) = call(&["\u{1b}[31mred\u{7}"]);
        assert_eq!(code, EXIT_USAGE);
        assert!(!err.chars().any(|c| c.is_control() && c != '\n'));
    }
}
