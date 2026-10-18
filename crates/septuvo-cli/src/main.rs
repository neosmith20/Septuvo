//! `septuvo` command-line tool (N1-01, N1-04, N2-02).
//!
//! Development preview only: it has no VPN, tunnel or cryptographic
//! functionality. `status` asks the local service `septuvod` and shows the
//! four-stage status model (D-020); `connect` and `disconnect` ask the
//! service to turn the kill switch on or off; `captive-portal` asks it to
//! open a captive-portal window for the dedicated sign-in helper account
//! only, for at most five minutes (N3-02); `policy check`
//! validates and explains a policy file. The tool itself never changes the
//! system.

use septuvo_cli::{policy_cmd, service, watch};
use septuvo_ipc::Command;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
septuvo - Septuvo VPN command-line tool (development preview)

This build has no VPN functionality: it cannot connect, carry traffic
or protect anything yet.

Usage:
  septuvo help                         Show this help
  septuvo version                      Show the version
  septuvo status [--runtime-dir DIR]   Ask the Septuvo service for the four
                                       connection checks
  septuvo connect [--runtime-dir DIR]  Ask the service to turn the kill
                                       switch on (lab preview, no tunnel)
  septuvo disconnect [--runtime-dir DIR]
                                       Ask the service to turn it off
  septuvo captive-portal [--minutes N] [--network-dns] [--runtime-dir DIR]
                                       For N minutes (1-5, default 5), let
                                       ONLY the sign-in helper account reach
                                       web servers outside the tunnel, to
                                       sign in to a hotel or airport network;
                                       every other program stays blocked.
                                       --network-dns also lets the helper
                                       (only) use the network's own DNS for
                                       this window. Ends by itself.
  septuvo watch [--interval S] [--count N] [--runtime-dir DIR]
                                       Show the status live, refreshed every
                                       S seconds (1-60, default 2) until
                                       Ctrl-C, or N refreshes; read-only
  septuvo policy check FILE [--dst ADDRESS] [--path up|down]
                                       Validate a policy file and explain
                                       what happens to traffic

connect, disconnect and captive-portal need membership of the service's
control group.

Exit codes: 0 success, 1 invalid policy, unreadable file or service
error, 2 usage error, 3 status shows the VPN is not working.
";

/// Exit code for a successful command.
const EXIT_OK: u8 = 0;
/// Exit code for an invalid policy or an unreadable file.
const EXIT_INVALID: u8 = 1;
/// Exit code for a usage error (unknown command or extra arguments).
const EXIT_USAGE: u8 = 2;
/// Exit code when `status` shows the VPN is not fully working.
const EXIT_NOT_CONNECTED: u8 = 3;

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
        [cmd, rest @ ..] if cmd == "status" || cmd == "connect" || cmd == "disconnect" => {
            let Some(runtime_dir) = runtime_dir(rest) else {
                writeln!(
                    err,
                    "septuvo: expected nothing or '--runtime-dir DIR' after '{cmd}'"
                )?;
                writeln!(err, "Run 'septuvo help' for usage.")?;
                return Ok(EXIT_USAGE);
            };
            let command = match cmd.as_str() {
                "status" => Command::Status,
                "connect" => Command::Connect,
                _ => Command::Disconnect,
            };
            let reply = service::ask(&runtime_dir, command);
            out.write_all(reply.status.report(None).as_bytes())?;
            out.write_all(reply.text.as_bytes())?;
            Ok(if command == Command::Status {
                if reply.status.is_connected() {
                    EXIT_OK
                } else {
                    EXIT_NOT_CONNECTED
                }
            } else if reply.ok {
                EXIT_OK
            } else {
                EXIT_INVALID
            })
        }
        [cmd, rest @ ..] if cmd == "captive-portal" => {
            let Some((minutes, network_dns, runtime_dir)) = captive_args(rest) else {
                writeln!(
                    err,
                    "septuvo: usage: septuvo captive-portal [--minutes 1-5] [--network-dns] [--runtime-dir DIR]"
                )?;
                return Ok(EXIT_USAGE);
            };
            let reply = service::ask(
                &runtime_dir,
                Command::Captive {
                    minutes,
                    network_dns,
                },
            );
            out.write_all(reply.text.as_bytes())?;
            Ok(if reply.ok { EXIT_OK } else { EXIT_INVALID })
        }
        [cmd, rest @ ..] if cmd == "watch" => {
            let Some(opts) = watch_args(rest) else {
                writeln!(
                    err,
                    "septuvo: usage: septuvo watch [--interval 1-60] [--count N] [--runtime-dir DIR]"
                )?;
                return Ok(EXIT_USAGE);
            };
            run_watch(&opts, out)?;
            Ok(EXIT_OK)
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

/// `[]` or `["--runtime-dir", DIR]`; anything else is a usage error.
fn runtime_dir(rest: &[String]) -> Option<PathBuf> {
    match rest {
        [] => Some(PathBuf::from(septuvo_ipc::DEFAULT_RUNTIME_DIR)),
        [flag, dir] if flag == "--runtime-dir" && Path::new(dir).is_absolute() => {
            Some(PathBuf::from(dir))
        }
        _ => None,
    }
}

/// `[--minutes N] [--network-dns] [--runtime-dir DIR]` in any order, each
/// at most once. `N` may only shorten the five-minute window.
fn captive_args(rest: &[String]) -> Option<(u8, bool, PathBuf)> {
    let mut minutes = None;
    let mut network_dns = false;
    let mut dir = None;
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        if flag == "--network-dns" {
            if network_dns {
                return None;
            }
            network_dns = true;
            continue;
        }
        let value = it.next()?;
        match flag.as_str() {
            "--minutes" if minutes.is_none() => {
                let n: u8 = value.parse().ok()?;
                if !(1..=septuvo_ipc::MAX_CAPTIVE_MINUTES).contains(&n) || value.len() != 1 {
                    return None;
                }
                minutes = Some(n);
            }
            "--runtime-dir" if dir.is_none() && Path::new(value).is_absolute() => {
                dir = Some(PathBuf::from(value));
            }
            _ => return None,
        }
    }
    Some((
        minutes.unwrap_or(septuvo_ipc::MAX_CAPTIVE_MINUTES),
        network_dns,
        dir.unwrap_or_else(|| PathBuf::from(septuvo_ipc::DEFAULT_RUNTIME_DIR)),
    ))
}

