//! The view model: what the status page shows, in plain Rust.
//!
//! Built from one answer of the service ([`Snapshot`]) and nothing else, so
//! a lost service is shown at once, never as the last known state (as
//! `septuvo watch`, N3-04). The overall headline comes from the shared
//! four-stage model ([`ConnectionStatus`], D-020): **"Connected" only when
//! all four checks pass.** Everything the service sends is treated as
//! untrusted text: it is shown, never interpreted (ADV-10).
//!
//! No UI code here, so every state is unit-tested without a display.

use septuvo_cli::service::status_from;
use septuvo_cli::status::{Check, ConnectionStatus, Overall, Stage};
use septuvo_cli::watch::Snapshot;
use septuvo_ipc::Response;

/// How a piece of status should look. Every tone is always paired with
/// words: colour is never the only signal (brand rule, accessibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Informational.
    Neutral,
    /// Passed or safe.
    Good,
    /// Needs attention.
    Caution,
    /// Failed, unsafe or unknown protection.
    Danger,
}

/// The large status line at the top of the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Headline {
    /// For example "Not connected".
    pub title: String,
    /// One or two plain sentences.
    pub detail: String,
    /// How it looks.
    pub tone: Tone,
}

/// One of the four connection checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckRow {
    /// For example "Server identity verified".
    pub label: &'static str,
    /// "Passed", "Failed" or "Unknown".
    pub state: &'static str,
    /// What the check means, in plain language.
    pub help: &'static str,
    /// How it looks.
    pub tone: Tone,
}

/// A labelled value, for example "Kill switch: On (blocking)".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    /// The label.
    pub label: &'static str,
    /// The value, as plain text.
    pub value: String,
    /// How it looks.
    pub tone: Tone,
}

/// An open captive-portal window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captive {
    /// For example "4:59 left".
    pub left: String,
    /// The service's warning, as sent.
    pub warning: String,
    /// Which DNS the helper uses, if reported.
    pub dns: Option<String>,
}

/// Everything the status page shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    /// The headline.
    pub headline: Headline,
    /// Set when the service could not be asked: why.
    pub service_lost: Option<String>,
    /// The four checks, in order.
    pub checks: Vec<CheckRow>,
    /// Protection details reported by the service.
    pub details: Vec<Detail>,
    /// An open captive-portal window.
    pub captive: Option<Captive>,
}

/// The notice shown on every page until a real tunnel exists (G1).
pub const PREVIEW_NOTICE: &str = "Development preview: Septuvo VPN does not create a tunnel yet. \
     It can block traffic (kill switch) but cannot protect it until the Septuvo Protocol passes its \
     independent security review.";

/// Plain-language help for each check.
#[must_use]
pub fn help(stage: Stage) -> &'static str {
    match stage {
        Stage::Adapter => {
            "The virtual network adapter exists. On its own this proves nothing about protection."
        }
        Stage::PeerAuthenticated => {
            "The Septuvo Server proved its identity. Not available before the security review."
        }
        Stage::Reachability => "A test packet went through the tunnel and back.",
        Stage::Dns => "A name lookup went through the tunnel's own DNS resolver.",
    }
}

fn check_text(c: Check) -> (&'static str, Tone) {
    match c {
        Check::Pass => ("Passed", Tone::Good),
        Check::Fail => ("Failed", Tone::Danger),
        Check::Unknown => ("Unknown", Tone::Neutral),
    }
}

fn checks(status: ConnectionStatus) -> Vec<CheckRow> {
    Stage::ALL
        .into_iter()
        .map(|stage| {
            let (state, tone) = check_text(status.get(stage));
            CheckRow {
                label: stage.label(),
                state,
                help: help(stage),
                tone,
            }
        })
        .collect()
}

/// Plain text for an untrusted value: printable characters only, bounded.
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

fn service_state(state: &str) -> (String, Tone) {
    match state {
        "blocking" => (
            "Blocking all traffic: kill switch on, no tunnel yet".into(),
            Tone::Caution,
        ),
        "blocking-left-from-earlier-run" => (
            "Blocking (rules left from an earlier run)".into(),
            Tone::Caution,
        ),
        "error-kill-switch-missing" => ("Error: the kill switch is missing".into(), Tone::Danger),
        "disconnected" => ("Disconnected".into(), Tone::Neutral),
        other => (plain(other), Tone::Neutral),
    }
}

