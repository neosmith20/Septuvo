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
//! Captive-portal mode (Q1, N3-02, revised after the owner's review of
//! PR #63) applies **only to one dedicated helper account**: while a window
//! is open, that account (and no other program) may reach web servers
//! outside the tunnel, for at most five minutes, only while connected, at
//! most [`MAX_CAPTIVE_WINDOWS`] times per connection. Plain DNS to the
//! network's resolvers is opened for the helper only when explicitly
//! authorized for that window; the resolver file is never changed. The
//! **kernel** ends the window (timeout sets, see `septuvo_policy::nft`), so
//! it closes on time even if this service dies; the service only remembers
//! the deadline to show the time left.
//!
//! **Saved connections (N4-03).** With a [`Store`], the service keeps the
//! user's complete connections and uses the selected one; without a
//! selection it uses the device configuration named in its settings, as
//! before. **Switching while connected (D-041)** replaces the whole ruleset
//! in **one nftables transaction** (`nft -f` applies a script atomically:
//! the old table is deleted and the new one created together), so
//! protection never lapses. If the switch fails, the kernel keeps the old
//! rules (still blocking), the new selection is kept, and status reports
//! `state=failed` with the reason; the old connection is never silently
//! resumed.
//!
//! **Lockdown Mode (N4-04, D-038).** When on, a blocking ruleset
//! ([`nft::render_lockdown`]) is in force whenever no connection is: at
//! service start, after a deliberate disconnect, and from boot (the
//! `--apply-lockdown` start-up step). It is lifted only by turning Lockdown
//! Mode off, the administrator recovery command or uninstall.
//!
//! **Local Network Access (N4-04, D-040).** When Allowed, the directly
//! attached subnets ([`lan::attached_subnets`]) are opened in whichever
//! ruleset is in force, recomputed whenever an address changes.
//!
//! **State vocabulary.** `state` is one of `disconnected`, `blocking`,
//! `blocking-left-from-earlier-run`, `error-kill-switch-missing`, `lockdown`
//! (Lockdown Mode blocking, not connected), `error-lockdown-missing`
//! (Lockdown Mode on, but its rules are not in place) and `failed` (with
//! `failure`). The protocol also reserves `connecting`,
//! `switching`, `disconnecting` and `reconnecting` for operations that take
//! time; before G1 every operation is one atomic firewall transaction done
//! while the service holds its lock, so status never observes them.

use crate::addresses::{Addresses, NoAddresses};
use crate::resolver::Resolver;
use crate::store::{ServiceSettings, Store, StoreError};
use septuvo_config::{DeviceConfig, DnsPolicy};
use septuvo_ipc::{Command, ConnectionId, Request, Response};
use septuvo_policy::lan::{self, LocalAccess};
use septuvo_policy::nft::{self, DnsRules, KillSwitch, Lockdown};
use septuvo_policy::{Action, Prefix};
use std::fmt::Write as _;
use std::path::Path;
use std::process::{Command as Process, Stdio};
use std::time::{Duration, Instant};

/// Shown in status and on start while a captive-portal window is open.
pub const CAPTIVE_WARNING: &str = "captive-portal window: the sign-in helper account may reach web \
     servers (TCP 80/443) OUTSIDE the tunnel, unprotected; every other program stays blocked";

/// Shown when the network's DNS was authorized for the window.
pub const NETWORK_DNS_WARNING: &str = "network DNS authorized for the helper only: answers from this \
     network are untrusted; the system resolver is unchanged";

/// Most captive-portal windows per connection: asking again cannot keep a
/// window open indefinitely. Disconnect and connect again to start over.
pub const MAX_CAPTIVE_WINDOWS: u8 = 3;

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
    /// The configuration in use: the selected saved connection, else
    /// `default`.
    config: DeviceConfig,
    /// The device configuration from the service settings.
    default: DeviceConfig,
    /// Which saved connection `config` is (`None`: the default).
    current: Option<ConnectionId>,
    /// Saved connections (`None`: not configured).
    store: Option<Store>,
    /// Why the last connect or switch failed, until the next success or a
    /// deliberate disconnect.
    failure: Option<String>,
    firewall: F,
    interfaces: I,
    resolver: R,
    /// The user asked to be protected (connect) and has not disconnected.
    wanted: bool,
    /// When the current captive-portal window ends, if this service opened
    /// it. Display only: the kernel enforces the end.
    captive_until: Option<Instant>,
    /// Whether the current window authorized the network's DNS (`None`:
    /// opened before this service started).
    captive_network_dns: Option<bool>,
    /// User ID of the dedicated helper account; `None`: not configured.
    captive_helper: Option<u32>,
    /// Windows opened since the last connect.
    captive_windows: u8,
    /// Where service events go: standard error (the journal) by default.
    log: fn(&str),
    /// The interfaces' addresses, for Local Network Access.
    addresses: Box<dyn Addresses>,
    /// What Local Network Access opened in the rules last applied.
    applied_local: LocalAccess,
}

fn log_to_stderr(line: &str) {
    eprintln!("septuvod: {line}");
}

impl<F: Firewall, I: Interfaces, R: Resolver> Engine<F, I, R> {
    /// A new engine. Rules and a resolver file left by an earlier run (for
    /// example after a crash) are kept: they are only removed by a deliberate
    /// disconnect (Q2).
    pub fn new(config: DeviceConfig, firewall: F, interfaces: I, resolver: R) -> Self {
        Engine {
            default: config.clone(),
            config,
            current: None,
            store: None,
            failure: None,
            firewall,
            interfaces,
            resolver,
            wanted: false,
            captive_until: None,
            captive_network_dns: None,
            captive_helper: None,
            captive_windows: 0,
            log: log_to_stderr,
            addresses: Box::new(NoAddresses),
            applied_local: LocalAccess::default(),
        }
    }

    /// Reads interface addresses from `addresses` (Local Network Access).
    #[must_use]
    pub fn with_addresses(mut self, addresses: Box<dyn Addresses>) -> Self {
        self.addresses = addresses;
        self
    }

    /// The service settings (all off without a store).
    fn settings(&self) -> ServiceSettings {
        self.store.as_ref().map(Store::service).unwrap_or_default()
    }

    /// Lockdown Mode is on and no connection is wanted: the Lockdown Mode
    /// ruleset must be in force.
    fn lockdown_idle(&self) -> bool {
        self.settings().lockdown && !self.wanted
    }

    /// Start-up (and the boot step): with Lockdown Mode on and no
    /// connection, installs the Lockdown Mode ruleset. Otherwise nothing:
    /// rules left by an earlier run are kept (Q2).
    ///
    /// # Errors
    /// The ruleset could not be applied.
    pub fn apply_startup(&mut self) -> Result<bool, String> {
        if !self.lockdown_idle() {
            return Ok(false);
        }
        self.apply_lockdown()?;
        (self.log)("Lockdown Mode: blocking ruleset installed (no connection)");
        Ok(true)
    }

