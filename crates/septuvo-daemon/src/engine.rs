//! What the service does with each command, independent of sockets.
//!
//! Before the G1 review there is no tunnel. "Connect" means: turn on the
//! kill switch for the configured policy (N1), so protected traffic is
//! blocked because no VPN path exists. Status is honest about that: peer
//! authentication always **fails**, so the overall status is never
//! "connected" (D-020).
//!
//! With tunnel DNS (N2-03) connect also restricts DNS to the configured
//! resolvers through the tunnel and points the resolver file at them; a
//! deliberate disconnect restores both.
//!
//! Captive-portal mode (Q1, N3-02) opens web traffic and DNS to the
//! network's own resolvers outside the tunnel for at most five minutes, only
//! while connected. The **kernel** ends the window (timeout sets, see
//! `septuvo_policy::nft`), so it closes on time even if this service dies;
//! the service only remembers the deadline to show the time left.

use crate::resolver::Resolver;
use septuvo_config::{DeviceConfig, DnsPolicy};
use septuvo_ipc::{Command, Response};
use septuvo_policy::nft::{self, DnsRules, KillSwitch};
use std::path::Path;
use std::process::{Command as Process, Stdio};
use std::time::{Duration, Instant};

/// Shown in status and on start while captive-portal mode is open.
pub const CAPTIVE_WARNING: &str = "captive-portal mode: web traffic (TCP 80 and 443) and DNS to the \
     network's resolvers leave OUTSIDE the tunnel, unprotected, until the timer ends";

/// Shown for peer authentication, which does not exist before G1.
pub const PEER_NOTE: &str = "peer authentication is not available before the G1 security review";

/// Applies and removes kill-switch rules.
pub trait Firewall {
    /// Replaces any Septuvo rules with `script` (an `nft -f` script).
    ///
    /// # Errors
    /// Why the rules could not be applied; the previous rules then stay.
    fn apply(&mut self, script: &str) -> Result<(), String>;
    /// True if Septuvo rules are currently installed.
    fn active(&mut self) -> bool;
    /// True if the captive-portal web set currently holds elements, that is
    /// the kernel still keeps the window open.
    fn captive_open(&mut self) -> bool;
}

/// The real firewall: the `nft` program, found at a fixed path (a
/// privileged process does not trust `PATH`).
#[derive(Debug, Default)]
pub struct Nftables;

const NFT_PATHS: [&str; 3] = ["/usr/sbin/nft", "/sbin/nft", "/usr/bin/nft"];

fn nft_binary() -> Option<&'static str> {
    NFT_PATHS.into_iter().find(|p| Path::new(p).is_file())
}

