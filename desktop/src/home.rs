//! View model of the Advanced Home page (ADVANCED-UI §6, §11; N4-02).
//!
//! Built from the same service answer as the Simple window ([`MainView`]),
//! so both modes always agree. Everything shown is what the service
//! reported: the Kill Switch and DNS Protection tiles are status only, and a
//! value the service does not report is **Unknown**, never assumed. Local
//! Network Access is the one control (N4-04): clicking it asks the service,
//! and the tile shows only the value the service then reports.

use septuvo_cli::service::status_from;
use septuvo_cli::watch::Snapshot;

use crate::state::{Choice, MainView, Pending, Tone};

/// Whether a tile is a status indicator or a control (ADVANCED-UI §11, §14.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    /// Not focusable as a button; no click action.
    Status,
    /// A button; `reason` says why it is unavailable when disabled.
    Control {
        /// Whether it can be used now.
        enabled: bool,
        /// Why not, when disabled.
        reason: Option<&'static str>,
    },
}

/// One of the three Home tiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tile {
    /// `Kill Switch`, `DNS Protection` or `Local Network Access`.
    pub label: &'static str,
    /// `On`, `Off`, `Unknown`, `Failed`, `Allowed`, `Not Allowed`.
    pub value: &'static str,
    /// Colour of the value (always with the word).
    pub tone: Tone,
    /// The supporting sentence.
    pub text: String,
    /// Status or control.
    pub kind: TileKind,
}

impl Tile {
    /// What a screen reader announces, e.g. `Kill Switch, On, status`.
    #[must_use]
    pub fn accessible(&self) -> String {
        let kind = match self.kind {
            TileKind::Status => "status",
            TileKind::Control { .. } => "button",
        };
        format!("{}, {}, {kind}", self.label, self.value)
    }
}

/// The connection card (ADVANCED-UI §6.4): name and server address only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionCard {
    /// First line: the connection's name.
    pub title: String,
    /// Second line: its server address.
    pub detail: String,
}

/// Everything the Advanced Home page shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeView {
    /// The shared state: heading, lines, status, main button, notice.
    pub main: MainView,
    /// The large headline (the Simple heading, except "You’re Connected").
    pub headline: &'static str,
    /// Connection card.
    pub connection: ConnectionCard,
    /// The public-IP value: never sample or stale data (ADVANCED-UI §6.5).
    pub ip: &'static str,
    /// Whether the copy button is offered (only for a reported address).
    pub copy: bool,
    /// Kill Switch tile.
    pub kill_switch: Tile,
    /// DNS Protection tile.
    pub dns: Tile,
    /// Local Network Access tile.
    pub local_network: Tile,
}

/// The IP card's label.
pub const IP_LABEL: &str = "Your New IP Address";

const UNKNOWN_KS: &str = "Septuvo cannot check the kill switch right now.";
const UNKNOWN_DNS: &str = "Septuvo cannot check DNS protection right now.";
const UNKNOWN_LAN: &str = "Septuvo cannot check local network access right now.";
const LAN_NOT_RUNNING: &str = "The Septuvo service is not running.";
const LAN_WAITING: &str = "Waiting for the Septuvo service…";
/// The one short note shown after turning Local Network Access to Allowed
/// (ADVANCED-UI §11.3).
pub const LAN_ALLOWED_NOTE: &str =
    "Devices on this network will be able to reach you, and you them, outside the VPN.";

/// What the app last heard back about a Local Network Access change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LanNote {
    /// Nothing to add: the tile shows its usual sentence.
    #[default]
    None,
    /// The service confirmed **Allowed** after the user chose it: the one
    /// short note.
    Allowed,
    /// The service refused or failed; the tile keeps the value the service
    /// reports and says why.
    Refused(String),
}

fn tile(label: &'static str, value: &'static str, tone: Tone, text: &'static str) -> Tile {
    Tile {
        label,
        value,
        tone,
        text: text.to_string(),
        kind: TileKind::Status,
    }
}