    /// Recomputes Local Network Access after an address change and, if what
    /// it opens changed, re-applies the ruleset in force (NP-L6). Returns
    /// true if the rules were replaced.
    pub fn addresses_changed(&mut self) -> bool {
        if !self.settings().local_network || !(self.wanted || self.lockdown_idle()) {
            return false;
        }
        let now = if self.wanted {
            self.local_for_connection()
        } else {
            self.local_for_lockdown()
        };
        if now == self.applied_local {
            return false;
        }
        match self.refresh_rules() {
            Ok(()) => {
                (self.log)("addresses changed: Local Network Access rules updated");
                true
            }
            Err(e) => {
                (self.log)(&format!(
                    "addresses changed, but the rules could not be updated: {e}"
                ));
                false
            }
        }
    }

    /// The tunnel and blocked prefixes of `config` (NP-L4).
    fn excluded(config: &DeviceConfig) -> Vec<Prefix> {
        config
            .policy
            .rules()
            .iter()
            .filter(|r| r.action != Action::Direct)
            .map(|r| r.prefix)
            .collect()
    }

    /// Local Network Access for the connection in use.
    fn local_for_connection(&mut self) -> LocalAccess {
        if !self.settings().local_network {
            return LocalAccess::default();
        }
        let endpoints: Vec<_> = self
            .config
            .endpoints
            .iter()
            .map(std::net::SocketAddr::ip)
            .collect();
        let exclude = Self::excluded(&self.config);
        let addrs = self.addresses.current();
        lan::attached_subnets(&addrs, &self.config.tunnel.interface, &exclude, &endpoints).access
    }

    /// Every configuration the Lockdown Mode ruleset must let the user
    /// connect with: the default and every saved one.
    fn all_configs(&self) -> Vec<DeviceConfig> {
        let mut all = vec![self.default.clone()];
        if let Some(store) = &self.store {
            all.extend(store.configs());
        }
        all
    }

    /// Local Network Access while Lockdown Mode blocks: nothing that
    /// overlaps any saved connection's tunnel or blocked prefixes or
    /// endpoints is opened.
    fn local_for_lockdown(&mut self) -> LocalAccess {
        if !self.settings().local_network {
            return LocalAccess::default();
        }
        let configs = self.all_configs();
        let exclude: Vec<Prefix> = configs.iter().flat_map(Self::excluded).collect();
        let endpoints: Vec<_> = configs
            .iter()
            .flat_map(|c| c.endpoints.iter().map(std::net::SocketAddr::ip))
            .collect();
        let addrs = self.addresses.current();
        lan::attached_subnets(&addrs, &self.config.tunnel.interface, &exclude, &endpoints).access
    }

    fn apply_lockdown(&mut self) -> Result<(), String> {
        let mut endpoints: Vec<(std::net::IpAddr, u16)> = Vec::new();
        for c in self.all_configs() {
            for e in c.endpoints {
                if !endpoints.contains(&(e.ip(), e.port())) {
                    endpoints.push((e.ip(), e.port()));
                }
            }
        }
        endpoints.truncate(nft::MAX_LOCKDOWN_ENDPOINTS);
        let local = self.local_for_lockdown();
        let script = nft::render_lockdown(&Lockdown {
            tunnel_interface: self.config.tunnel.interface.clone(),
            endpoints,
            captive_helper: self.captive_helper,
            local: local.clone(),
        })
        .map_err(|e| e.to_string())?;
        self.firewall.apply(&script)?;
        self.applied_local = local;
        // A new table starts with empty captive-portal sets.
        self.captive_until = None;
        self.captive_network_dns = None;
        Ok(())
    }

    /// While Lockdown Mode blocks, the saved connections' endpoints are part
    /// of its ruleset: re-applied after the list changes.
    fn refresh_lockdown_endpoints(&mut self) {
        if self.lockdown_idle()
            && let Err(e) = self.apply_lockdown()
        {
            (self.log)(&format!("Lockdown Mode rules could not be updated: {e}"));
        }
    }

    /// Re-applies the ruleset that should be in force: the kill switch while
    /// connected, Lockdown Mode while it blocks; nothing otherwise.
    fn refresh_rules(&mut self) -> Result<(), String> {
        if self.wanted {
            self.apply_rules()
        } else if self.lockdown_idle() {
            self.apply_lockdown()
        } else {
            Ok(())
        }
    }

    /// Sends service events to `log` instead of standard error (tests).
    #[must_use]
    pub fn with_log(mut self, log: fn(&str)) -> Self {
        self.log = log;
        self
    }

    /// Uses saved connections from `store`. The selected one, if any, is
    /// used from now on; if it is no longer valid, the default is used and
    /// the reason logged.
    #[must_use]
    pub fn with_store(mut self, store: Store) -> Self {
        if let Some(id) = store.selected() {
            match store.config(id) {
                Ok(c) => {
                    self.config = c;
                    self.current = Some(id);
                }
                Err(e) => (self.log)(&format!(
                    "selected connection {id} cannot be used ({}); using the default configuration",
                    e.message
                )),
            }
        }
        self.store = Some(store);
        self
    }

    /// Enables captive-portal mode for the dedicated helper account `uid`
    /// (never 0; the renderer refuses root as well).
    #[must_use]
    pub fn with_captive_helper(mut self, uid: Option<u32>) -> Self {
        self.captive_helper = uid.filter(|&u| u != 0);
        self
    }

    /// Handles one command without a body. `control` is true if it came
    /// over the control socket; changes are refused otherwise.
    pub fn handle(&mut self, command: Command, control: bool) -> Response {
        self.handle_request(&Request::new(command), control)
    }

    /// Handles one request.
    pub fn handle_request(&mut self, request: &Request, control: bool) -> Response {
        let command = request.command;
        if command.needs_control() && !control {
            return Response::error("not_permitted", "this command needs the control socket");
        }
        if command.has_body() != request.body.is_some() {
            return Response::error("bad_request", "body missing or not expected");
        }
        let body = request.body.as_deref().unwrap_or_default();
        match command {
            Command::Status => self.status(),
            Command::Connect => self.connect(),
            Command::Disconnect => self.disconnect(),
            Command::Captive {
                minutes,
                network_dns,
            } => self.captive(minutes, network_dns),
            Command::List => self.list(),
            Command::Get(id) => self.with_store_ref(|s| s.export(id)).map_or_else(
                |e| e,
                |text| Response::ok().with("id", &id.to_string()).with_body(&text),
            ),
            Command::Add => self.add(body),
            Command::Replace(id) => self.replace(id, body),
            Command::Rename(id) => self.rename(id, body),
            Command::Delete(id) => self.delete(id),
            Command::Select(id) => self.select(id),
            Command::Lockdown(on) => self.set_lockdown(on),
            Command::LocalNetwork(allowed) => self.set_local_network(allowed),
        }
    }

