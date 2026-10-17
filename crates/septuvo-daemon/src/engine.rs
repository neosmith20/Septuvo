//! What the service does with each command, independent of sockets.
//!
//! Before the G1 review there is no tunnel. "Connect" means: turn on the
//! kill switch for the configured policy (N1), so protected traffic is
//! blocked because no VPN path exists. Status is honest about that: peer
//! authentication always **fails**, so the overall status is never
//! "connected" (D-020).

use septuvo_config::DeviceConfig;
use septuvo_ipc::{Command, Response};
use septuvo_policy::nft::{self, KillSwitch};
use std::path::Path;
use std::process::{Command as Process, Stdio};

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
        let (family, name) = nft::TABLE.split_once(' ').unwrap_or(("inet", "septuvo"));
        Process::new(nft)
            .args(["list", "table", family, name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
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
pub struct Engine<F: Firewall, I: Interfaces> {
    config: DeviceConfig,
    firewall: F,
    interfaces: I,
    /// The user asked to be protected (connect) and has not disconnected.
    wanted: bool,
}

impl<F: Firewall, I: Interfaces> Engine<F, I> {
    /// A new engine. Rules left by an earlier run (for example after a crash)
    /// are kept: they are only removed by a deliberate disconnect (Q2).
    pub fn new(config: DeviceConfig, firewall: F, interfaces: I) -> Self {
        Engine {
            config,
            firewall,
            interfaces,
            wanted: false,
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
        }
    }

    fn connect(&mut self) -> Response {
        let script = match nft::render(&self.config.policy, &self.kill_switch()) {
            Ok(s) => s,
            Err(e) => return Response::error("invalid_policy", &e.to_string()),
        };
        match self.firewall.apply(&script) {
            Ok(()) => {
                self.wanted = true;
                self.status().with(
                    "message",
                    "kill switch on: protected traffic is blocked; no tunnel exists before G1",
                )
            }
            Err(e) => Response::error("firewall", &e),
        }
    }

    fn disconnect(&mut self) -> Response {
        match self.firewall.apply(&nft::removal_script()) {
            Ok(()) => {
                self.wanted = false;
                self.status().with(
                    "message",
                    "disconnected: Septuvo rules removed (deliberate disconnect)",
                )
            }
            Err(e) => Response::error("firewall", &e),
        }
    }

    fn status(&mut self) -> Response {
        let kill_switch = self.firewall.active();
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
            .with("note", PEER_NOTE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use septuvo_ipc::Outcome;

    #[derive(Default)]
    struct FakeFirewall {
        installed: Option<String>,
        fail: bool,
        applied: usize,
    }

    impl Firewall for FakeFirewall {
        fn apply(&mut self, script: &str) -> Result<(), String> {
            if self.fail {
                return Err("simulated failure".into());
            }
            self.applied += 1;
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

    fn engine(fw: FakeFirewall, up: bool) -> Engine<FakeFirewall, FakeInterfaces> {
        Engine::new(config(), fw, FakeInterfaces(up))
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
