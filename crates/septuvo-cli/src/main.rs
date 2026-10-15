//! `septuvo` command-line tool (N1-01, N1-04).
//!
//! Development preview only: it has no VPN, network or cryptographic
//! functionality. `status` shows the four-stage status model (D-020) and
//! `policy check` validates and explains a policy file.

use septuvo_cli::policy_cmd;
use septuvo_cli::status::ConnectionStatus;
use std::io::{self, Write};
use std::process::ExitCode;

const USAGE: &str = "\
septuvo - Septuvo VPN command-line tool (development preview)

This build has no VPN functionality: it cannot connect, carry traffic
or protect anything yet.

Usage:
  septuvo help                         Show this help
  septuvo version                      Show the version
  septuvo status                       Show the four connection checks
  septuvo policy check FILE [--dst ADDRESS] [--path up|down]
                                       Validate a policy file and explain
                                       what happens to traffic

Exit codes: 0 success, 1 invalid policy or unreadable file, 2 usage
error, 3 status shows the VPN is not working.
";

/// Exit code for a successful command.
const EXIT_OK: u8 = 0;
/// Exit code for an invalid policy or an unreadable file.
const EXIT_INVALID: u8 = 1;
/// Exit code for a usage error (unknown command or extra arguments).
const EXIT_USAGE: u8 = 2;
/// Exit code when `status` shows the VPN is not fully working.
const EXIT_NOT_CONNECTED: u8 = 3;

/// Shown by `status` until a Septuvo service exists to ask.
const NO_SERVICE: &str =
    "No Septuvo service is running (development preview): nothing could be checked.";

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
        [cmd] if cmd == "status" => {
            // There is no Septuvo service yet, so every stage is unknown.
            let status = ConnectionStatus::UNKNOWN;
            out.write_all(status.report(Some(NO_SERVICE)).as_bytes())?;
            Ok(if status.is_connected() {
                EXIT_OK
            } else {
                EXIT_NOT_CONNECTED
            })
        }
        [cmd, sub, file, rest @ ..] if cmd == "policy" && sub == "check" => {
            let opts = match policy_cmd::parse_args(rest) {
                Ok(o) => o,
                Err(e) => {
                    writeln!(err, "septuvo: {e}")?;
                    writeln!(err, "Run 'septuvo help' for usage.")?;
                    return Ok(EXIT_USAGE);
                }
            };
            match policy_cmd::load(file) {
                Ok(policy) => {
                    out.write_all(policy_cmd::explain(&policy, &opts).as_bytes())?;
                    Ok(EXIT_OK)
                }
                Err(e) => {
                    writeln!(err, "septuvo: invalid policy: {e}")?;
                    Ok(EXIT_INVALID)
                }
            }
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
        assert_eq!(err, "");
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
        assert_eq!(out, "");
        assert!(err.contains("unknown command"));
    }

    #[test]
    fn extra_arguments_are_rejected() {
        let (code, _, _) = call(&["version", "extra"]);
        assert_eq!(code, EXIT_USAGE);
    }

    fn temp_policy(name: &str, text: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "septuvo-cli-test-{}-{name}.policy",
            std::process::id()
        ));
        std::fs::write(&path, text).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn status_without_a_service_is_not_connected() {
        let (code, out, _) = call(&["status"]);
        assert_eq!(code, EXIT_NOT_CONNECTED);
        assert!(out.contains("Status: Not connected"));
        assert!(out.contains("No Septuvo service is running"));
        for label in [
            "Tunnel adapter",
            "Server identity verified",
            "Traffic through tunnel",
            "DNS through tunnel",
        ] {
            assert!(out.contains(label), "{label}");
        }
    }

    #[test]
    fn policy_check_valid_file() {
        let file = temp_policy("valid", "mode full-tunnel\ndirect 192.168.50.10/32\n");
        let (code, out, err) = call(&[
            "policy",
            "check",
            &file,
            "--dst",
            "198.51.100.20",
            "--path",
            "down",
        ]);
        assert_eq!(code, EXIT_OK, "{err}");
        assert!(out.contains("Policy is valid."));
        assert!(out.contains("198.51.100.20 with the tunnel down: dropped"));
    }

    #[test]
    fn policy_check_invalid_file() {
        let file = temp_policy("invalid", "mode split\n");
        let (code, out, err) = call(&["policy", "check", &file]);
        assert_eq!(code, EXIT_INVALID);
        assert_eq!(out, "");
        assert!(err.contains("split mode needs at least one tunnel rule"));
    }

    #[test]
    fn policy_check_missing_file_and_bad_options() {
        let (code, _, _) = call(&["policy", "check", "/nonexistent/septuvo.policy"]);
        assert_eq!(code, EXIT_INVALID);
        let file = temp_policy("opts", "mode full-tunnel\n");
        let (code, _, err) = call(&["policy", "check", &file, "--dst", "not-an-ip"]);
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("not an IP address"));
        let (code, _, _) = call(&["policy", "check"]);
        assert_eq!(code, EXIT_USAGE);
    }

    #[test]
    fn hostile_argument_is_not_echoed_with_control_characters() {
        let (code, _, err) = call(&["\u{1b}[31mred\u{7}"]);
        assert_eq!(code, EXIT_USAGE);
        assert!(!err.chars().any(|c| c.is_control() && c != '\n'));
    }
}