    /// Lockdown Mode on or off (D-038). While connected nothing changes
    /// now: it applies when the connection ends.
    fn set_lockdown(&mut self, on: bool) -> Response {
        let old = self.settings();
        if old.lockdown == on {
            // Already on with no connection: install the rules again, which
            // repairs rules removed by hand (`error-lockdown-missing`).
            if on && !self.wanted {
                if let Err(e) = self.apply_lockdown() {
                    return Response::error(
                        "firewall",
                        &format!("Lockdown Mode rules could not be installed: {e}"),
                    );
                }
                return self.status().with(
                    "message",
                    "Lockdown Mode on: blocking rules installed again",
                );
            }
            return self.status().with("message", "Lockdown Mode unchanged");
        }
        if let Err(r) = self.with_store_mut(|s| {
            s.set_service(ServiceSettings {
                lockdown: on,
                ..old
            })
        }) {
            return r;
        }
        if !self.wanted {
            let applied = if on {
                self.apply_lockdown()
            } else {
                self.firewall.apply(&nft::removal_script())
            };
            if let Err(e) = applied {
                // Keep the setting and the rules consistent: undo.
                let _ = self.with_store_mut(|s| s.set_service(old));
                return Response::error("firewall", &format!("Lockdown Mode was not changed: {e}"));
            }
        }
        (self.log)(&format!(
            "Lockdown Mode turned {}",
            if on { "on" } else { "off" }
        ));
        let message = match (on, self.wanted) {
            (true, false) => "Lockdown Mode on: internet access is blocked until you connect",
            (true, true) => "Lockdown Mode on: it will keep blocking when this connection ends",
            (false, false) => "Lockdown Mode off: normal networking restored",
            (false, true) => {
                "Lockdown Mode off: a deliberate disconnect will restore normal networking"
            }
        };
        self.status().with("message", message)
    }

    /// Local Network Access Allowed or Not Allowed (D-040). The service
    /// applies it at once to the ruleset in force; if that fails the setting
    /// is not changed.
    fn set_local_network(&mut self, allowed: bool) -> Response {
        let old = self.settings();
        if let Err(r) = self.with_store_mut(|s| {
            s.set_service(ServiceSettings {
                local_network: allowed,
                ..old
            })
        }) {
            return r;
        }
        if let Err(e) = self.refresh_rules() {
            let _ = self.with_store_mut(|s| s.set_service(old));
            let _ = self.refresh_rules();
            return Response::error(
                "firewall",
                &format!("Local Network Access was not changed: {e}"),
            );
        }
        (self.log)(&format!(
            "Local Network Access set to {}",
            if allowed { "Allowed" } else { "Not Allowed" }
        ));
        let mut r = self.status();
        if allowed {
            r = r.with(
                "message",
                "Devices on this network will be able to reach you, and you them, outside the VPN.",
            );
        }
        r
    }

    fn store_error(e: &StoreError) -> Response {
        Response::error(e.code, &e.message)
    }

    fn not_configured() -> Response {
        Response::error(
            "not_configured",
            "saved connections are not enabled in the service settings",
        )
    }

    fn with_store_ref<T>(
        &self,
        f: impl FnOnce(&Store) -> Result<T, StoreError>,
    ) -> Result<T, Response> {
        let Some(store) = &self.store else {
            return Err(Self::not_configured());
        };
        f(store).map_err(|e| Self::store_error(&e))
    }

    fn with_store_mut<T>(
        &mut self,
        f: impl FnOnce(&mut Store) -> Result<T, StoreError>,
    ) -> Result<T, Response> {
        let Some(store) = &mut self.store else {
            return Err(Self::not_configured());
        };
        f(store).map_err(|e| Self::store_error(&e))
    }

    /// `selected=` and a body of `ID<TAB>name` lines, in display order.
    fn list(&self) -> Response {
        let Some(store) = &self.store else {
            return Self::not_configured();
        };
        let mut body = String::new();
        for (id, name) in store.list() {
            let _ = writeln!(body, "{id}\t{name}");
        }
        Response::ok()
            .with("count", &store.list().len().to_string())
            .with(
                "selected",
                &store.selected().map_or("none".into(), |i| i.to_string()),
            )
            .with_body(&body)
    }

    fn add(&mut self, text: &str) -> Response {
        match self.with_store_mut(|s| s.add(text)) {
            Ok(id) => {
                (self.log)(&format!("saved connection {id} added"));
                self.refresh_lockdown_endpoints();
                Response::ok()
                    .with("id", &id.to_string())
                    .with("message", "connection saved (not selected)")
            }
            Err(r) => r,
        }
    }

    fn rename(&mut self, id: ConnectionId, name: &str) -> Response {
        match self.with_store_mut(|s| s.rename(id, name)) {
            Ok(_) => Response::ok()
                .with("id", &id.to_string())
                .with("message", "renamed"),
            Err(r) => r,
        }
    }

    /// Replaces a connection's configuration. If it is the one in use, the
    /// new configuration is used from now on, and applied at once while
    /// connected (as a switch).
    fn replace(&mut self, id: ConnectionId, text: &str) -> Response {
        let config = match self.with_store_mut(|s| s.replace(id, text)) {
            Ok(c) => c,
            Err(r) => return r,
        };
        (self.log)(&format!("saved connection {id} replaced"));
        self.refresh_lockdown_endpoints();
        if self.current == Some(id) {
            return self.use_config(Some(id), config, "connection updated");
        }
        Response::ok()
            .with("id", &id.to_string())
            .with("message", "connection updated")
    }

    /// Deletes a connection. The one in use cannot be deleted while
    /// connected: disconnect first (DEFAULT-UI §4.1).
    fn delete(&mut self, id: ConnectionId) -> Response {
        if self.current == Some(id) && self.wanted {
            return Response::error(
                "in_use",
                "this connection is in use: disconnect before deleting it",
            );
        }
        let next = match self.with_store_mut(|s| s.delete(id)) {
            Ok(n) => n,
            Err(r) => return r,
        };
        (self.log)(&format!("saved connection {id} deleted"));
        self.refresh_lockdown_endpoints();
        if self.current == Some(id) {
            // Not connected: the new selection (or the default) is used next.
            let config = next.and_then(|n| self.store.as_ref().and_then(|s| s.config(n).ok()));
            self.current = config.as_ref().and(next);
            self.config = config.unwrap_or_else(|| self.default.clone());
        }
        self.status().with("message", "connection deleted")
    }

    /// Selects a connection; while connected, switches to it (D-041). A
    /// connection that cannot be used is refused and nothing changes.
    fn select(&mut self, id: ConnectionId) -> Response {
        let config = match self.with_store_ref(|s| s.config(id)) {
            Ok(c) => c,
            Err(r) => return r,
        };
        if let Err(r) = self.with_store_mut(|s| s.select(id)) {
            return r;
        }
        self.use_config(Some(id), config, "connection selected")
    }

    /// Makes `config` the one in use; while connected, replaces the rules
    /// in one transaction.
    fn use_config(
        &mut self,
        id: Option<ConnectionId>,
        config: DeviceConfig,
        done: &str,
    ) -> Response {
        self.config = config;
        self.current = id;
        if !self.wanted {
            return self.status().with("message", done);
        }
        let label = id.map_or("the default configuration".into(), |i| i.to_string());
        match self.apply_rules() {
            Ok(()) => {
                self.failure = None;
                (self.log)(&format!(
                    "switched to {label}: rules replaced in one transaction"
                ));
                let message = match self.install_resolver() {
                    Ok(()) => "switched: protection was kept throughout".to_string(),
                    // The kill switch is on, so DNS still cannot leak.
                    Err(e) => format!("switched, but {e}"),
                };
                self.status().with("message", &message)
            }
            Err(e) => {
                // The kernel kept the previous rules: still blocking.
                let why = format!(
                    "switching to {label} failed: {e}; the previous rules stay in place (still blocking)"
                );
                (self.log)(&why);
                self.failure = Some(why.clone());
                self.status().with("message", &why)
            }
        }
    }

