//! The Simple main window's view model (DEFAULT-UI §3.5, §7.1).
//!
//! Plain Rust, built only from the service's latest answer ([`Snapshot`])
//! plus what the app itself is doing (a disconnect in progress) and any
//! notice to show. Nothing is remembered between answers, so a lost service
//! is shown at once and **"Protected" appears only when all four checks
//! pass** (D-020). Every state carries words; colour only repeats them.
//! Everything the service sends is shown as bounded plain text.

use septuvo_cli::service::status_from;
use septuvo_cli::status::Check;
use septuvo_cli::watch::Snapshot;
use septuvo_ipc::Response;

/// How a piece of status looks. Always paired with words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Informational.
    Neutral,
    /// Verified positive status only.
    Good,
    /// Working, attention, limited or unknown.
    Caution,
    /// Failure, lost service.
    Danger,
}

/// What the large shield shows. It is a status indicator, never a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shield {
    /// Verified protection: the only state with a check mark.
    Verified,
    /// Nothing verified, nothing wrong.
    Neutral,
    /// Something is in progress.
    Working,
    /// Attention: for example the kill switch is blocking.
    Warning,
    /// Failure or lost service.
    Error,
}

/// What the main button does when activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    /// Connect the selected connection.
    Connect,
    /// Deliberately disconnect the active session.
    Disconnect,
    /// Nothing (busy or unavailable).
    None,
}

/// The main Connect / Disconnect button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainButton {
    /// Visible text: `Connect`, `Disconnect`, `Disconnecting…`.
    pub label: &'static str,
    /// Accessible name, for example `Disconnect from Home VPN`.
    pub accessible: String,
    /// Whether it can be activated.
    pub enabled: bool,
    /// Shown under the button when disabled: why.
    pub reason: Option<&'static str>,
    /// What activating it does.
    pub action: ButtonAction,
}

/// The profile selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    /// Friendly name of the selected connection, or `No Connections`.
    pub text: String,
    /// Whether it can be opened.
    pub enabled: bool,
}

/// The small line under the supporting text (DEFAULT-UI §7.1, duration line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine {
    /// Dot colour.
    pub tone: Tone,
    /// Text, for example `Not connected`.
    pub text: String,
}

/// Everything the Simple main window shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainView {
    /// Status heading, for example `Service Unavailable`.
    pub heading: &'static str,
    /// Heading colour.
    pub tone: Tone,
    /// Up to two supporting lines.
    pub lines: Vec<String>,
    /// The duration / status line.
    pub status: StatusLine,
    /// The shield.
    pub shield: Shield,
    /// The main button.
    pub button: MainButton,
    /// The profile selector.
    pub selector: Selector,
    /// A notice in the error colour, for example a failed disconnect.
    pub notice: Option<String>,
}

/// What the app itself is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pending {
    /// Nothing.
    #[default]
    None,
    /// A deliberate disconnect was sent; waiting for the service.
    Disconnecting,
}

/// Text from the service: printable characters only, bounded.
#[must_use]
pub fn plain(value: &str) -> String {
    const MAX: usize = 200;
    let mut out: String = value
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX)
        .collect();
    if value.chars().count() > MAX {
        out.push('…');
    }
    out
}

/// True when the service has anything active that a deliberate disconnect
/// must end: rules installed, a wanted connection, or any check passing.
#[must_use]
pub fn needs_disconnect(r: &Response) -> bool {
    let status = status_from(r);
    let any_pass =
        [status.adapter, status.peer, status.reachability, status.dns].contains(&Check::Pass);
    r.field("kill_switch") == Some("on")
        || r.field("state").is_some_and(|s| s != "disconnected")
        || any_pass
}

fn button(
    label: &'static str,
    action: ButtonAction,
    enabled: bool,
    reason: Option<&'static str>,
) -> MainButton {
    MainButton {
        label,
        accessible: label.to_string(),
        enabled,
        reason,
        action,
    }
}

const NO_CONNECTIONS: &str = "No Connections";
const ADD_FIRST: &str = "Add or import a VPN configuration to get started.";

