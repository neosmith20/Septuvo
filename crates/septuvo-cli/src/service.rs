//! Talking to the local Septuvo service `septuvod` (N2-02).
//!
//! The CLI never changes the network itself: it asks the privileged
//! service, which decides. Status is read from the service's answer and
//! shown with the four-stage model (D-020); anything the service does not
//! report as passed is shown as not passed.

use crate::status::{Check, ConnectionStatus};
use septuvo_ipc::{self as ipc, ClientError, Command, Outcome, Request, Response};
use std::fmt::Write as _;
use std::path::Path;

fn check(value: Option<&str>) -> Check {
    match value {
        Some("pass") => Check::Pass,
        Some("fail") => Check::Fail,
        _ => Check::Unknown,
    }
}

/// The four stages as reported by the service.
#[must_use]
pub fn status_from(response: &Response) -> ConnectionStatus {
    ConnectionStatus {
        adapter: check(response.field("adapter")),
        peer: check(response.field("peer")),
        reachability: check(response.field("reachability")),
        dns: check(response.field("dns")),
    }
}

/// Extra lines shown under the four stages: service state, kill switch and
/// the service's own notes.
#[must_use]
pub fn details(response: &Response) -> String {
    let mut s = String::new();
    for (key, label) in [
        ("message", "Service"),
        ("state", "Service state"),
        ("failure", "Failure"),
        ("connection", "Connection"),
        ("kill_switch", "Kill switch"),
        ("dns_protection", "DNS protection"),
        ("interface", "Tunnel interface"),
        ("dns_policy", "DNS policy"),
        ("resolver_file", "Resolver file"),
        ("note", "Note"),
    ] {
        if let Some(value) = response.field(key) {
            let _ = writeln!(s, "{label}: {value}");
        }
    }
    s.push_str(&captive_lines(response));
    s
}

/// The captive-portal lines: always the state; while open, a warning that
/// cannot be missed, who the window applies to, the DNS used and the time
/// left.
#[must_use]
pub fn captive_lines(response: &Response) -> String {
    let mut s = String::new();
    match response.field("captive_portal") {
        Some("open") => {
            let left = match response.field("captive_remaining") {
                Some(n) if n.bytes().all(|b| b.is_ascii_digit()) && !n.is_empty() => {
                    let n: u64 = n.parse().unwrap_or(0);
                    format!("{}:{:02} left", n / 60, n % 60)
                }
                _ => "time left unknown; it still ends on time".to_string(),
            };
            let _ = writeln!(s, "Captive portal: OPEN ({left})");
            let warning = response
                .field("captive_warning")
                .unwrap_or("traffic is leaving outside the tunnel");
            let _ = writeln!(s, "WARNING: {warning}");
            if let Some(dns) = response.field("captive_dns") {
                let _ = writeln!(s, "Captive portal DNS: {dns}");
            }
            if let Some(dns) = response.field("captive_resolvers") {
                let _ = writeln!(s, "Captive portal: {dns}");
            }
        }
        Some("closed") => {
            let _ = writeln!(s, "Captive portal: closed");
        }
        Some("not-configured") => {
            let _ = writeln!(s, "Captive portal: not configured (no helper account)");
        }
        _ => {}
    }
    if let Some(helper) = response.field("captive_helper") {
        let _ = writeln!(s, "Captive portal helper account: {helper}");
    }
    if let Some(n) = response.field("captive_windows_left") {
        let _ = writeln!(s, "Captive portal windows left this connection: {n}");
    }
    s
}

/// The result of asking the service, ready to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// True if the service did what was asked.
    pub ok: bool,
    /// The four stages (unknown if the service could not be asked).
    pub status: ConnectionStatus,
    /// Text to print after the stage report.
    pub text: String,
}

/// Sends `command` to the service under `runtime_dir`: status over the
/// status socket, changes over the control socket.
#[must_use]
pub fn ask(runtime_dir: &Path, command: Command) -> Reply {
    reply(runtime_dir, call(runtime_dir, &Request::new(command)))
}