    /// Renders and installs the kill switch for the configuration in use,
    /// in one transaction.
    fn apply_rules(&mut self) -> Result<(), String> {
        let ks = self.kill_switch();
        let script = nft::render(&self.config.policy, &ks).map_err(|e| e.to_string())?;
        self.firewall.apply(&script)?;
        self.applied_local = ks.local;
        Ok(())
    }

    fn install_resolver(&mut self) -> Result<(), String> {
        let servers = self.tunnel_resolvers().to_vec();
        self.resolver.install(&servers)
    }

    fn kill_switch(&mut self) -> KillSwitch {
        let local = self.local_for_connection();
        KillSwitch {
            local,
            tunnel_interface: self.config.tunnel.interface.clone(),
            endpoints: self
                .config
                .endpoints
                .iter()
                .map(|e| (e.ip(), e.port()))
                .collect(),
            dns: {
                let DnsPolicy::Tunnel {
                    resolvers,
                    lan_exceptions,
                } = &self.config.dns;
                DnsRules::Resolvers {
                    resolvers: resolvers.clone(),
                    lan_exceptions: lan_exceptions.clone(),
                }
            },
            captive_helper: self.captive_helper,
        }
    }

    fn tunnel_resolvers(&self) -> &[std::net::IpAddr] {
        let DnsPolicy::Tunnel { resolvers, .. } = &self.config.dns;
        resolvers
    }

    fn connect(&mut self) -> Response {
        let ks = self.kill_switch();
        let script = match nft::render(&self.config.policy, &ks) {
            Ok(s) => s,
            Err(e) => return Response::error("invalid_policy", &e.to_string()),
        };
        if let Err(e) = self.firewall.apply(&script) {
            return Response::error("firewall", &e);
        }
        self.applied_local = ks.local;
        self.wanted = true;
        self.failure = None;
        // The new table starts with empty captive-portal sets.
        self.captive_until = None;
        self.captive_network_dns = None;
        self.captive_windows = 0;
        if let Err(e) = self.install_resolver() {
            // The kill switch is on, so DNS still cannot leak; report it.
            return Response::error("resolver", &format!("kill switch on, but {e}"));
        }
        self.status().with(
            "message",
            "kill switch on: protected traffic is blocked; no tunnel exists before G1",
        )
    }

    fn disconnect(&mut self) -> Response {
        // With Lockdown Mode on, a deliberate disconnect keeps blocking
        // (D-038, amends Q2); otherwise it removes the rules.
        let lockdown = self.settings().lockdown;
        self.wanted = false;
        let removed = if lockdown {
            self.apply_lockdown()
        } else {
            self.firewall.apply(&nft::removal_script())
        };
        if let Err(e) = removed {
            self.wanted = true;
            return Response::error("firewall", &e);
        }
        self.failure = None;
        // Removing the table ended any captive-portal window at once.
        self.captive_until = None;
        self.captive_network_dns = None;
        if let Err(e) = self.resolver.restore() {
            return Response::error("resolver", &format!("rules removed, but {e}"));
        }
        self.status().with(
            "message",
            if lockdown {
                "disconnected: Lockdown Mode keeps blocking internet access until you connect"
            } else {
                "disconnected: Septuvo rules removed and the resolver file restored (deliberate disconnect)"
            },
        )
    }

    /// Opens a captive-portal window for the helper account for `minutes`
    /// (1 to 5). Asking again restarts the window from now (it never adds
    /// up) and counts as a new window.
    fn captive(&mut self, minutes: u8, network_dns: bool) -> Response {
        if !(1..=septuvo_ipc::MAX_CAPTIVE_MINUTES).contains(&minutes) {
            return Response::error(
                "invalid_request",
                "captive-portal mode lasts 1 to 5 minutes",
            );
        }
        if self.captive_helper.is_none() {
            return Response::error(
                "not_configured",
                "captive-portal mode needs a dedicated helper account (daemon.captive_helper_user)",
            );
        }
        // While connected, or while Lockdown Mode blocks (NP-K2).
        if !(self.wanted || self.lockdown_idle()) || !self.firewall.active() {
            return Response::error(
                "not_connected",
                "captive-portal mode is only available while connected or while Lockdown Mode blocks",
            );
        }
        if self.captive_windows >= MAX_CAPTIVE_WINDOWS {
            return Response::error(
                "limit",
                "at most 3 captive-portal windows per connection; disconnect and connect again",
            );
        }
        // Never automatic: the network's DNS only when asked for this window.
        let resolvers = if network_dns {
            self.resolver.network_resolvers()
        } else {
            Vec::new()
        };
        let seconds = u32::from(minutes) * 60;
        let script = match nft::captive_script(seconds, &resolvers) {
            Ok(s) => s,
            Err(e) => return Response::error("invalid_request", &e.to_string()),
        };
        if let Err(e) = self.firewall.apply(&script) {
            return Response::error("firewall", &e);
        }
        self.captive_windows += 1;
        self.captive_until = Some(Instant::now() + Duration::from_secs(u64::from(seconds)));
        self.captive_network_dns = Some(!resolvers.is_empty());
        // Every window is logged (journal), never silent. Only counts and
        // yes/no flags: no account, address or other detail is written.
        (self.log)(&format!(
            "captive-portal window {} of {MAX_CAPTIVE_WINDOWS} opened for {minutes} min \
             for the helper account (network DNS: {})",
            self.captive_windows,
            if resolvers.is_empty() { "no" } else { "yes" }
        ));
        let mut r = self.status().with("message", CAPTIVE_WARNING);
        if network_dns && resolvers.is_empty() {
            r = r.with(
                "captive_resolvers",
                "no network resolver found: web ports only (DNS over HTTPS)",
            );
        } else if !resolvers.is_empty() {
            let list: Vec<String> = resolvers.iter().map(ToString::to_string).collect();
            r = r.with(
                "captive_resolvers",
                &format!("network DNS for the helper: {}", list.join(", ")),
            );
        }
        r
    }