impl Firewall for Nftables {
    fn apply(&mut self, script: &str) -> Result<(), String> {
        use std::io::Write;
        let nft = nft_binary().ok_or("the nft program is not installed")?;
        let mut child = Process::new(nft)
            .args(["-f", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot run nft: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(script.as_bytes())
                .map_err(|e| format!("cannot send rules to nft: {e}"))?;
        }
        let out = child
            .wait_with_output()
            .map_err(|e| format!("nft did not finish: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            let msg = String::from_utf8_lossy(&out.stderr);
            Err(format!("nft rejected the rules: {}", msg.trim()))
        }
    }

    fn active(&mut self) -> bool {
        let Some(nft) = nft_binary() else {
            return false;
        };
        // Fixed arguments only, passed directly (no shell).
        Process::new(nft)
            .args(["list", "table", nft::TABLE_FAMILY, nft::TABLE_NAME])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    fn captive_open(&mut self) -> bool {
        let Some(nft) = nft_binary() else {
            return false;
        };
        // Fixed arguments only, passed directly (no shell). The kernel hides
        // expired elements, so "elements =" means the window is still open.
        Process::new(nft)
            .args([
                "list",
                "set",
                nft::TABLE_FAMILY,
                nft::TABLE_NAME,
                nft::CAPTIVE_WEB_SET,
            ])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .is_ok_and(|o| {
                o.status.success() && String::from_utf8_lossy(&o.stdout).contains("elements =")
            })
    }
}

/// Whether the tunnel interface exists, from `/sys/class/net`.
pub trait Interfaces {
    /// True if an interface called `name` exists and is up.
    fn is_up(&self, name: &str) -> bool;
}

/// The real interface table.
#[derive(Debug, Default)]
pub struct SysClassNet;

impl Interfaces for SysClassNet {
    fn is_up(&self, name: &str) -> bool {
        let path = Path::new("/sys/class/net").join(name).join("operstate");
        std::fs::read_to_string(path).is_ok_and(|s| matches!(s.trim(), "up" | "unknown"))
    }
}

/// The service's state machine.
pub struct Engine<F: Firewall, I: Interfaces, R: Resolver> {
    config: DeviceConfig,
    firewall: F,
    interfaces: I,
    resolver: R,
    /// The user asked to be protected (connect) and has not disconnected.
    wanted: bool,
    /// When the current captive-portal window ends, if this service opened
    /// it. Display only: the kernel enforces the end.
    captive_until: Option<Instant>,
}

impl<F: Firewall, I: Interfaces, R: Resolver> Engine<F, I, R> {
    /// A new engine. Rules and a resolver file left by an earlier run (for
    /// example after a crash) are kept: they are only removed by a deliberate
    /// disconnect (Q2).
    pub fn new(config: DeviceConfig, firewall: F, interfaces: I, resolver: R) -> Self {
        Engine {
            config,
            firewall,
            interfaces,
            resolver,
            wanted: false,
            captive_until: None,
        }
    }

    /// Handles one command. `control` is true if it came over the control
    /// socket; changes are refused otherwise.
    pub fn handle(&mut self, command: Command, control: bool) -> Response {
        if command.needs_control() && !control {
            return Response::error("not_permitted", "this command needs the control socket");
        }
        match command {
            Command::Status => self.status(),
            Command::Connect => self.connect(),
            Command::Disconnect => self.disconnect(),
            Command::Captive { minutes } => self.captive(minutes),
        }
    }

    fn kill_switch(&self) -> KillSwitch {
        KillSwitch {
            tunnel_interface: self.config.tunnel.interface.clone(),
            endpoints: self
                .config
                .endpoints
                .iter()
                .map(|e| (e.ip(), e.port()))
                .collect(),
            dns: match &self.config.dns {
                DnsPolicy::Tunnel {
                    resolvers,
                    lan_exceptions,
                } => DnsRules::Resolvers {
                    resolvers: resolvers.clone(),
                    lan_exceptions: lan_exceptions.clone(),
                },
                DnsPolicy::System => DnsRules::System,
            },
        }
    }

    fn tunnel_resolvers(&self) -> Option<&[std::net::IpAddr]> {
        match &self.config.dns {
            DnsPolicy::Tunnel { resolvers, .. } => Some(resolvers),
            DnsPolicy::System => None,
        }
    }

    fn connect(&mut self) -> Response {
        let script = match nft::render(&self.config.policy, &self.kill_switch()) {
            Ok(s) => s,
            Err(e) => return Response::error("invalid_policy", &e.to_string()),
        };
        if let Err(e) = self.firewall.apply(&script) {
            return Response::error("firewall", &e);
        }
        self.wanted = true;
        // The new table starts with empty captive-portal sets.
        self.captive_until = None;
        if let Some(servers) = self.tunnel_resolvers().map(<[_]>::to_vec)
            && let Err(e) = self.resolver.install(&servers)
        {
            // The kill switch is on, so DNS still cannot leak; report it.
            return Response::error("resolver", &format!("kill switch on, but {e}"));
        }
        self.status().with(
            "message",
            "kill switch on: protected traffic is blocked; no tunnel exists before G1",
        )
    }

    fn disconnect(&mut self) -> Response {
        if let Err(e) = self.firewall.apply(&nft::removal_script()) {
            return Response::error("firewall", &e);
        }
        self.wanted = false;
        // Removing the table ended any captive-portal window at once.
        self.captive_until = None;
        if let Err(e) = self.resolver.restore() {
            return Response::error("resolver", &format!("rules removed, but {e}"));
        }
        self.status().with(
            "message",
            "disconnected: Septuvo rules removed and the resolver file restored (deliberate disconnect)",
        )
    }

    /// Opens captive-portal mode for `minutes` (1 to 5). Asking again
    /// restarts the window from now; it never adds up.
    fn captive(&mut self, minutes: u8) -> Response {
        if !(1..=septuvo_ipc::MAX_CAPTIVE_MINUTES).contains(&minutes) {
            return Response::error(
                "invalid_request",
                "captive-portal mode lasts 1 to 5 minutes",
            );
        }
        if !self.wanted || !self.firewall.active() {
            return Response::error(
                "not_connected",
                "captive-portal mode is only available while connected (kill switch on)",
            );
        }
        let resolvers = match self.config.dns {
            DnsPolicy::Tunnel { .. } => self.resolver.network_resolvers(),
            // System DNS (split mode only) is not restricted, so nothing to open.
            DnsPolicy::System => Vec::new(),
        };
        let seconds = u32::from(minutes) * 60;
        let script = match nft::captive_script(seconds, &resolvers) {
            Ok(s) => s,
            Err(e) => return Response::error("invalid_request", &e.to_string()),
        };
        if let Err(e) = self.firewall.apply(&script) {
            return Response::error("firewall", &e);
        }
        self.captive_until = Some(Instant::now() + Duration::from_secs(u64::from(seconds)));
        let dns_note = if resolvers.is_empty() {
            "no network resolver found, web traffic only".to_string()
        } else {
            let list: Vec<String> = resolvers.iter().map(ToString::to_string).collect();
            format!("network DNS: {}", list.join(", "))
        };
        self.status()
            .with("captive_resolvers", &dns_note)
            .with("message", CAPTIVE_WARNING)
    }

    fn status(&mut self) -> Response {
        let kill_switch = self.firewall.active();
        let captive = kill_switch && self.firewall.captive_open();
        if !captive {
            self.captive_until = None;
        }
        let adapter = if self.interfaces.is_up(&self.config.tunnel.interface) {
            "pass"
        } else {
            "fail"
        };
        let state = match (self.wanted, kill_switch) {
            (true, true) => "blocking",
            (false, true) => "blocking-left-from-earlier-run",
            (true, false) => "error-kill-switch-missing",
            (false, false) => "disconnected",
        };
        Response::ok()
            .with("state", state)
            .with("kill_switch", if kill_switch { "on" } else { "off" })
            .with("interface", &self.config.tunnel.interface)
            .with("adapter", adapter)
            .with("peer", "fail")
            .with("reachability", "unknown")
            .with("dns", "unknown")
            .with("dns_policy", &self.dns_policy())
            .with(
                "resolver_file",
                if self.resolver.installed() {
                    "septuvo"
                } else {
                    "system"
                },
            )
            .with("captive_portal", if captive { "open" } else { "closed" })
            .with_if(captive, "captive_remaining", &self.captive_remaining())
            .with_if(captive, "captive_warning", CAPTIVE_WARNING)
            .with("note", PEER_NOTE)
    }

    /// Seconds left in the window, or "unknown" if it was opened before this
    /// service started (it still ends on time: the kernel enforces it).
    fn captive_remaining(&self) -> String {
        self.captive_until.map_or_else(
            || "unknown".to_string(),
            |until| {
                until
                    .saturating_duration_since(Instant::now())
                    .as_secs()
                    .to_string()
            },
        )
    }

    fn dns_policy(&self) -> String {
        match &self.config.dns {
            DnsPolicy::Tunnel { resolvers, .. } => {
                let list: Vec<String> = resolvers.iter().map(ToString::to_string).collect();
                format!("only through the tunnel to {}", list.join(", "))
            }
            DnsPolicy::System => "system resolver (split mode, not protected)".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use septuvo_ipc::Outcome;

    #[derive(Default)]
    struct FakeFirewall {
        installed: Option<String>,
        /// The last captive-portal script, while the "kernel" keeps it open.
        captive: Option<String>,
        fail: bool,
        applied: usize,
    }

    impl Firewall for FakeFirewall {
        fn apply(&mut self, script: &str) -> Result<(), String> {
            if self.fail {
                return Err("simulated failure".into());
            }
            self.applied += 1;
            if script.contains("add element") {
                assert!(self.installed.is_some(), "sets exist only with the table");
                self.captive = Some(script.to_string());
                return Ok(());
            }
            // A new or removed table always starts with empty sets.
            self.captive = None;
            self.installed = if script.contains("delete table") && !script.contains("chain") {
                None
            } else {
                Some(script.to_string())
            };
            Ok(())
        }
        fn active(&mut self) -> bool {
            self.installed.is_some()
        }
        fn captive_open(&mut self) -> bool {
            self.captive.is_some()
        }
    }

    #[derive(Default)]
    struct FakeResolver {
        servers: Option<Vec<std::net::IpAddr>>,
        fail: bool,
    }

    impl Resolver for FakeResolver {
        fn install(&mut self, servers: &[std::net::IpAddr]) -> Result<(), String> {
            if self.fail {
                return Err("simulated resolver failure".into());
            }
            self.servers = Some(servers.to_vec());
            Ok(())
        }
        fn restore(&mut self) -> Result<(), String> {
            self.servers = None;
            Ok(())
        }
        fn installed(&self) -> bool {
            self.servers.is_some()
        }
        fn network_resolvers(&self) -> Vec<std::net::IpAddr> {
            vec!["192.168.1.1".parse().unwrap()]
        }
    }

    struct FakeInterfaces(bool);
    impl Interfaces for FakeInterfaces {
        fn is_up(&self, _: &str) -> bool {
            self.0
        }
    }

    fn config() -> DeviceConfig {
        septuvo_config::parse_device(
            "version = 1\n[server]\nendpoints = [\"203.0.113.7:51820\"]\n\
             [routing]\nmode = \"full-tunnel\"\ndirect = [\"192.168.1.30/32\"]\n\
             [dns]\nresolvers = [\"10.77.0.1\"]\n",
        )
        .expect("valid test configuration")
    }

    fn engine(fw: FakeFirewall, up: bool) -> Engine<FakeFirewall, FakeInterfaces, FakeResolver> {
        Engine::new(config(), fw, FakeInterfaces(up), FakeResolver::default())
    }

    #[test]
    fn connect_restricts_dns_and_installs_the_resolver_file() {
        let mut e = engine(FakeFirewall::default(), false);
        let r = e.handle(Command::Connect, true);
        assert_eq!(r.outcome, Outcome::Ok);
        let rules = e.firewall.installed.clone().expect("rules");
        assert!(
            rules.contains("ip daddr 10.77.0.1 oifname \"septuvo0\" udp dport { 53, 853 } accept")
        );
        assert!(rules.contains("udp dport { 53, 853 } drop"));
        assert_eq!(e.resolver.servers, Some(vec!["10.77.0.1".parse().unwrap()]));
        assert_eq!(r.field("resolver_file"), Some("septuvo"));
        assert_eq!(
            r.field("dns_policy"),
            Some("only through the tunnel to 10.77.0.1")
        );
        let r = e.handle(Command::Disconnect, true);
        assert_eq!(r.outcome, Outcome::Ok);
        assert_eq!(e.resolver.servers, None);
        assert_eq!(r.field("resolver_file"), Some("system"));
    }

    #[test]
    fn resolver_failure_keeps_the_kill_switch_on() {
        let mut e = engine(FakeFirewall::default(), false);
        e.resolver.fail = true;
        let r = e.handle(Command::Connect, true);
        assert!(matches!(r.outcome, Outcome::Err { ref code, .. } if code == "resolver"));
        assert!(e.firewall.installed.is_some(), "still failing closed");
        assert_eq!(
            e.handle(Command::Status, false).field("state"),
            Some("blocking")
        );
    }

    #[test]
    fn system_dns_in_split_mode_leaves_the_resolver_file_alone() {
        let config = septuvo_config::parse_device(
            "version = 1\n[server]\nendpoints = [\"203.0.113.7:51820\"]\n\
             [routing]\nmode = \"split\"\ntunnel = [\"192.168.50.0/24\"]\n\
             [dns]\nmode = \"system\"\n",
        )
        .expect("valid");
        let mut e = Engine::new(
            config,
            FakeFirewall::default(),
            FakeInterfaces(false),
            FakeResolver::default(),
        );
        assert_eq!(e.handle(Command::Connect, true).outcome, Outcome::Ok);
        assert_eq!(e.resolver.servers, None);
        assert!(
            !e.firewall
                .installed
                .clone()
                .unwrap()
                .contains("dport { 53, 853 } drop")
        );
    }

    #[test]
    fn control_commands_are_refused_on_the_status_socket() {
        let mut e = engine(FakeFirewall::default(), false);
        for c in [Command::Connect, Command::Disconnect] {
            let r = e.handle(c, false);
            assert!(matches!(r.outcome, Outcome::Err { ref code, .. } if code == "not_permitted"));
        }
        assert_eq!(e.firewall.applied, 0, "nothing may change");
        assert_eq!(e.handle(Command::Status, false).outcome, Outcome::Ok);
    }

    #[test]
    fn connect_installs_the_kill_switch_and_never_claims_a_peer() {
        let mut e = engine(FakeFirewall::default(), true);
        let r = e.handle(Command::Connect, true);
        assert_eq!(r.outcome, Outcome::Ok);
        let rules = e.firewall.installed.clone().expect("rules installed");
        assert!(rules.contains("policy drop"));
        assert!(rules.contains("203.0.113.7"));
        assert_eq!(r.field("state"), Some("blocking"));
        assert_eq!(r.field("kill_switch"), Some("on"));
        assert_eq!(r.field("adapter"), Some("pass"));
        assert_eq!(r.field("peer"), Some("fail"), "no peer before G1");
        assert_eq!(r.field("note"), Some(PEER_NOTE));
    }

    /// Defence in depth: even if a hostile interface name got past the
    /// configuration check, the renderer refuses it and nothing reaches nft.
    #[test]
    fn hostile_interface_name_never_reaches_the_firewall() {
        let mut cfg = config();
        cfg.tunnel.interface = "septuvo0\" accept; #".to_string();
        let mut e = Engine::new(
            cfg,
            FakeFirewall::default(),
            FakeInterfaces(false),
            FakeResolver::default(),
        );
        let r = e.handle(Command::Connect, true);
        assert!(matches!(r.outcome, Outcome::Err { ref code, .. } if code == "invalid_policy"));
        assert_eq!(e.firewall.applied, 0, "nothing was sent to nft");
    }

    fn remaining(r: &Response) -> u64 {
        r.field("captive_remaining")
            .and_then(|s| s.parse().ok())
            .expect("seconds left")
    }

    #[test]
    fn captive_portal_needs_the_control_socket_and_a_connection() {
        let mut e = engine(FakeFirewall::default(), false);
        let r = e.handle(Command::Captive { minutes: 5 }, true);
        assert!(matches!(r.outcome, Outcome::Err { ref code, .. } if code == "not_connected"));
        let _ = e.handle(Command::Connect, true);
        let applied = e.firewall.applied;
        let r = e.handle(Command::Captive { minutes: 5 }, false);
        assert!(matches!(r.outcome, Outcome::Err { ref code, .. } if code == "not_permitted"));
        for minutes in [0, 6, 255] {
            let r = e.handle(Command::Captive { minutes }, true);
            assert!(
                matches!(r.outcome, Outcome::Err { ref code, .. } if code == "invalid_request"),
                "{minutes}"
            );
        }
        assert_eq!(e.firewall.applied, applied, "refusals change nothing");
        assert_eq!(e.firewall.captive, None);
        let s = e.handle(Command::Status, false);
        assert_eq!(s.field("captive_portal"), Some("closed"));
        assert_eq!(s.field("captive_warning"), None);
    }

    #[test]
    fn captive_portal_opens_web_and_network_dns_with_a_kernel_timeout() {
        let mut e = engine(FakeFirewall::default(), false);
        let _ = e.handle(Command::Connect, true);
        let r = e.handle(Command::Captive { minutes: 5 }, true);
        assert_eq!(r.outcome, Outcome::Ok);
        let script = e.firewall.captive.clone().expect("window opened");
        assert!(script.contains("captive_web { 80 timeout 300s, 443 timeout 300s }"));
        assert!(script.contains("captive_dns4 { 192.168.1.1 timeout 300s }"));
        assert!(!script.contains("10.77.0.1"), "never the tunnel resolver");
        assert_eq!(r.field("captive_portal"), Some("open"));
        assert!((295..=300).contains(&remaining(&r)));
        assert_eq!(r.field("captive_warning"), Some(CAPTIVE_WARNING));
        assert_eq!(r.field("message"), Some(CAPTIVE_WARNING));
        assert_eq!(r.field("peer"), Some("fail"), "still never connected");
        let r = e.handle(Command::Captive { minutes: 1 }, true);
        assert!(
            remaining(&r) <= 60,
            "asking again restarts from now, never adds up"
        );
        assert!(
            e.firewall
                .captive
                .clone()
                .unwrap()
                .contains("80 timeout 60s")
        );
    }

    #[test]
    fn captive_portal_ends_on_disconnect_reconnect_and_kernel_expiry() {
        let mut e = engine(FakeFirewall::default(), false);
        let _ = e.handle(Command::Connect, true);
        let _ = e.handle(Command::Captive { minutes: 5 }, true);
        let r = e.handle(Command::Disconnect, true);
        assert_eq!(r.field("captive_portal"), Some("closed"));
        assert_eq!(e.firewall.captive, None);

        let _ = e.handle(Command::Connect, true);
        let _ = e.handle(Command::Captive { minutes: 5 }, true);
        let r = e.handle(Command::Connect, true);
        assert_eq!(
            r.field("captive_portal"),
            Some("closed"),
            "a new table starts closed"
        );

        let _ = e.handle(Command::Captive { minutes: 5 }, true);
        e.firewall.captive = None; // the kernel removed the elements
        let s = e.handle(Command::Status, false);
        assert_eq!(s.field("captive_portal"), Some("closed"));
        assert_eq!(s.field("captive_remaining"), None);
    }

    #[test]
    fn a_window_opened_before_a_restart_is_shown_with_unknown_time() {
        let fw = FakeFirewall {
            installed: Some("table inet septuvo { chain output { policy drop; } }".into()),
            captive: Some("add element".into()),
            ..FakeFirewall::default()
        };
        let mut e = engine(fw, false);
        let s = e.handle(Command::Status, false);
        assert_eq!(s.field("captive_portal"), Some("open"));
        assert_eq!(s.field("captive_remaining"), Some("unknown"));
        assert_eq!(s.field("captive_warning"), Some(CAPTIVE_WARNING));
    }

    #[test]
    fn captive_portal_with_system_dns_opens_web_only() {
        let config = septuvo_config::parse_device(
            "version = 1\n[server]\nendpoints = [\"203.0.113.7:51820\"]\n\
             [routing]\nmode = \"split\"\ntunnel = [\"192.168.50.0/24\"]\n\
             [dns]\nmode = \"system\"\n",
        )
        .expect("valid");
        let mut e = Engine::new(
            config,
            FakeFirewall::default(),
            FakeInterfaces(false),
            FakeResolver::default(),
        );
        let _ = e.handle(Command::Connect, true);
        let r = e.handle(Command::Captive { minutes: 2 }, true);
        assert_eq!(r.outcome, Outcome::Ok);
        let script = e.firewall.captive.clone().unwrap();
        assert!(!script.contains("captive_dns4 {"));
        assert_eq!(
            r.field("captive_resolvers"),
            Some("no network resolver found, web traffic only")
        );
    }

    #[test]
    fn disconnect_removes_rules() {
        let mut e = engine(FakeFirewall::default(), false);
        let _ = e.handle(Command::Connect, true);
        let r = e.handle(Command::Disconnect, true);
        assert_eq!(r.outcome, Outcome::Ok);
        assert_eq!(e.firewall.installed, None);
        assert_eq!(r.field("state"), Some("disconnected"));
        assert_eq!(r.field("kill_switch"), Some("off"));
    }

    #[test]
    fn firewall_failure_is_reported_and_keeps_state() {
        let mut e = engine(FakeFirewall::default(), false);
        let _ = e.handle(Command::Connect, true);
        e.firewall.fail = true;
        let r = e.handle(Command::Disconnect, true);
        assert!(matches!(r.outcome, Outcome::Err { ref code, .. } if code == "firewall"));
        // The rules are still there and the service still wants them.
        let s = e.handle(Command::Status, false);
        assert_eq!(s.field("state"), Some("blocking"));
    }

    #[test]
    fn rules_left_from_an_earlier_run_are_kept_and_reported() {
        let fw = FakeFirewall {
            installed: Some("table inet septuvo { chain output { policy drop; } }".into()),
            ..FakeFirewall::default()
        };
        let mut e = engine(fw, false);
        let s = e.handle(Command::Status, false);
        assert_eq!(s.field("state"), Some("blocking-left-from-earlier-run"));
        assert_eq!(e.firewall.applied, 0, "start-up never removes rules");
    }

    #[test]
    fn missing_kill_switch_while_wanted_is_an_error_state() {
        let mut e = engine(FakeFirewall::default(), false);
        let _ = e.handle(Command::Connect, true);
        e.firewall.installed = None; // removed behind the service's back
        assert_eq!(
            e.handle(Command::Status, false).field("state"),
            Some("error-kill-switch-missing")
        );
    }
}