/// Sends `request` to the right socket under `runtime_dir`.
///
/// # Errors
/// See [`ClientError`].
pub fn call(runtime_dir: &Path, request: &Request) -> Result<Response, ClientError> {
    let socket = if request.command.needs_control() {
        ipc::control_socket(runtime_dir)
    } else {
        ipc::status_socket(runtime_dir)
    };
    ipc::send(&socket, request)
}

/// A printable reply for a service answer (or the failure to get one).
#[must_use]
pub fn reply(runtime_dir: &Path, answer: Result<Response, ClientError>) -> Reply {
    match answer {
        Ok(response) => {
            let status = status_from(&response);
            let mut text = details(&response);
            let ok = match &response.outcome {
                Outcome::Ok => true,
                Outcome::Err { code, message } => {
                    let _ = writeln!(text, "Error ({code}): {message}");
                    false
                }
            };
            Reply { ok, status, text }
        }
        Err(e) => {
            let text = match e {
                ClientError::NotRunning => format!(
                    "No Septuvo service is running (looked in {}): nothing could be checked.\n",
                    runtime_dir.display()
                ),
                other => format!("{other}\n"),
            };
            Reply {
                ok: false,
                status: ConnectionStatus::UNKNOWN,
                text,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_service_fields_to_stages() {
        let r = Response::ok()
            .with("adapter", "pass")
            .with("peer", "fail")
            .with("reachability", "unknown")
            .with("dns", "PASS");
        let s = status_from(&r);
        assert_eq!(
            (s.adapter, s.peer, s.reachability, s.dns),
            (Check::Pass, Check::Fail, Check::Unknown, Check::Unknown),
            "only the exact word 'pass' counts as passed"
        );
        assert!(!s.is_connected());
    }

    #[test]
    fn missing_fields_are_unknown_never_pass() {
        let s = status_from(&Response::ok());
        assert_eq!(s, ConnectionStatus::UNKNOWN);
    }

    #[test]
    fn details_show_state_and_notes() {
        let r = Response::ok()
            .with("state", "blocking")
            .with("kill_switch", "on")
            .with(
                "note",
                "peer authentication is not available before the G1 security review",
            );
        let d = details(&r);
        assert!(d.contains("Service state: blocking"));
        assert!(d.contains("Kill switch: on"));
        assert!(d.contains("G1"));
    }

    #[test]
    fn captive_portal_lines_warn_and_count_down() {
        let open = Response::ok()
            .with("captive_portal", "open")
            .with("captive_remaining", "299")
            .with("captive_warning", "outside the tunnel");
        let d = details(
            &open
                .with("captive_helper", "uid 995")
                .with("captive_windows_left", "2"),
        );
        assert!(d.contains("Captive portal: OPEN (4:59 left)"));
        assert!(d.contains("WARNING: outside the tunnel"));
        assert!(d.contains("Captive portal helper account: uid 995"));
        assert!(d.contains("Captive portal windows left this connection: 2"));
        let unknown = Response::ok()
            .with("captive_portal", "open")
            .with("captive_remaining", "unknown");
        let d = details(&unknown);
        assert!(d.contains("time left unknown; it still ends on time"));
        assert!(
            d.contains("WARNING:"),
            "a warning even without the service's text"
        );
        assert!(
            details(&Response::ok().with("captive_portal", "closed"))
                .contains("Captive portal: closed")
        );
        assert!(!details(&Response::ok()).contains("Captive"));
    }

    #[test]
    fn missing_service_reports_not_running() {
        let reply = ask(Path::new("/nonexistent-septuvo-runtime"), Command::Status);
        assert!(!reply.ok);
        assert_eq!(reply.status, ConnectionStatus::UNKNOWN);
        assert!(reply.text.contains("No Septuvo service is running"));
    }
}