    fn status(&mut self) -> Response {
        let kill_switch = self.firewall.active();
        let captive = kill_switch && self.firewall.captive_open();
        if !captive {
            self.captive_until = None;
            self.captive_network_dns = None;
        }
        let adapter = if self.interfaces.is_up(&self.config.tunnel.interface) {
            "pass"
        } else {
            "fail"
        };
        let settings = self.settings();
        let state = match (self.wanted, kill_switch) {
            (true, _) if self.failure.is_some() => "failed",
            (true, true) => "blocking",
            (false, true) if settings.lockdown => "lockdown",
            (false, true) => "blocking-left-from-earlier-run",
            (true, false) => "error-kill-switch-missing",
            // Lockdown Mode is on but its rules are not in place (removed
            // by hand, or they could not be installed): never shown as an
            // ordinary disconnected state.
            (false, false) if settings.lockdown => "error-lockdown-missing",
            (false, false) => "disconnected",
        };
        // DNS leak prevention is part of the same ruleset as the kill
        // switch (one table, installed atomically), so it is verified the
        // same way: the table is in place.
        let dns_protection = match (self.wanted, kill_switch) {
            (_, true) => "on",
            (true, false) => "failed",
            (false, false) => "off",
        };
        let verified_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Response::ok()
            .with("state", state)
            .with_if(
                self.failure.is_some(),
                "failure",
                self.failure.as_deref().unwrap_or(""),
            )
            .with(
                "connection",
                &self.current.map_or("default".into(), |i| i.to_string()),
            )
            .with("kill_switch", if kill_switch { "on" } else { "off" })
            .with("dns_protection", dns_protection)
            .with("lockdown", if settings.lockdown { "on" } else { "off" })
            .with(
                "local_network",
                if settings.local_network {
                    "allowed"
                } else {
                    "not-allowed"
                },
            )
            .with_if(
                kill_switch && !self.applied_local.is_empty(),
                "local_subnets",
                &self.local_summary(),
            )
            .with("verified_at", &verified_at.to_string())
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
            .with("captive_portal", self.captive_state(captive))
            .with_if(captive, "captive_remaining", &self.captive_remaining())
            .with_if(captive, "captive_warning", CAPTIVE_WARNING)
            .with_if(captive, "captive_dns", self.captive_dns())
            .with_if(
                self.captive_helper.is_some(),
                "captive_helper",
                &self
                    .captive_helper
                    .map(|u| format!("uid {u}"))
                    .unwrap_or_default(),
            )
            .with_if(
                self.captive_helper.is_some() && self.wanted,
                "captive_windows_left",
                &(MAX_CAPTIVE_WINDOWS - self.captive_windows.min(MAX_CAPTIVE_WINDOWS)).to_string(),
            )
            .with("note", PEER_NOTE)
    }

    /// The opened local subnets, for status.
    fn local_summary(&self) -> String {
        let mut list: Vec<String> = self
            .applied_local
            .subnets
            .iter()
            .map(ToString::to_string)
            .collect();
        if self.applied_local.ipv6_link_local {
            list.push("fe80::/10".into());
        }
        list.join(", ")
    }