fn details(r: &Response) -> Vec<Detail> {
    let mut out = Vec::new();
    if let Some(state) = r.field("state") {
        let (value, tone) = service_state(state);
        out.push(Detail {
            label: "Service state",
            value,
            tone,
        });
    }
    if let Some(k) = r.field("kill_switch") {
        let (value, tone) = match k {
            "on" => (
                "On: traffic outside the tunnel is blocked".to_string(),
                Tone::Good,
            ),
            "off" => ("Off: nothing is blocked".to_string(), Tone::Caution),
            other => (plain(other), Tone::Neutral),
        };
        out.push(Detail {
            label: "Kill switch",
            value,
            tone,
        });
    }
    if let Some(v) = r.field("dns_policy") {
        out.push(Detail {
            label: "DNS policy",
            value: plain(v),
            tone: Tone::Neutral,
        });
    }
    if let Some(v) = r.field("resolver_file") {
        let value = match v {
            "septuvo" => "Septuvo's resolver settings are in place".to_string(),
            "system" => "The system's own resolver settings".to_string(),
            other => plain(other),
        };
        out.push(Detail {
            label: "Resolver file",
            value,
            tone: Tone::Neutral,
        });
    }
    if let Some(v) = r.field("interface") {
        out.push(Detail {
            label: "Tunnel interface",
            value: plain(v),
            tone: Tone::Neutral,
        });
    }
    out
}

fn captive(r: &Response) -> Option<Captive> {
    if r.field("captive_portal") != Some("open") {
        return None;
    }
    let left = match r
        .field("captive_remaining")
        .and_then(|n| n.parse::<u64>().ok())
    {
        Some(n) => format!("{}:{:02} left", n / 60, n % 60),
        None => "time left unknown; it still ends on time".to_string(),
    };
    let warning = plain(
        r.field("captive_warning")
            .unwrap_or("traffic is leaving outside the tunnel"),
    );
    Some(Captive {
        left,
        warning,
        dns: r.field("captive_dns").map(plain),
    })
}

impl StatusView {
    /// The view for one snapshot.
    #[must_use]
    pub fn from_snapshot(snapshot: &Snapshot) -> StatusView {
        match snapshot {
            Snapshot::Lost(why) => StatusView {
                headline: Headline {
                    title: "Service not running".into(),
                    detail: "Septuvo VPN cannot reach its service, so nothing is known: \
                             treat this computer as unprotected."
                        .into(),
                    tone: Tone::Danger,
                },
                service_lost: Some(plain(why)),
                checks: checks(ConnectionStatus::UNKNOWN),
                details: Vec::new(),
                captive: None,
            },
            Snapshot::Service(r) => {
                let status = status_from(r);
                let headline = match status.overall() {
                    Overall::Connected => Headline {
                        title: "Connected".into(),
                        detail: "All four checks passed.".into(),
                        tone: Tone::Good,
                    },
                    Overall::NotConnected(stage) => Headline {
                        title: "Not connected".into(),
                        detail: format!("Waiting for: {}.", stage.label().to_lowercase()),
                        tone: Tone::Caution,
                    },
                    Overall::Inconsistent(stage) => Headline {
                        title: "Not connected".into(),
                        detail: format!(
                            "The service reported a later check as passed while \"{}\" has not, \
                             so this is not trusted.",
                            stage.label().to_lowercase()
                        ),
                        tone: Tone::Danger,
                    },
                };
                let mut headline = headline;
                match r.field("kill_switch") {
                    Some("on") => headline
                        .detail
                        .push_str(" The kill switch is on: traffic outside the tunnel is blocked."),
                    Some("off") => headline
                        .detail
                        .push_str(" The kill switch is off: nothing is protected or blocked."),
                    _ => {}
                }
                StatusView {
                    headline,
                    service_lost: None,
                    checks: checks(status),
                    details: details(r),
                    captive: captive(r),
                }
            }
        }
    }

    /// True only when the headline says "Connected" (all four checks passed).
    #[must_use]
    pub fn claims_connected(&self) -> bool {
        self.headline.title == "Connected"
    }
}

/// Sample answers for the design preview (`examples/preview.rs`) and tests:
/// what today's lab service reports. Labelled as samples wherever shown.
pub mod samples {
    use septuvo_ipc::Response;

    fn base(state: &str, kill_switch: &str) -> Response {
        Response::ok()
            .with("state", state)
            .with("kill_switch", kill_switch)
            .with("interface", "septuvo0")
            .with("adapter", "fail")
            .with("peer", "fail")
            .with("reachability", "unknown")
            .with("dns", "unknown")
            .with("dns_policy", "tunnel resolvers only: 10.77.0.1")
            .with(
                "resolver_file",
                if kill_switch == "on" {
                    "septuvo"
                } else {
                    "system"
                },
            )
            .with(
                "note",
                "peer authentication is not available before the G1 security review",
            )
    }