/// The Kill Switch tile (ADVANCED-UI §11.1).
#[must_use]
pub fn kill_switch(snapshot: &Snapshot) -> Tile {
    let label = "Kill Switch";
    let Snapshot::Service(r) = snapshot else {
        return tile(label, "Unknown", Tone::Caution, UNKNOWN_KS);
    };
    match (r.field("state"), r.field("kill_switch")) {
        (Some("error-kill-switch-missing"), _) => tile(
            label,
            "Failed",
            Tone::Danger,
            "Protection could not be enforced. Open Diagnostics.",
        ),
        (Some("error-lockdown-missing"), _) => tile(
            label,
            "Failed",
            Tone::Danger,
            "Lockdown Mode could not be enforced. Open Diagnostics.",
        ),
        (Some("lockdown"), Some("on")) => tile(
            label,
            "On",
            Tone::Good,
            "Lockdown Mode blocks internet access until you connect.",
        ),
        (Some(_), Some("on")) => tile(
            label,
            "On",
            Tone::Good,
            "Stops internet if the VPN connection drops.",
        ),
        (Some("disconnected"), Some("off")) => tile(
            label,
            "Off",
            Tone::Neutral,
            "Turns on automatically when you connect.",
        ),
        _ => tile(label, "Unknown", Tone::Caution, UNKNOWN_KS),
    }
}

/// The DNS Protection tile (ADVANCED-UI §11.2). The service does not yet
/// report DNS enforcement separately, so while blocking it is **Unknown**;
/// **On** needs the service's verified DNS check (all four checks pass).
#[must_use]
pub fn dns(snapshot: &Snapshot) -> Tile {
    let label = "DNS Protection";
    let Snapshot::Service(r) = snapshot else {
        return tile(label, "Unknown", Tone::Caution, UNKNOWN_DNS);
    };
    if status_from(r).is_connected() {
        return tile(
            label,
            "On",
            Tone::Good,
            "Your DNS queries are encrypted and private.",
        );
    }
    match (r.field("state"), r.field("kill_switch")) {
        (Some("error-kill-switch-missing"), _) => tile(
            label,
            "Failed",
            Tone::Danger,
            "DNS protection could not be enforced. Open Diagnostics.",
        ),
        (Some("disconnected"), Some("off")) => tile(
            label,
            "Off",
            Tone::Neutral,
            "Turns on automatically when you connect.",
        ),
        _ => tile(label, "Unknown", Tone::Caution, UNKNOWN_DNS),
    }
}

/// The Local Network Access tile (ADVANCED-UI §11.3): the service's value,
/// read from its `local_network` field, as a control. It can be used only
/// while the service reports a value and nothing else is waiting for it.
#[must_use]
pub fn local_network(snapshot: &Snapshot, pending: Pending, note: &LanNote) -> Tile {
    let (value, tone, text) = match snapshot {
        Snapshot::Service(r) => match r.field("local_network") {
            Some("allowed") => (
                "Allowed",
                Tone::Good,
                "You can access devices on your local network.",
            ),
            Some("not-allowed") => (
                "Not Allowed",
                Tone::Neutral,
                "Devices on your local network are blocked while you are protected.",
            ),
            _ => ("Unknown", Tone::Caution, UNKNOWN_LAN),
        },
        Snapshot::Lost(_) => ("Unknown", Tone::Caution, UNKNOWN_LAN),
    };
    let text = match note {
        LanNote::Refused(why) => format!("Not changed: {}", crate::state::plain(why)),
        LanNote::Allowed if value == "Allowed" => LAN_ALLOWED_NOTE.to_string(),
        _ => text.to_string(),
    };
    let reason = match (snapshot, pending) {
        (Snapshot::Lost(_), _) => Some(LAN_NOT_RUNNING),
        (_, Pending::Setting | Pending::Disconnecting) => Some(LAN_WAITING),
        _ if value == "Unknown" => Some(UNKNOWN_LAN),
        _ => None,
    };
    Tile {
        label: "Local Network Access",
        value,
        tone,
        text,
        kind: TileKind::Control {
            enabled: reason.is_none(),
            reason,
        },
    }
}

fn connected(snapshot: &Snapshot) -> bool {
    matches!(snapshot, Snapshot::Service(r) if status_from(r).is_connected())
}