    fn captive_state(&self, open: bool) -> &'static str {
        match (self.captive_helper, open) {
            (_, true) => "open",
            (Some(_), false) => "closed",
            (None, false) => "not-configured",
        }
    }

    fn captive_dns(&self) -> &'static str {
        match self.captive_network_dns {
            Some(true) => NETWORK_DNS_WARNING,
            Some(false) => {
                "helper: encrypted DNS only (DNS over HTTPS); the network's DNS is not opened"
            }
            None => "unknown (window opened before this service started)",
        }
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
        let list: Vec<String> = self
            .tunnel_resolvers()
            .iter()
            .map(ToString::to_string)
            .collect();
        format!("only through the tunnel to {}", list.join(", "))
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

    const HELPER: u32 = 995;

    fn engine(fw: FakeFirewall, up: bool) -> Engine<FakeFirewall, FakeInterfaces, FakeResolver> {
        Engine::new(config(), fw, FakeInterfaces(up), FakeResolver::default())
            .with_captive_helper(Some(HELPER))
    }

    fn captive(minutes: u8, network_dns: bool) -> Command {
        Command::Captive {
            minutes,
            network_dns,
        }
    }

    fn code(r: &Response) -> &str {
        match &r.outcome {
            Outcome::Err { code, .. } => code,
            Outcome::Ok => "ok",
        }
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

    // --- Lockdown Mode and Local Network Access (N4-04) ------------------

    #[derive(Clone, Default)]
    struct FakeAddresses(
        std::sync::Arc<std::sync::Mutex<Vec<septuvo_policy::lan::InterfaceAddress>>>,
    );

    impl Addresses for FakeAddresses {
        fn current(&mut self) -> Vec<septuvo_policy::lan::InterfaceAddress> {
            self.0.lock().unwrap().clone()
        }
    }

    impl FakeAddresses {
        fn set(&self, list: &[(&str, &str)]) {
            *self.0.lock().unwrap() = list
                .iter()
                .map(|(iface, cidr)| {
                    let (a, l) = cidr.split_once('/').unwrap();
                    septuvo_policy::lan::InterfaceAddress {
                        interface: (*iface).into(),
                        addr: a.parse().unwrap(),
                        prefix_len: l.parse().unwrap(),
                    }
                })
                .collect();
        }
    }

    fn lab(
        tag: &str,
    ) -> (
        Engine<FakeFirewall, FakeInterfaces, FakeResolver>,
        FakeAddresses,
        std::path::PathBuf,
    ) {
        let dir = store_dir(tag);
        let addrs = FakeAddresses::default();
        addrs.set(&[
            ("lo", "127.0.0.1/8"),
            ("eth0", "192.168.1.30/24"),
            ("eth0", "fe80::1/64"),
        ]);
        let e = engine(FakeFirewall::default(), false)
            .with_store(Store::open(&dir, false).unwrap())
            .with_addresses(Box::new(addrs.clone()));
        (e, addrs, dir)
    }

    #[test]
    fn lockdown_is_off_by_default_and_blocks_when_turned_on() {
        let (mut e, _, _) = lab("ld-on");
        let r = e.handle(Command::Status, false);
        assert_eq!(
            (r.field("lockdown"), r.field("local_network")),
            (Some("off"), Some("not-allowed"))
        );
        assert_eq!(
            code(&e.handle(Command::Lockdown(true), false)),
            "not_permitted"
        );
        let r = e.handle(Command::Lockdown(true), true);
        assert_eq!(r.field("state"), Some("lockdown"));
        assert_eq!(r.field("kill_switch"), Some("on"));
        assert!(rules(&e).contains("Lockdown Mode"));
        assert!(
            rules(&e).contains("203.0.113.7 udp dport 51820 accept"),
            "the user can still connect"
        );
        // Off again: normal networking.
        let r = e.handle(Command::Lockdown(false), true);
        assert_eq!(r.field("state"), Some("disconnected"));
        assert!(e.firewall.installed.is_none());
    }

    /// D-038: a deliberate disconnect keeps blocking with Lockdown Mode on.
    #[test]
    fn lockdown_keeps_blocking_after_disconnect() {
        let (mut e, _, _) = lab("ld-disc");
        let _ = e.handle(Command::Connect, true);
        let r = e.handle(Command::Lockdown(true), true);
        assert!(
            r.field("message")
                .is_some_and(|m| m.contains("when this connection ends"))
        );
        assert!(
            !rules(&e).contains("Lockdown Mode"),
            "nothing changes while connected"
        );
        let r = e.handle(Command::Disconnect, true);
        assert_eq!(r.outcome, Outcome::Ok);
        assert_eq!(r.field("state"), Some("lockdown"));
        assert!(rules(&e).contains("Lockdown Mode"));
    }

    /// Rules removed by hand show as an error; turning Lockdown Mode on
    /// again installs them again.
    #[test]
    fn lockdown_on_again_repairs_missing_rules() {
        let (mut e, _, dir) = lab("ld-repair");
        let _ = e.handle(Command::Lockdown(true), true);
        let mut again =
            engine(FakeFirewall::default(), false).with_store(Store::open(&dir, false).unwrap());
        assert_eq!(
            again.handle(Command::Status, false).field("state"),
            Some("error-lockdown-missing")
        );
        let r = again.handle(Command::Lockdown(true), true);
        assert_eq!(r.outcome, Outcome::Ok);
        assert_eq!(r.field("state"), Some("lockdown"));
        assert!(rules(&again).contains("Lockdown Mode"));
    }

    /// Across a service restart (and from boot): the start-up step installs
    /// the Lockdown Mode ruleset again.
    #[test]
    fn lockdown_survives_a_restart() {
        let (mut e, _, dir) = lab("ld-restart");
        let _ = e.handle(Command::Lockdown(true), true);
        let mut again =
            engine(FakeFirewall::default(), false).with_store(Store::open(&dir, false).unwrap());
        // Before the start-up step the rules are missing: an error, never
        // an ordinary "disconnected".
        assert_eq!(
            again.handle(Command::Status, false).field("state"),
            Some("error-lockdown-missing")
        );
        assert_eq!(again.apply_startup(), Ok(true));
        assert!(rules(&again).contains("Lockdown Mode"));
        assert_eq!(
            again.handle(Command::Status, false).field("state"),
            Some("lockdown")
        );
        // With Lockdown Mode off the start-up step changes nothing.
        let mut off = engine(FakeFirewall::default(), false)
            .with_store(Store::open(&store_dir("ld-off"), false).unwrap());
        assert_eq!(off.apply_startup(), Ok(false));
        assert_eq!(off.firewall.applied, 0);
    }

    #[test]
    fn lockdown_lets_the_user_reach_every_saved_connection() {
        let (mut e, _, _) = lab("ld-endpoints");
        let _ = add(&mut e, &conn("A", "198.51.100.1:5000"));
        let _ = add(&mut e, &conn("B", "[2001:db8::7]:443"));
        let _ = e.handle(Command::Lockdown(true), true);
        let s = rules(&e);
        assert!(s.contains("ip daddr 198.51.100.1 udp dport 5000 accept"));
        assert!(s.contains("ip6 daddr 2001:db8::7 udp dport 443 accept"));
        // A connection saved while Lockdown Mode blocks becomes reachable at once.
        let _ = add(&mut e, &conn("C", "198.51.100.9:6000"));
        assert!(rules(&e).contains("ip daddr 198.51.100.9 udp dport 6000 accept"));
    }

    #[test]
    fn a_failed_lockdown_change_changes_nothing() {
        let (mut e, _, _) = lab("ld-fail");
        e.firewall.fail = true;
        let r = e.handle(Command::Lockdown(true), true);
        assert_eq!(code(&r), "firewall");
        assert_eq!(
            e.handle(Command::Status, false).field("lockdown"),
            Some("off")
        );
    }

    #[test]
    fn captive_portal_works_while_lockdown_blocks() {
        let (mut e, _, _) = lab("ld-captive");
        assert_eq!(code(&e.handle(captive(1, false), true)), "not_connected");
        let _ = e.handle(Command::Lockdown(true), true);
        assert_eq!(e.handle(captive(1, false), true).outcome, Outcome::Ok);
    }

    #[test]
    fn local_network_access_opens_only_the_attached_subnet() {
        let (mut e, addrs, _) = lab("lan");
        let _ = e.handle(Command::Connect, true);
        assert!(!rules(&e).contains("192.168.1.0/24"));
        let r = e.handle(Command::LocalNetwork(true), true);
        assert_eq!(r.field("local_network"), Some("allowed"));
        assert!(
            r.field("message")
                .is_some_and(|m| m.contains("outside the VPN"))
        );
        assert_eq!(r.field("local_subnets"), Some("192.168.1.0/24, fe80::/10"));
        assert!(rules(&e).contains("ip daddr 192.168.1.0/24 accept"));
        // NP-L6: a new network replaces the old subnet at once.
        addrs.set(&[("eth0", "10.9.8.7/24")]);
        assert!(e.addresses_changed());
        assert!(rules(&e).contains("ip daddr 10.9.8.0/24 accept"));
        assert!(
            !rules(&e).contains("192.168.1.0/24"),
            "no stale subnet stays open"
        );
        assert!(
            !e.addresses_changed(),
            "nothing changed: nothing re-applied"
        );
        // NP-L3: a hostile, huge prefix opens nothing.
        addrs.set(&[("eth0", "10.9.8.7/8")]);
        assert!(e.addresses_changed());
        assert!(!rules(&e).contains("10.0.0.0/8"));
        // Not Allowed again: the subnets are gone.
        addrs.set(&[("eth0", "192.168.1.30/24")]);
        let _ = e.handle(Command::LocalNetwork(false), true);
        assert!(!rules(&e).contains("192.168.1.0/24"));
    }

    #[test]
    fn local_network_access_never_opens_the_server_or_tunnel_prefixes() {
        let (mut e, addrs, _) = lab("lan-overlap");
        // The endpoint 203.0.113.7 sits in the attached subnet: not opened.
        addrs.set(&[("eth0", "203.0.113.20/24")]);
        let _ = e.handle(Command::Connect, true);
        let _ = e.handle(Command::LocalNetwork(true), true);
        assert!(!rules(&e).contains("203.0.113.0/24 accept"));
    }

    #[test]
    fn local_network_access_applies_in_lockdown_too() {
        let (mut e, _, _) = lab("lan-ld");
        let _ = e.handle(Command::Lockdown(true), true);
        let _ = e.handle(Command::LocalNetwork(true), true);
        let s = rules(&e);
        assert!(s.contains("Lockdown Mode") && s.contains("ip daddr 192.168.1.0/24 accept"));
    }

    #[test]
    fn a_failed_local_network_change_is_undone() {
        let (mut e, _, _) = lab("lan-fail");
        let _ = e.handle(Command::Connect, true);
        e.firewall.fail = true;
        let r = e.handle(Command::LocalNetwork(true), true);
        assert_eq!(code(&r), "firewall");
        assert_eq!(
            e.handle(Command::Status, false).field("local_network"),
            Some("not-allowed")
        );
    }

    // --- saved connections and switching (N4-03) -------------------------

    fn conn(name: &str, endpoint: &str) -> String {
        format!(
            "version = 1\n[server]\nendpoints = [\"{endpoint}\"]\n[routing]\nmode = \"full-tunnel\"\n\
             [dns]\nresolvers = [\"10.77.0.1\"]\n[connection]\nname = \"{name}\"\n"
        )
    }

    fn store_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("septuvo-engine-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn with_store(tag: &str) -> Engine<FakeFirewall, FakeInterfaces, FakeResolver> {
        let store = Store::open(&store_dir(tag), false).expect("store");
        engine(FakeFirewall::default(), false).with_store(store)
    }

    fn add(e: &mut Engine<FakeFirewall, FakeInterfaces, FakeResolver>, text: &str) -> ConnectionId {
        let r = e.handle_request(&Request::with_body(Command::Add, text), true);
        ConnectionId::parse(r.field("id").expect("id")).expect("valid id")
    }

    fn rules(e: &Engine<FakeFirewall, FakeInterfaces, FakeResolver>) -> String {
        e.firewall.installed.clone().unwrap_or_default()
    }

    #[test]
    fn connections_are_listed_by_name_for_everyone() {
        let mut e = with_store("list");
        let a = add(&mut e, &conn("Home VPN", "203.0.113.1:5000"));
        let _ = add(&mut e, &conn("Büro", "203.0.113.2:5000"));
        let r = e.handle(Command::List, false);
        assert_eq!(r.outcome, Outcome::Ok);
        assert_eq!(r.field("count"), Some("2"));
        assert_eq!(r.field("selected"), Some("none"));
        assert_eq!(r.body.as_deref(), Some("c0001\tHome VPN\nc0002\tBüro\n"));
        // The configuration itself needs the control socket.
        assert_eq!(code(&e.handle(Command::Get(a), false)), "not_permitted");
        let g = e.handle(Command::Get(a), true);
        assert!(
            g.body.as_deref().is_some_and(
                |b| b.contains("203.0.113.1:5000") && b.contains("name = \"Home VPN\"")
            )
        );
        assert_eq!(e.firewall.applied, 0, "nothing touched the network");
    }

    #[test]
    fn selecting_while_disconnected_changes_no_rules() {
        let mut e = with_store("select");
        let a = add(&mut e, &conn("A", "203.0.113.1:5000"));
        let r = e.handle(Command::Select(a), true);
        assert_eq!(r.field("connection"), Some("c0001"));
        assert_eq!(e.firewall.applied, 0);
        assert_eq!(e.handle(Command::Connect, true).outcome, Outcome::Ok);
        assert!(
            rules(&e).contains("203.0.113.1"),
            "connect uses the selection"
        );
    }

    /// D-041: switching while connected replaces the rules in one firewall
    /// transaction; there is never a moment without them.
    #[test]
    fn switching_while_connected_is_one_transaction() {
        let mut e = with_store("switch");
        let a = add(&mut e, &conn("A", "203.0.113.1:5000"));
        let b = add(&mut e, &conn("B", "203.0.113.2:443"));
        let _ = e.handle(Command::Select(a), true);
        let _ = e.handle(Command::Connect, true);
        let before = e.firewall.applied;
        let r = e.handle(Command::Select(b), true);
        assert_eq!(r.outcome, Outcome::Ok);
        assert_eq!(e.firewall.applied, before + 1, "exactly one transaction");
        let script = rules(&e);
        // One script: the old table is replaced, not removed first.
        assert!(script.contains("\nadd table inet septuvo\nflush table inet septuvo\n"));
        assert!(script.contains("203.0.113.2") && !script.contains("203.0.113.1"));
        assert_eq!(r.field("state"), Some("blocking"));
        assert_eq!(r.field("connection"), Some("c0002"));
        assert_eq!(r.field("kill_switch"), Some("on"));
        assert_eq!(r.field("dns_protection"), Some("on"));
        assert!(
            r.field("verified_at")
                .is_some_and(|t| t.parse::<u64>().is_ok())
        );
    }

    /// A failed switch keeps the new selection, keeps blocking (the kernel
    /// kept the old rules) and never resumes the old connection by itself.
    #[test]
    fn a_failed_switch_fails_closed() {
        let mut e = with_store("switch-fail");
        let a = add(&mut e, &conn("A", "203.0.113.1:5000"));
        let b = add(&mut e, &conn("B", "203.0.113.2:443"));
        let _ = e.handle(Command::Select(a), true);
        let _ = e.handle(Command::Connect, true);
        let old = rules(&e);
        e.firewall.fail = true;
        let r = e.handle(Command::Select(b), true);
        assert_eq!(r.field("state"), Some("failed"));
        assert!(
            r.field("failure")
                .is_some_and(|f| f.contains("still blocking"))
        );
        assert_eq!(
            r.field("connection"),
            Some("c0002"),
            "the new selection is kept"
        );
        assert_eq!(rules(&e), old, "the previous rules are still in force");
        assert_eq!(r.field("kill_switch"), Some("on"));
        assert_eq!(e.store.as_ref().and_then(Store::selected), Some(b));
        // Retrying works once the firewall does; the failure clears.
        e.firewall.fail = false;
        let r = e.handle(Command::Connect, true);
        assert_eq!(r.field("state"), Some("blocking"));
        assert!(rules(&e).contains("203.0.113.2"));
        assert_eq!(r.field("failure"), None);
    }

    #[test]
    fn a_bad_connection_changes_nothing() {
        let mut e = with_store("bad");
        let a = add(&mut e, &conn("A", "203.0.113.1:5000"));
        let _ = e.handle(Command::Select(a), true);
        let _ = e.handle(Command::Connect, true);
        let (old_rules, applied) = (rules(&e), e.firewall.applied);
        let bad = conn("A", "203.0.113.1:5000").replace("10.77.0.1", "10.77.0.999");
        for r in [
            e.handle_request(&Request::with_body(Command::Add, bad.as_str()), true),
            e.handle_request(&Request::with_body(Command::Replace(a), bad.as_str()), true),
            e.handle_request(&Request::with_body(Command::Rename(a), ""), true),
            e.handle(Command::Select(ConnectionId::new(77).unwrap()), true),
            e.handle(Command::Delete(a), true),
        ] {
            assert!(matches!(r.outcome, Outcome::Err { .. }), "{r:?}");
        }
        assert_eq!((rules(&e), e.firewall.applied), (old_rules, applied));
        assert_eq!(code(&e.handle(Command::Delete(a), true)), "in_use");
        let r = e.handle(Command::List, false);
        assert_eq!(r.body.as_deref(), Some("c0001\tA\n"));
    }

    #[test]
    fn replacing_the_connection_in_use_applies_it_while_connected() {
        let mut e = with_store("replace");
        let a = add(&mut e, &conn("A", "203.0.113.1:5000"));
        let _ = e.handle(Command::Select(a), true);
        let _ = e.handle(Command::Connect, true);
        let r = e.handle_request(
            &Request::with_body(Command::Replace(a), conn("A", "203.0.113.9:5000")),
            true,
        );
        assert_eq!(r.outcome, Outcome::Ok);
        assert!(rules(&e).contains("203.0.113.9"));
        // Renaming has no network effect at all.
        let applied = e.firewall.applied;
        let r = e.handle_request(&Request::with_body(Command::Rename(a), "Home"), true);
        assert_eq!(r.outcome, Outcome::Ok);
        assert_eq!(e.firewall.applied, applied);
    }

    #[test]
    fn deleting_the_selected_connection_selects_the_next_one() {
        let mut e = with_store("delete");
        let a = add(&mut e, &conn("A", "203.0.113.1:5000"));
        let b = add(&mut e, &conn("B", "203.0.113.2:5000"));
        let _ = e.handle(Command::Select(a), true);
        let r = e.handle(Command::Delete(a), true);
        assert_eq!(r.outcome, Outcome::Ok);
        assert_eq!(r.field("connection"), Some("c0002"));
        let r = e.handle(Command::Delete(b), true);
        assert_eq!(
            r.field("connection"),
            Some("default"),
            "back to the settings' configuration"
        );
    }

    #[test]
    fn the_selection_survives_a_restart() {
        let dir = store_dir("restart");
        let mut e =
            engine(FakeFirewall::default(), false).with_store(Store::open(&dir, false).unwrap());
        let _ = add(&mut e, &conn("A", "203.0.113.1:5000"));
        let b = add(&mut e, &conn("B", "203.0.113.2:5000"));
        let _ = e.handle(Command::Select(b), true);
        let mut again =
            engine(FakeFirewall::default(), false).with_store(Store::open(&dir, false).unwrap());
        assert_eq!(
            again.handle(Command::Status, false).field("connection"),
            Some("c0002")
        );
        let _ = again.handle(Command::Connect, true);
        assert!(rules(&again).contains("203.0.113.2"));
    }

    #[test]
    fn without_a_store_connection_commands_are_refused() {
        let mut e = engine(FakeFirewall::default(), false);
        assert_eq!(code(&e.handle(Command::List, false)), "not_configured");
        let r = e.handle_request(&Request::with_body(Command::Add, "x"), true);
        assert_eq!(code(&r), "not_configured");
        assert_eq!(
            e.handle(Command::Status, false).field("connection"),
            Some("default")
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
    fn captive_portal_needs_a_helper_the_control_socket_and_a_connection() {
        let mut none = Engine::new(
            config(),
            FakeFirewall::default(),
            FakeInterfaces(false),
            FakeResolver::default(),
        );
        let _ = none.handle(Command::Connect, true);
        assert_eq!(
            code(&none.handle(captive(5, false), true)),
            "not_configured"
        );
        let installed = none.firewall.installed.clone().unwrap();
        // The script's start only deletes any old captive sets.
        let (_, table) = installed.split_once("table inet septuvo {").unwrap();
        assert!(
            !table.contains("captive"),
            "no helper, no captive rules or sets at all"
        );
        assert_eq!(
            none.handle(Command::Status, false).field("captive_portal"),
            Some("not-configured")
        );
        let root = Engine::new(
            config(),
            FakeFirewall::default(),
            FakeInterfaces(false),
            FakeResolver::default(),
        )
        .with_captive_helper(Some(0));
        assert_eq!(root.captive_helper, None, "root is never the helper");

        let mut e = engine(FakeFirewall::default(), false);
        assert_eq!(code(&e.handle(captive(5, false), true)), "not_connected");
        let _ = e.handle(Command::Connect, true);
        let applied = e.firewall.applied;
        assert_eq!(code(&e.handle(captive(5, false), false)), "not_permitted");
        for minutes in [0, 6, 255] {
            assert_eq!(
                code(&e.handle(captive(minutes, false), true)),
                "invalid_request"
            );
        }
        assert_eq!(e.firewall.applied, applied, "refusals change nothing");
        assert_eq!(e.firewall.captive, None);
        let s = e.handle(Command::Status, false);
        assert_eq!(s.field("captive_portal"), Some("closed"));
        assert_eq!(s.field("captive_warning"), None);
    }

    #[test]
    fn the_rules_name_the_helper_and_trusted_dns_is_the_default() {
        let mut e = engine(FakeFirewall::default(), false);
        let _ = e.handle(Command::Connect, true);
        let rules = e.firewall.installed.clone().unwrap();
        assert!(rules.contains("meta skuid 995 tcp dport @captive_web accept"));
        assert!(rules.contains("meta skuid 995 oifname \"septuvo0\" drop"));
        let r = e.handle(captive(5, false), true);
        assert_eq!(r.outcome, Outcome::Ok);
        let script = e.firewall.captive.clone().expect("window opened");
        assert!(script.contains("captive_web { 80 timeout 300s, 443 timeout 300s }"));
        assert!(
            !script.contains("captive_dns4 {"),
            "no fallback to the network's DNS unless explicitly authorized"
        );
        assert_eq!(r.field("captive_portal"), Some("open"));
        assert!((295..=300).contains(&remaining(&r)));
        assert_eq!(r.field("captive_warning"), Some(CAPTIVE_WARNING));
        assert!(
            r.field("captive_dns")
                .unwrap()
                .contains("encrypted DNS only")
        );
        assert_eq!(r.field("captive_windows_left"), Some("2"));
        assert_eq!(r.field("peer"), Some("fail"), "still never connected");
        assert_eq!(
            e.resolver.servers,
            Some(vec!["10.77.0.1".parse().unwrap()]),
            "the resolver file still names the tunnel resolver"
        );
    }

    #[test]
    fn network_dns_only_when_explicitly_authorized_and_only_for_the_window() {
        let mut e = engine(FakeFirewall::default(), false);
        let _ = e.handle(Command::Connect, true);
        let r = e.handle(captive(2, true), true);
        assert_eq!(r.outcome, Outcome::Ok);
        let script = e.firewall.captive.clone().unwrap();
        assert!(script.contains("captive_dns4 { 192.168.1.1 timeout 120s }"));
        assert!(!script.contains("10.77.0.1"), "never the tunnel resolver");
        assert_eq!(r.field("captive_dns"), Some(NETWORK_DNS_WARNING));
        let r = e.handle(captive(1, false), true);
        assert!(
            remaining(&r) <= 60,
            "asking again restarts from now, never adds up"
        );
        let script = e.firewall.captive.clone().unwrap();
        assert!(script.contains("80 timeout 60s"));
        assert!(
            script.contains("flush set inet septuvo captive_dns4"),
            "the next window drops the earlier DNS authorization"
        );
        assert!(!script.contains("captive_dns4 {"));
    }

    #[test]
    fn repeated_requests_cannot_keep_a_window_open_indefinitely() {
        let mut e = engine(FakeFirewall::default(), false);
        let _ = e.handle(Command::Connect, true);
        for left in ["2", "1", "0"] {
            let r = e.handle(captive(5, false), true);
            assert_eq!(r.field("captive_windows_left"), Some(left));
        }
        let applied = e.firewall.applied;
        let r = e.handle(captive(5, false), true);
        assert_eq!(code(&r), "limit");
        assert_eq!(
            e.firewall.applied, applied,
            "the fourth request changes nothing"
        );
        e.firewall.captive = None; // the kernel ended the last window
        assert_eq!(
            code(&e.handle(captive(1, false), true)),
            "limit",
            "still refused after the window ends"
        );
        let _ = e.handle(Command::Disconnect, true);
        let _ = e.handle(Command::Connect, true);
        assert_eq!(
            e.handle(captive(1, false), true).outcome,
            Outcome::Ok,
            "a deliberate reconnect starts over"
        );
    }

    #[test]
    fn captive_portal_ends_on_disconnect_reconnect_and_kernel_expiry() {
        let mut e = engine(FakeFirewall::default(), false);
        let _ = e.handle(Command::Connect, true);
        let _ = e.handle(captive(5, false), true);
        let r = e.handle(Command::Disconnect, true);
        assert_eq!(r.field("captive_portal"), Some("closed"));
        assert_eq!(e.firewall.captive, None);

        let _ = e.handle(Command::Connect, true);
        let _ = e.handle(captive(5, false), true);
        let r = e.handle(Command::Connect, true);
        assert_eq!(
            r.field("captive_portal"),
            Some("closed"),
            "a new table starts closed"
        );

        let _ = e.handle(captive(5, false), true);
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
        assert!(s.field("captive_dns").unwrap().starts_with("unknown"));
        assert_eq!(
            code(&e.handle(captive(5, false), true)),
            "not_connected",
            "rules from an earlier run must be reconnected first"
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