impl MainView {
    /// The view for one service answer, what the app is doing and a notice.
    #[must_use]
    pub fn build(snapshot: &Snapshot, pending: Pending, notice: Option<&str>) -> MainView {
        // Saved connections arrive with the service's connection store (N4-03).
        let selector = Selector {
            text: NO_CONNECTIONS.into(),
            enabled: false,
        };
        let mut v = match snapshot {
            Snapshot::Lost(_) => MainView {
                heading: "Service Unavailable",
                tone: Tone::Danger,
                lines: vec!["Septuvo cannot communicate with its background service.".into()],
                status: StatusLine {
                    tone: Tone::Danger,
                    text: "Status unknown".into(),
                },
                shield: Shield::Error,
                button: button(
                    "Connect",
                    ButtonAction::None,
                    false,
                    Some("The Septuvo service is not running."),
                ),
                selector,
                notice: None,
            },
            Snapshot::Service(r) => Self::from_service(r, selector),
        };
        if pending == Pending::Disconnecting {
            v.heading = "Disconnecting…";
            v.tone = Tone::Caution;
            v.lines = vec!["Safely ending your VPN connection…".into()];
            v.shield = Shield::Working;
            v.button = button("Disconnecting…", ButtonAction::None, false, None);
        }
        v.notice = notice.map(plain);
        v
    }

    fn from_service(r: &Response, selector: Selector) -> MainView {
        let status = status_from(r);
        let state = r.field("state").unwrap_or("");
        let blocking = r.field("kill_switch") == Some("on");
        let not_connected = StatusLine {
            tone: Tone::Neutral,
            text: "Not connected".into(),
        };
        if status.is_connected() {
            // Never before G1. The full-internet copy needs a verified
            // full-tunnel routing mode, which the service does not report yet.
            return MainView {
                heading: "Protected",
                tone: Tone::Good,
                lines: vec!["Your selected networks are protected.".into()],
                status: StatusLine {
                    tone: Tone::Good,
                    text: "Connected".into(),
                },
                shield: Shield::Verified,
                button: button("Disconnect", ButtonAction::Disconnect, true, None),
                selector,
                notice: None,
            };
        }
        match state {
            "error-kill-switch-missing" => MainView {
                heading: "Connection Failed",
                tone: Tone::Danger,
                lines: vec![
                    "Protection could not be enforced: the kill switch is missing.".into(),
                    "Open Help & Diagnostics for more information.".into(),
                ],
                status: not_connected,
                shield: Shield::Error,
                button: button("Disconnect", ButtonAction::Disconnect, true, None),
                selector,
                notice: None,
            },
            "blocking" | "blocking-left-from-earlier-run" if blocking => {
                let second = if r.field("captive_portal") == Some("open") {
                    "A captive-portal sign-in window is open for the sign-in helper only."
                } else if state == "blocking" {
                    "Lab test: this blocks traffic only; there is no tunnel."
                } else {
                    "Rules from an earlier session are still in place. Disconnect to remove them."
                };
                MainView {
                    heading: "Traffic Protected by Kill Switch",
                    tone: Tone::Caution,
                    lines: vec!["Internet access is restricted while the VPN is unavailable.".into(), second.into()],
                    status: not_connected,
                    shield: Shield::Warning,
                    button: button("Disconnect", ButtonAction::Disconnect, true, None),
                    selector,
                    notice: None,
                }
            }
            _ if needs_disconnect(r) => MainView {
                // Something is active that is neither verified nor a known
                // blocking state: never call it protected.
                heading: "Connection Failed",
                tone: Tone::Danger,
                lines: vec![
                    "The VPN connection could not be verified. Your connection is not marked Protected.".into(),
                ],
                status: not_connected,
                shield: Shield::Error,
                button: button("Disconnect", ButtonAction::Disconnect, true, None),
                selector,
                notice: None,
            },
            _ => MainView {
                heading: "VPN Not Yet Available",
                tone: Tone::Neutral,
                lines: vec!["This development build does not establish an encrypted VPN tunnel.".into()],
                status: not_connected,
                shield: Shield::Neutral,
                button: button("Connect", ButtonAction::None, false, Some(ADD_FIRST)),
                selector,
                notice: None,
            },
        }
    }
}

/// Sample service answers, for the design preview and tests. What today's
/// lab service reports.
pub mod samples {
    use septuvo_ipc::Response;

    fn base(state: &str, kill_switch: &str) -> Response {
        with_captive(state, kill_switch, "closed")
    }

    fn with_captive(state: &str, kill_switch: &str, captive: &str) -> Response {
        Response::ok()
            .with("state", state)
            .with("kill_switch", kill_switch)
            .with("interface", "septuvo0")
            .with("adapter", "fail")
            .with("peer", "fail")
            .with("reachability", "unknown")
            .with("dns", "unknown")
            .with("captive_portal", captive)
    }

    /// Service running, nothing active.
    #[must_use]
    pub fn disconnected() -> Response {
        base("disconnected", "off")
    }

    /// Connect asked: the kill switch blocks (no tunnel before G1).
    #[must_use]
    pub fn blocking() -> Response {
        base("blocking", "on")
    }