    /// Service running, not connected.
    #[must_use]
    pub fn disconnected() -> Response {
        base("disconnected", "off").with("captive_portal", "closed")
    }

    /// Connect asked: the kill switch blocks everything (no tunnel before G1).
    #[must_use]
    pub fn blocking() -> Response {
        base("blocking", "on").with("captive_portal", "closed")
    }

    /// A captive-portal window is open.
    #[must_use]
    pub fn captive() -> Response {
        base("blocking", "on")
            .with("captive_portal", "open")
            .with("captive_remaining", "272")
            .with(
                "captive_warning",
                "captive-portal window: the sign-in helper account may reach web servers \
                 (TCP 80/443) OUTSIDE the tunnel, unprotected; every other program stays blocked",
            )
            .with("captive_dns", "trusted DNS over HTTPS for the helper only")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(r: Response) -> StatusView {
        StatusView::from_snapshot(&Snapshot::Service(r))
    }

    #[test]
    fn a_lost_service_is_shown_at_once_with_nothing_known() {
        let v = StatusView::from_snapshot(&Snapshot::Lost("no Septuvo service is running".into()));
        assert_eq!(v.headline.title, "Service not running");
        assert_eq!(v.headline.tone, Tone::Danger);
        assert_eq!(
            v.service_lost.as_deref(),
            Some("no Septuvo service is running")
        );
        assert!(v.checks.iter().all(|c| c.state == "Unknown"));
        assert!(v.details.is_empty() && v.captive.is_none());
    }

    #[test]
    fn todays_service_is_never_connected() {
        for r in [
            samples::disconnected(),
            samples::blocking(),
            samples::captive(),
        ] {
            let v = view(r);
            assert!(!v.claims_connected());
            assert_eq!(v.headline.title, "Not connected");
            assert_eq!(v.checks.len(), 4);
            assert_eq!(v.checks[1].state, "Failed");
            assert!(
                v.headline.detail.contains("kill switch"),
                "{}",
                v.headline.detail
            );
        }
    }

    #[test]
    fn connected_only_when_all_four_pass_in_order() {
        let all = ["adapter", "peer", "reachability", "dns"];
        // Every combination of pass/fail/unknown for the four checks (81).
        for n in 0..81u32 {
            let mut r = Response::ok();
            let mut passes = 0;
            for (i, key) in all.iter().enumerate() {
                let v = ["pass", "fail", "unknown"]
                    [((n / 3u32.pow(u32::try_from(i).unwrap())) % 3) as usize];
                passes += u32::from(v == "pass");
                r = r.with(key, v);
            }
            let v = view(r);
            assert_eq!(v.claims_connected(), passes == 4, "combination {n}");
        }
        let v = view(Response::ok().with("adapter", "fail").with("peer", "pass"));
        assert_eq!(
            v.headline.tone,
            Tone::Danger,
            "out-of-order passes are not trusted"
        );
    }

    #[test]
    fn details_are_plain_language_and_tones_carry_words() {
        let v = view(samples::blocking());
        let ks = v.details.iter().find(|d| d.label == "Kill switch").unwrap();
        assert!(ks.value.starts_with("On"));
        assert_eq!(ks.tone, Tone::Good);
        let v = view(samples::disconnected());
        let ks = v.details.iter().find(|d| d.label == "Kill switch").unwrap();
        assert!(ks.value.starts_with("Off"));
    }

    #[test]
    fn the_captive_window_shows_time_and_warning() {
        let c = view(samples::captive()).captive.unwrap();
        assert_eq!(c.left, "4:32 left");
        assert!(c.warning.contains("OUTSIDE the tunnel"));
        let soon = Response::ok()
            .with("captive_portal", "open")
            .with("captive_remaining", "soon");
        let c = view(soon).captive;
        assert_eq!(c.unwrap().left, "time left unknown; it still ends on time");
        assert!(view(samples::blocking()).captive.is_none());
    }

    #[test]
    fn untrusted_text_is_bounded_and_has_no_control_characters() {
        assert_eq!(plain("a\u{1b}[31mb\nc"), "a[31mbc");
        let long = "x".repeat(500);
        assert_eq!(plain(&long).chars().count(), 201);
        assert_eq!(service_state("weird\u{7}").0, "weird");
    }
}