/// Options of `septuvo watch`.
#[derive(Debug, PartialEq, Eq)]
struct WatchOptions {
    interval: u64,
    count: Option<u64>,
    runtime_dir: PathBuf,
    style: watch::Style,
}

/// `[--interval S] [--count N] [--runtime-dir DIR]`, each at most once.
fn watch_args(rest: &[String]) -> Option<WatchOptions> {
    let mut interval = None;
    let mut count = None;
    let mut dir = None;
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let value = it.next()?;
        let number = || {
            (value.len() <= 7
                && value.bytes().all(|b| b.is_ascii_digit())
                && !value.starts_with('0'))
            .then(|| value.parse::<u64>().ok())
            .flatten()
        };
        match flag.as_str() {
            "--interval" if interval.is_none() => {
                interval = Some(number().filter(|n| watch::INTERVAL_RANGE.contains(n))?);
            }
            "--count" if count.is_none() => count = Some(number()?),
            "--runtime-dir" if dir.is_none() && Path::new(value).is_absolute() => {
                dir = Some(PathBuf::from(value));
            }
            _ => return None,
        }
    }
    let terminal =
        io::stdout().is_terminal() && std::env::var("TERM").map_or(true, |t| t != "dumb");
    Some(WatchOptions {
        interval: interval.unwrap_or(watch::DEFAULT_INTERVAL),
        count,
        runtime_dir: dir.unwrap_or_else(|| PathBuf::from(septuvo_ipc::DEFAULT_RUNTIME_DIR)),
        style: watch::Style {
            in_place: terminal,
            color: terminal && std::env::var_os("NO_COLOR").is_none(),
        },
    })
}