    /// Blocking, with a captive-portal sign-in window open.
    #[must_use]
    pub fn captive() -> Response {
        with_captive("blocking", "on", "open")
    }

    /// The service wants a connection but its rules are missing.
    #[must_use]
    pub fn kill_switch_missing() -> Response {
        base("error-kill-switch-missing", "off")
    }

    /// All four checks pass (impossible before G1; tests and design only).
    #[must_use]
    pub fn protected() -> Response {
        Response::ok()
            .with("state", "connected")
            .with("kill_switch", "on")
            .with("adapter", "pass")
            .with("peer", "pass")
            .with("reachability", "pass")
            .with("dns", "pass")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(r: Response) -> MainView {
        MainView::build(&Snapshot::Service(r), Pending::None, None)
    }

    #[test]
    fn a_lost_service_is_shown_at_once_and_cannot_be_acted_on() {
        let v = MainView::build(&Snapshot::Lost("x".into()), Pending::None, None);
        assert_eq!(v.heading, "Service Unavailable");
        assert_eq!(v.shield, Shield::Error);
        assert!(!v.button.enabled);
        assert_eq!(v.button.reason, Some("The Septuvo service is not running."));
    }

    #[test]
    fn todays_states_are_never_protected() {
        for r in [
            samples::disconnected(),
            samples::blocking(),
            samples::kill_switch_missing(),
        ] {
            let v = view(r);
            assert_ne!(v.heading, "Protected");
            assert_ne!(v.shield, Shield::Verified);
            assert_ne!(v.status.tone, Tone::Good);
        }
        assert_eq!(
            view(samples::disconnected()).heading,
            "VPN Not Yet Available"
        );
        assert_eq!(
            view(samples::blocking()).heading,
            "Traffic Protected by Kill Switch"
        );
        assert_eq!(
            view(samples::kill_switch_missing()).heading,
            "Connection Failed"
        );
    }

    #[test]
    fn protected_only_when_all_four_checks_pass() {
        let keys = ["adapter", "peer", "reachability", "dns"];
        for n in 0..81u32 {
            let mut r = Response::ok()
                .with("state", "connected")
                .with("kill_switch", "on");
            let mut passes = 0;
            for (i, key) in keys.iter().enumerate() {
                let v = ["pass", "fail", "unknown"]
                    [((n / 3u32.pow(u32::try_from(i).unwrap())) % 3) as usize];
                passes += u32::from(v == "pass");
                r = r.with(key, v);
            }
            let v = view(r);
            assert_eq!(v.heading == "Protected", passes == 4, "combination {n}");
            assert_eq!(v.shield == Shield::Verified, passes == 4, "combination {n}");
        }
    }

    #[test]
    fn the_button_follows_the_real_session() {
        let v = view(samples::disconnected());
        assert_eq!((v.button.label, v.button.enabled), ("Connect", false));
        assert_eq!(v.button.reason, Some(ADD_FIRST));
        assert_eq!(v.selector.text, "No Connections");
        let v = view(samples::blocking());
        assert_eq!(
            (v.button.label, v.button.action),
            ("Disconnect", ButtonAction::Disconnect)
        );
        let v = MainView::build(
            &Snapshot::Service(samples::blocking()),
            Pending::Disconnecting,
            None,
        );
        assert_eq!(
            (v.heading, v.button.label, v.button.enabled),
            ("Disconnecting…", "Disconnecting…", false)
        );
    }

    #[test]
    fn an_unverified_active_session_is_a_failure_not_protection() {
        let v = view(
            Response::ok()
                .with("state", "connected")
                .with("kill_switch", "off")
                .with("adapter", "pass"),
        );
        assert_eq!(v.heading, "Connection Failed");
        assert_eq!(v.button.action, ButtonAction::Disconnect);
    }

    #[test]
    fn what_needs_a_disconnect() {
        assert!(!needs_disconnect(&samples::disconnected()));
        assert!(needs_disconnect(&samples::blocking()));
        assert!(needs_disconnect(&samples::kill_switch_missing()));
        assert!(needs_disconnect(&samples::protected()));
    }

    #[test]
    fn notices_and_service_text_are_plain_and_bounded() {
        let v = MainView::build(
            &Snapshot::Service(samples::blocking()),
            Pending::None,
            Some("bad\u{1b}[31m\nthing"),
        );
        assert_eq!(v.notice.as_deref(), Some("bad[31mthing"));
        assert_eq!(plain(&"x".repeat(500)).chars().count(), 201);
    }
}
