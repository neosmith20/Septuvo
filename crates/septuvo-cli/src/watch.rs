//! `septuvo watch`: a live status view in the terminal (N3-04).
//!
//! Every refresh asks the service for its status over the **status socket**
//! (read-only) and draws a complete frame from that answer alone. Nothing is
//! remembered between refreshes, so:
//!
//! - **a lost service is shown at once**, on the very next refresh, never as
//!   the last known state;
//! - **"Connected" appears only when all four checks pass** in that answer
//!   (D-020), through the same [`ConnectionStatus`] logic as `septuvo status`.
//!
//! On a terminal the frame is redrawn in place (ANSI: cursor home and clear),
//! with colour unless `NO_COLOR` is set. When the output is not a terminal
//! (a file, a pipe, a log), frames are printed one after another as plain
//! text with a separator line. Standard library only. The same status
//! interface would serve a desktop app later (a GUI toolkit is a separate
//! dependency decision, D-015).

use crate::service::{details, status_from};
use crate::status::ConnectionStatus;
use septuvo_ipc::{self as ipc, ClientError, Command, Outcome, Response};
use std::fmt::Write as _;
use std::path::Path;

/// Default seconds between refreshes.
pub const DEFAULT_INTERVAL: u64 = 2;
/// Shortest and longest accepted refresh interval, in seconds.
pub const INTERVAL_RANGE: std::ops::RangeInclusive<u64> = 1..=60;

/// What one refresh found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Snapshot {
    /// The service answered.
    Service(Response),
    /// The service could not be asked: not running, not permitted, or an
    /// invalid answer. Nothing is known.
    Lost(String),
}

/// Asks the service once, over the read-only status socket.
#[must_use]
pub fn snapshot(runtime_dir: &Path) -> Snapshot {
    match ipc::request(&ipc::status_socket(runtime_dir), Command::Status) {
        Ok(response) => Snapshot::Service(response),
        Err(ClientError::NotRunning) => Snapshot::Lost(format!(
            "no Septuvo service is running (looked in {})",
            runtime_dir.display()
        )),
        Err(e) => Snapshot::Lost(e.to_string()),
    }
}

/// How a frame is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    /// Redraw in place (a terminal).
    pub in_place: bool,
    /// Use colour (a terminal, and `NO_COLOR` not set).
    pub color: bool,
}

impl Style {
    /// Plain text, for files and pipes.
    pub const PLAIN: Style = Style {
        in_place: false,
        color: false,
    };
}

const RESET: &str = "\x1b[0m";
const RED_BOLD: &str = "\x1b[1;31m";
const YELLOW: &str = "\x1b[33m";
const GREEN_BOLD: &str = "\x1b[1;32m";

fn paint(style: Style, code: &str, line: &str) -> String {
    if style.color {
        format!("{code}{line}{RESET}")
    } else {
        line.to_string()
    }
}