impl HomeView {
    /// The Home page for one service answer.
    #[must_use]
    pub fn build(snapshot: &Snapshot, pending: Pending, notice: Option<&str>) -> HomeView {
        Self::build_with(
            snapshot,
            pending,
            notice,
            &LanNote::None,
            &Choice::default(),
        )
    }

    /// The Home page, with what the app last heard about a Local Network
    /// Access change.
    #[must_use]
    pub fn build_with(
        snapshot: &Snapshot,
        pending: Pending,
        notice: Option<&str>,
        lan: &LanNote,
        choice: &Choice,
    ) -> HomeView {
        let main = MainView::build_with(snapshot, pending, notice, choice);
        let headline = if main.heading == "Protected" {
            "You’re Connected"
        } else {
            main.heading
        };
        let is_connected = connected(snapshot);
        HomeView {
            headline,
            // The name and server address only, never a location (§6.4).
            connection: ConnectionCard {
                title: main.selector.text.clone(),
                detail: match (&choice.name, choice.draft, &choice.detail) {
                    (None, _, _) => "Add or import a VPN configuration.".into(),
                    (Some(_), true, _) => "Draft · Setup Required".into(),
                    (Some(_), false, Some(d)) => crate::state::plain(d),
                    (Some(_), false, None) => String::new(),
                },
            },
            // The address comes only from the user's own server over the
            // verified tunnel (after G1); nothing reports it yet.
            ip: if is_connected {
                "Not Available"
            } else {
                "Not Connected"
            },
            copy: false,
            kill_switch: kill_switch(snapshot),
            dns: dns(snapshot),
            local_network: local_network(snapshot, pending, lan),
            main,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::samples;
    use septuvo_ipc::Response;

    fn svc(r: Response) -> Snapshot {
        Snapshot::Service(r)
    }

    fn lost() -> Snapshot {
        Snapshot::Lost("gone".into())
    }

    fn values(s: &Snapshot) -> [&'static str; 3] {
        let v = HomeView::build(s, Pending::None, None);
        [v.kill_switch.value, v.dns.value, v.local_network.value]
    }

    #[test]
    fn every_home_state() {
        let cases = [
            (
                svc(samples::disconnected()),
                "VPN Not Yet Available",
                ["Off", "Off", "Unknown"],
                "Not Connected",
            ),
            (
                svc(samples::blocking()),
                "Traffic Protected by Kill Switch",
                ["On", "Unknown", "Unknown"],
                "Not Connected",
            ),
            (
                svc(samples::captive()),
                "Traffic Protected by Kill Switch",
                ["On", "Unknown", "Unknown"],
                "Not Connected",
            ),
            (
                svc(samples::kill_switch_missing()),
                "Connection Failed",
                ["Failed", "Failed", "Unknown"],
                "Not Connected",
            ),
            (
                lost(),
                "Service Unavailable",
                ["Unknown", "Unknown", "Unknown"],
                "Not Connected",
            ),
            (
                svc(samples::protected()),
                "You’re Connected",
                ["On", "On", "Unknown"],
                "Not Available",
            ),
        ];
        for (s, headline, tiles, ip) in cases {
            let v = HomeView::build(&s, Pending::None, None);
            assert_eq!(v.headline, headline);
            assert_eq!(values(&s), tiles, "{headline}");
            assert_eq!(v.ip, ip, "{headline}");
            assert!(!v.copy, "no address is ever reported before G1");
        }
        let v = HomeView::build(&svc(samples::blocking()), Pending::Disconnecting, None);
        assert_eq!(v.headline, "Disconnecting…");
    }

    #[test]
    fn earlier_run_rules_count_as_on_but_never_as_connected() {
        let mut fields = Response::ok()
            .with("state", "blocking-left-from-earlier-run")
            .with("kill_switch", "on");
        for k in ["adapter", "peer", "reachability", "dns"] {
            fields = fields.with(k, "fail");
        }
        let v = HomeView::build(&svc(fields), Pending::None, None);
        assert_eq!(v.kill_switch.value, "On");
        assert_eq!(v.dns.value, "Unknown");
        assert_ne!(v.headline, "You’re Connected");
    }

    #[test]
    fn values_the_service_does_not_report_are_unknown() {
        let v = HomeView::build(&svc(Response::ok()), Pending::None, None);
        assert_eq!(
            [v.kill_switch.value, v.dns.value, v.local_network.value],
            ["Unknown"; 3]
        );
        // A kill switch the service reports as on with no state is still unknown.
        let v = HomeView::build(
            &svc(Response::ok().with("kill_switch", "on")),
            Pending::None,
            None,
        );
        assert_eq!(v.kill_switch.value, "Unknown");
    }

    #[test]
    fn local_network_access_shows_the_services_value() {
        let lan = |s: &Snapshot| local_network(s, Pending::None, &LanNote::None);
        for (field, value) in [
            ("allowed", "Allowed"),
            ("not-allowed", "Not Allowed"),
            ("maybe", "Unknown"),
        ] {
            let s = svc(samples::blocking().with("local_network", field));
            assert_eq!(lan(&s).value, value);
        }
        // Operable only with a value from the service and nothing waiting.
        let known = svc(samples::lockdown());
        assert_eq!(
            lan(&known).kind,
            TileKind::Control {
                enabled: true,
                reason: None
            }
        );
        for (s, pending, reason) in [
            (svc(samples::blocking()), Pending::None, UNKNOWN_LAN),
            (lost(), Pending::None, LAN_NOT_RUNNING),
            (known.clone(), Pending::Setting, LAN_WAITING),
            (known.clone(), Pending::Disconnecting, LAN_WAITING),
        ] {
            assert_eq!(
                local_network(&s, pending, &LanNote::None).kind,
                TileKind::Control {
                    enabled: false,
                    reason: Some(reason)
                }
            );
        }
    }

    #[test]
    fn allowing_shows_the_one_note_and_a_refusal_says_why() {
        let allowed = svc(Response::ok()
            .with("state", "lockdown")
            .with("kill_switch", "on")
            .with("local_network", "allowed"));
        let t = local_network(&allowed, Pending::None, &LanNote::Allowed);
        assert_eq!((t.value, t.text.as_str()), ("Allowed", LAN_ALLOWED_NOTE));
        // The note never contradicts the service: Not Allowed keeps its text.
        let t = local_network(&svc(samples::lockdown()), Pending::None, &LanNote::Allowed);
        assert_eq!(t.value, "Not Allowed");
        assert_ne!(t.text, LAN_ALLOWED_NOTE);
        let t = local_network(
            &svc(samples::lockdown()),
            Pending::None,
            &LanNote::Refused("the firewall refused\u{1b}[2J".into()),
        );
        assert_eq!(t.value, "Not Allowed");
        assert_eq!(t.text, "Not changed: the firewall refused[2J");
    }

    #[test]
    fn lockdown_mode_tiles() {
        let v = HomeView::build(&svc(samples::lockdown()), Pending::None, None);
        assert_eq!(v.headline, "Internet Blocked");
        assert_eq!(
            [v.kill_switch.value, v.dns.value, v.local_network.value],
            ["On", "Unknown", "Not Allowed"]
        );
        let v = HomeView::build(
            &svc(Response::ok().with("state", "error-lockdown-missing")),
            Pending::None,
            None,
        );
        assert_eq!(v.kill_switch.value, "Failed");
    }

    #[test]
    fn tiles_announce_label_value_and_kind() {
        let s = svc(samples::blocking());
        assert_eq!(kill_switch(&s).accessible(), "Kill Switch, On, status");
        assert_eq!(dns(&s).accessible(), "DNS Protection, Unknown, status");
        assert_eq!(
            local_network(&s, Pending::None, &LanNote::None).accessible(),
            "Local Network Access, Unknown, button"
        );
    }

    #[test]
    fn only_all_four_checks_turn_dns_on() {
        for passing in 0..16u8 {
            let mut r = Response::ok()
                .with("state", "connected")
                .with("kill_switch", "on");
            for (i, k) in ["adapter", "peer", "reachability", "dns"]
                .iter()
                .enumerate()
            {
                r = r.with(
                    k,
                    if passing & (1 << i) != 0 {
                        "pass"
                    } else {
                        "fail"
                    },
                );
            }
            let on = dns(&svc(r)).value == "On";
            assert_eq!(on, passing == 15, "checks {passing:04b}");
        }
    }
}