/// Refreshes until stopped (Ctrl-C) or `count` frames were shown. Every
/// frame comes from a fresh answer: nothing is carried over.
fn run_watch(opts: &WatchOptions, out: &mut impl Write) -> io::Result<()> {
    let started = std::time::Instant::now();
    let mut refresh: u64 = 0;
    loop {
        refresh += 1;
        let snap = watch::snapshot(&opts.runtime_dir);
        let elapsed = started.elapsed().as_secs();
        out.write_all(watch::frame(&snap, refresh, elapsed, opts.interval, opts.style).as_bytes())?;
        out.flush()?;
        if opts.count.is_some_and(|n| refresh >= n) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_secs(opts.interval));
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
        let (code, out, err) = call(&["reboot"]);
        assert_eq!(code, EXIT_USAGE);
        assert_eq!(out, "");
        assert!(err.contains("unknown command"));
    }

    #[test]
    fn extra_arguments_are_rejected() {
        let (code, _, _) = call(&["version", "extra"]);
        assert_eq!(code, EXIT_USAGE);
        for args in [
            &["status", "extra"][..],
            &["connect", "--runtime-dir"],
            &["disconnect", "--runtime-dir", "relative/dir"],
        ] {
            assert_eq!(call(args).0, EXIT_USAGE, "{args:?}");
        }
    }

    #[test]
    fn captive_portal_arguments_only_shorten_the_window() {
        let s = |v: &[&str]| v.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert_eq!(
            captive_args(&s(&[])),
            Some((5, false, PathBuf::from("/run/septuvo")))
        );
        assert_eq!(
            captive_args(&s(&[
                "--runtime-dir",
                "/tmp/x",
                "--network-dns",
                "--minutes",
                "1"
            ])),
            Some((1, true, PathBuf::from("/tmp/x")))
        );
        for bad in [
            &["--minutes", "6"][..],
            &["--minutes", "0"],
            &["--minutes", "05"],
            &["--minutes", "+5"],
            &["--minutes", "-1"],
            &["--minutes"],
            &["--minutes", "1", "--minutes", "2"],
            &["--runtime-dir", "relative"],
            &["--forever", "1"],
            &["--network-dns", "--network-dns"],
            &["--network-dns=yes"],
        ] {
            assert_eq!(captive_args(&s(bad)), None, "{bad:?}");
            let mut args = vec!["captive-portal"];
            args.extend_from_slice(bad);
            assert_eq!(call(&args).0, EXIT_USAGE, "{bad:?}");
        }
    }

    #[test]
    fn captive_portal_without_a_service_fails_cleanly() {
        let (code, out, _) = call(&[
            "captive-portal",
            "--runtime-dir",
            "/nonexistent-septuvo-cli",
        ]);
        assert_eq!(code, EXIT_INVALID);
        assert!(out.contains("No Septuvo service is running"));
    }

    #[test]
    fn watch_arguments_are_strict() {
        let s = |v: &[&str]| v.iter().map(ToString::to_string).collect::<Vec<_>>();
        let o = watch_args(&s(&[
            "--interval",
            "1",
            "--count",
            "3",
            "--runtime-dir",
            "/tmp/x",
        ]))
        .unwrap();
        assert_eq!(
            (o.interval, o.count, o.runtime_dir),
            (1, Some(3), PathBuf::from("/tmp/x"))
        );
        let d = watch_args(&s(&[])).unwrap();
        assert_eq!((d.interval, d.count), (watch::DEFAULT_INTERVAL, None));
        for bad in [
            &["--interval", "0"][..],
            &["--interval", "61"],
            &["--interval", "01"],
            &["--interval", "-1"],
            &["--interval", "1.5"],
            &["--count", "0"],
            &["--count", "99999999"],
            &["--interval"],
            &["--interval", "2", "--interval", "3"],
            &["--runtime-dir", "relative"],
            &["--forever", "1"],
        ] {
            assert_eq!(watch_args(&s(bad)), None, "{bad:?}");
            let mut args = vec!["watch"];
            args.extend_from_slice(bad);
            assert_eq!(call(&args).0, EXIT_USAGE, "{bad:?}");
        }
    }

    #[test]
    fn watch_without_a_service_shows_service_lost_every_refresh() {
        let (code, out, _) = call(&[
            "watch",
            "--interval",
            "1",
            "--count",
            "2",
            "--runtime-dir",
            "/nonexistent-septuvo-watch",
        ]);
        assert_eq!(code, EXIT_OK);
        assert_eq!(out.matches("SERVICE LOST").count(), 2);
        assert_eq!(out.matches("refresh ").count(), 2);
        assert!(!out.contains("Status: Connected"));
        assert!(!out.contains('\x1b'), "not a terminal: plain text");
    }

    #[test]
    fn connect_without_a_service_fails_cleanly() {
        let (code, out, _) = call(&["connect", "--runtime-dir", "/nonexistent-septuvo-cli"]);
        assert_eq!(code, EXIT_INVALID);
        assert!(out.contains("No Septuvo service is running"));
        assert!(out.contains("Status: Not connected"));
    }

    /// A stand-in service on a temporary socket: the CLI must show exactly
    /// what it reports, and never "connected" while peer authentication fails.
    #[test]
    fn status_shows_what_the_service_reports() {
        use std::os::unix::net::UnixListener;
        let dir = std::env::temp_dir().join(format!("septuvo-cli-svc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let listener = UnixListener::bind(septuvo_ipc::status_socket(&dir)).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let req = septuvo_ipc::read_message(&mut stream, 64, b"\n").unwrap();
            assert_eq!(req, b"SEPTUVO/1 STATUS\n");
            let reply = septuvo_ipc::Response::ok()
                .with("adapter", "pass")
                .with("peer", "fail")
                .with("state", "blocking")
                .with("kill_switch", "on");
            stream.write_all(reply.encode().as_bytes()).unwrap();
        });
        let (code, out, _) = call(&["status", "--runtime-dir", dir.to_str().unwrap()]);
        server.join().unwrap();
        assert_eq!(code, EXIT_NOT_CONNECTED);
        assert!(out.contains("Tunnel adapter             ok"));
        assert!(out.contains("Server identity verified   failed"));
        assert!(out.contains("Kill switch: on"));
        assert!(out.contains("Status: Not connected"));
        std::fs::remove_dir_all(&dir).unwrap();
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
        let (code, out, _) = call(&["status", "--runtime-dir", "/nonexistent-septuvo-cli"]);
        assert_eq!(code, EXIT_NOT_CONNECTED);
        assert!(out.contains("/nonexistent-septuvo-cli"));
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