/// One complete frame, built only from `snap`.
#[must_use]
pub fn frame(
    snap: &Snapshot,
    refresh: u64,
    elapsed_secs: u64,
    interval: u64,
    style: Style,
) -> String {
    let mut s = String::new();
    if style.in_place {
        s.push_str("\x1b[H\x1b[2J");
    } else if refresh > 1 {
        s.push_str("----\n");
    }
    let _ = writeln!(
        s,
        "Septuvo watch: refresh {refresh}, {elapsed_secs}s, every {interval}s (Ctrl-C to stop)"
    );
    s.push_str("Development preview: no tunnel exists before the G1 security review.\n\n");
    let (status, body) = match snap {
        Snapshot::Lost(reason) => {
            let line = format!(
                "SERVICE LOST: {reason}. Nothing is known; nothing below is a last known state."
            );
            let _ = writeln!(s, "{}\n", paint(style, RED_BOLD, &line));
            (ConnectionStatus::UNKNOWN, String::new())
        }
        Snapshot::Service(response) => {
            let mut body = details(response);
            if let Outcome::Err { code, message } = &response.outcome {
                let _ = writeln!(body, "Service error ({code}): {message}");
            }
            (status_from(response), body)
        }
    };
    for line in status.report(None).lines() {
        let painted = if line.starts_with("Status: Connected") {
            paint(style, GREEN_BOLD, line)
        } else if line.starts_with("Status: ") {
            paint(style, YELLOW, line)
        } else {
            line.to_string()
        };
        let _ = writeln!(s, "{painted}");
    }
    if !body.is_empty() {
        s.push('\n');
        for line in body.lines() {
            let painted = if line.starts_with("WARNING") || line.starts_with("Captive portal: OPEN")
            {
                paint(style, RED_BOLD, line)
            } else {
                line.to_string()
            };
            let _ = writeln!(s, "{painted}");
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::{Check, ConnectionStatus};

    fn service(fields: &[(&str, &str)]) -> Snapshot {
        let mut r = Response::ok();
        for (k, v) in fields {
            r = r.with(k, v);
        }
        Snapshot::Service(r)
    }

    fn blocking() -> Snapshot {
        service(&[
            ("state", "blocking"),
            ("kill_switch", "on"),
            ("adapter", "pass"),
            ("peer", "fail"),
            ("reachability", "unknown"),
            ("dns", "unknown"),
            ("dns_policy", "only through the tunnel to 10.77.0.1"),
            ("resolver_file", "septuvo"),
            ("captive_portal", "closed"),
        ])
    }

    #[test]
    fn a_lost_service_is_shown_at_once_with_nothing_remembered() {
        let f = frame(
            &Snapshot::Lost("no Septuvo service is running".into()),
            7,
            12,
            2,
            Style::PLAIN,
        );
        assert!(f.contains("SERVICE LOST: no Septuvo service is running"));
        assert!(f.contains("Status: Not connected"));
        for stale in [
            "Kill switch",
            "Service state",
            "Captive portal",
            "DNS policy",
        ] {
            assert!(!f.contains(stale), "a lost frame must not show {stale}");
        }
        for label in ["Tunnel adapter", "Server identity verified"] {
            assert!(
                f.contains(&format!("{label:<26} unknown")),
                "{label} unknown"
            );
        }
    }

    #[test]
    fn the_frame_follows_only_the_current_answer() {
        let on = frame(&blocking(), 1, 0, 2, Style::PLAIN);
        assert!(on.contains("Kill switch: on"));
        assert!(on.contains("Service state: blocking"));
        assert!(on.contains("DNS policy: only through the tunnel to 10.77.0.1"));
        assert!(on.contains("Resolver file: septuvo"));
        assert!(on.contains("Captive portal: closed"));
        assert!(on.contains("Server identity verified   failed"));
        let off = frame(
            &service(&[
                ("state", "disconnected"),
                ("kill_switch", "off"),
                ("resolver_file", "system"),
            ]),
            2,
            2,
            2,
            Style::PLAIN,
        );
        assert!(off.contains("Kill switch: off") && !off.contains("Kill switch: on"));
        assert!(off.starts_with("----\n"), "plain frames are separated");
    }

    #[test]
    fn captive_countdown_and_warning_are_shown() {
        let f = frame(
            &service(&[
                ("kill_switch", "on"),
                ("captive_portal", "open"),
                ("captive_remaining", "125"),
                (
                    "captive_warning",
                    "captive-portal window: the sign-in helper account may reach web servers",
                ),
                ("captive_dns", "helper: encrypted DNS only"),
                ("captive_windows_left", "2"),
            ]),
            3,
            4,
            1,
            Style::PLAIN,
        );
        assert!(f.contains("Captive portal: OPEN (2:05 left)"));
        assert!(f.contains("WARNING: captive-portal window"));
        assert!(f.contains("windows left this connection: 2"));
    }

    #[test]
    fn connected_only_when_all_four_checks_pass_in_every_combination() {
        let states = [
            ("unknown", Check::Unknown),
            ("fail", Check::Fail),
            ("pass", Check::Pass),
        ];
        for (a, ac) in states {
            for (p, pc) in states {
                for (r, rc) in states {
                    for (d, dc) in states {
                        let snap = service(&[
                            ("adapter", a),
                            ("peer", p),
                            ("reachability", r),
                            ("dns", d),
                        ]);
                        let f = frame(&snap, 1, 0, 2, Style::PLAIN);
                        let expected = ConnectionStatus {
                            adapter: ac,
                            peer: pc,
                            reachability: rc,
                            dns: dc,
                        }
                        .is_connected();
                        assert_eq!(f.contains("Status: Connected"), expected, "{a} {p} {r} {d}");
                        assert_eq!(expected, [a, p, r, d] == ["pass"; 4]);
                    }
                }
            }
        }
    }

    #[test]
    fn service_errors_are_shown() {
        let f = frame(
            &Snapshot::Service(Response::error(
                "not_permitted",
                "this command needs the control socket",
            )),
            1,
            0,
            2,
            Style::PLAIN,
        );
        assert!(f.contains("Service error (not_permitted)"));
        assert!(f.contains("Status: Not connected"));
    }

    #[test]
    fn terminal_frames_redraw_in_place_and_plain_frames_have_no_escapes() {
        let term = Style {
            in_place: true,
            color: true,
        };
        let f = frame(&blocking(), 2, 2, 2, term);
        assert!(f.starts_with("\x1b[H\x1b[2J"), "cursor home and clear");
        assert!(f.contains("\x1b[33mStatus: Not connected"));
        let lost = frame(&Snapshot::Lost("gone".into()), 3, 4, 2, term);
        assert!(lost.contains("\x1b[1;31mSERVICE LOST"));
        let no_color = frame(
            &blocking(),
            2,
            2,
            2,
            Style {
                in_place: true,
                color: false,
            },
        );
        assert_eq!(
            no_color.matches('\x1b').count(),
            2,
            "only the redraw sequence"
        );
        for snap in [blocking(), Snapshot::Lost("x".into())] {
            assert!(!frame(&snap, 2, 2, 2, Style::PLAIN).contains('\x1b'));
        }
    }

    #[test]
    fn a_missing_service_gives_a_lost_snapshot() {
        let s = snapshot(Path::new("/nonexistent-septuvo-watch"));
        assert!(matches!(s, Snapshot::Lost(ref r) if r.contains("no Septuvo service is running")));
    }
}
