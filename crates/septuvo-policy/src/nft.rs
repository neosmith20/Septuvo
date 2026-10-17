//! Renders a [`Policy`] as an nftables kill-switch ruleset for Linux (N1-03).
//!
//! Pure text generation, no system calls: the caller applies the script with
//! `nft -f` (atomic) and removes it with [`removal_script`]. The ruleset follows
//! `docs/spec/network-policies.md`:
//!
//! - the **firewall** is the kill switch (NP-E1): the output chain drops by
//!   default and only the tunnel interface may carry protected traffic, so a
//!   missing or down tunnel blocks it whatever the routing table says;
//! - IPv4 and IPv6 are handled together in one `inet` table (NP-E3);
//! - only the underlay exceptions NP-U1 to NP-U4 come before the policy rules:
//!   loopback, DHCP, IPv6 neighbour discovery (hop limit 255 only, so it cannot
//!   be routed) and UDP to the configured server endpoints;
//! - DNS (NP-D1): with [`DnsRules::TunnelOnly`] (the N1 behaviour) full-tunnel
//!   mode drops DNS that does not use the tunnel; with
//!   [`DnsRules::Resolvers`] (N2-03) DNS is allowed **only** to the configured
//!   resolvers through the tunnel, plus explicitly listed LAN resolvers, and
//!   all other DNS (UDP/TCP 53, DNS over TLS on TCP 853, DNS over QUIC on
//!   UDP 853) is dropped in either mode. DNS over HTTPS uses port 443 like
//!   any web traffic and cannot be told apart by port: see `docs/build/lab.md`;
//! - policy rules follow in most-specific-first order, and every rule is
//!   terminal, so a less specific rule can never re-allow a tunnel prefix.
//!
//! - captive-portal mode (Q1, N3-02, revised after the owner's review of
//!   PR #63) exists **only for one dedicated helper account**
//!   ([`KillSwitch::captive_helper`]), never for ordinary applications. Every
//!   rule that mentions the helper matches its socket owner (`meta skuid`), so
//!   the rules for every other program are exactly the kill switch without
//!   captive-portal mode. The helper is isolated at all times: never through
//!   the tunnel, never to a destination the policy names, never to the
//!   server endpoints. While a window is open it may reach web servers (TCP
//!   80 and 443) outside the tunnel, which covers encrypted DNS over HTTPS;
//!   plain DNS to the network's resolvers only if that was explicitly
//!   authorized for the window. The window lives in three **timeout sets**
//!   that [`captive_script`] fills with elements of at most
//!   [`MAX_CAPTIVE_SECONDS`]; the **kernel** removes them when the time is
//!   up, so a crashed or killed service cannot leave the window open.
//!
//! Scope: locally generated traffic (the output hook). Forwarded traffic and
//! routes are not rendered here.

use crate::policy::{Action, Mode, Policy};
use crate::prefix::Family;
use std::fmt::{self, Write};
use std::net::IpAddr;

/// nftables table that holds every Septuvo rule.
pub const TABLE: &str = "inet septuvo";
/// The table's address family, for `nft list table FAMILY NAME`.
pub const TABLE_FAMILY: &str = "inet";
/// The table's name, for `nft list table FAMILY NAME`.
pub const TABLE_NAME: &str = "septuvo";
/// Maximum number of server endpoints in one ruleset.
pub const MAX_ENDPOINTS: usize = 16;
/// Maximum number of resolver plus LAN DNS exception addresses in one ruleset.
pub const MAX_DNS_ADDRESSES: usize = 16;
/// Longest captive-portal window, in seconds (Q1: five minutes).
pub const MAX_CAPTIVE_SECONDS: u32 = 300;
/// Most network resolvers opened during a captive-portal window.
pub const MAX_CAPTIVE_RESOLVERS: usize = 4;
/// Set of TCP ports (80, 443) the helper may use outside the tunnel during
/// the window.
pub const CAPTIVE_WEB_SET: &str = "captive_web";
/// Set of IPv4 network resolvers the helper may query during the window
/// (only when explicitly authorized).
pub const CAPTIVE_DNS4_SET: &str = "captive_dns4";
/// Set of IPv6 network resolvers the helper may query during the window
/// (only when explicitly authorized).
pub const CAPTIVE_DNS6_SET: &str = "captive_dns6";

/// How the kill switch treats DNS.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DnsRules {
    /// N1 behaviour: in full-tunnel mode, DNS that does not leave through the
    /// tunnel interface is dropped; split mode adds no DNS rules.
    #[default]
    TunnelOnly,
    /// DNS only to `resolvers`, only through the tunnel; DNS directly to the
    /// `lan_exceptions`; every other DNS packet dropped, in both modes (N2-03).
    Resolvers {
        /// Resolvers reached through the tunnel.
        resolvers: Vec<IpAddr>,
        /// Local resolvers allowed to receive DNS directly.
        lan_exceptions: Vec<IpAddr>,
    },
    /// The operating system's DNS, unrestricted. Split mode only.
    System,
}

/// Settings the kill switch needs besides the policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillSwitch {
    /// Name of the tunnel interface, for example `septuvo0`.
    pub tunnel_interface: String,
    /// Septuvo Server endpoints (address and UDP port) the device may reach
    /// outside the tunnel (NP-U3).
    pub endpoints: Vec<(IpAddr, u16)>,
    /// DNS handling.
    pub dns: DnsRules,
    /// User ID of the dedicated captive-portal helper account, or `None` if
    /// captive-portal mode is not configured (then no captive rule or set is
    /// rendered at all). Never 0 (root).
    pub captive_helper: Option<u32>,
}

/// Why a ruleset could not be rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// The interface name is empty, longer than 15 bytes, or contains
    /// characters other than ASCII letters, digits, `_`, `-` and `.`.
    BadInterfaceName,
    /// An endpoint uses port 0.
    BadEndpointPort,
    /// More than [`MAX_ENDPOINTS`] endpoints.
    TooManyEndpoints,
    /// More than [`MAX_DNS_ADDRESSES`] resolver and LAN DNS addresses, or no
    /// resolver at all.
    BadDnsAddresses,
    /// [`DnsRules::System`] with a full-tunnel policy would leak DNS.
    SystemDnsInFullTunnel,
    /// A captive-portal window of 0 seconds or longer than
    /// [`MAX_CAPTIVE_SECONDS`].
    BadCaptiveDuration,
    /// More than [`MAX_CAPTIVE_RESOLVERS`] network resolvers, or an address
    /// that cannot be a resolver (unspecified or multicast).
    BadCaptiveResolvers,
    /// The captive-portal helper would be root (user ID 0).
    BadCaptiveHelper,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::BadInterfaceName => f.write_str(
                "tunnel interface name must be 1-15 characters: letters, digits, '_', '-' or '.'",
            ),
            RenderError::BadEndpointPort => f.write_str("endpoint port must be 1-65535"),
            RenderError::TooManyEndpoints => {
                write!(f, "at most {MAX_ENDPOINTS} server endpoints")
            }
            RenderError::BadDnsAddresses => write!(
                f,
                "between 1 and {MAX_DNS_ADDRESSES} DNS resolver and exception addresses are required"
            ),
            RenderError::SystemDnsInFullTunnel => {
                f.write_str("system DNS would send queries outside the tunnel in full-tunnel mode")
            }
            RenderError::BadCaptiveDuration => write!(
                f,
                "a captive-portal window lasts between 1 and {MAX_CAPTIVE_SECONDS} seconds"
            ),
            RenderError::BadCaptiveResolvers => write!(
                f,
                "at most {MAX_CAPTIVE_RESOLVERS} network resolvers, each a unicast address"
            ),
            RenderError::BadCaptiveHelper => {
                f.write_str("the captive-portal helper must be a dedicated account, never root")
            }
        }
    }
}

impl std::error::Error for RenderError {}

/// Interface names are written into the ruleset, so they are validated
/// strictly: this is hostile input as far as the generator is concerned.
fn valid_interface(name: &str) -> bool {
    (1..=15).contains(&name.len())
        && name != "."
        && name != ".."
        && !name.starts_with('-')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

fn daddr(family: Family) -> &'static str {
    match family {
        Family::V4 => "ip daddr",
        Family::V6 => "ip6 daddr",
    }
}

/// Renders a complete `nft -f` script that atomically replaces any previous
/// Septuvo table with the kill switch for `policy`.
///
/// # Errors
/// See [`RenderError`].
pub fn render(policy: &Policy, ks: &KillSwitch) -> Result<String, RenderError> {
    validate(policy, ks)?;
    let tun = &ks.tunnel_interface;
    let mut s = String::new();
    // `add` then `delete` makes replacement work whether or not the table
    // exists; nft applies the whole file as one transaction (NP-E6).
    let _ = writeln!(
        s,
        "# Septuvo kill switch, generated from a validated policy"
    );
    let _ = writeln!(s, "add table {TABLE}");
    let _ = writeln!(s, "delete table {TABLE}");
    let _ = writeln!(s, "table {TABLE} {{");
    if ks.captive_helper.is_some() {
        render_captive_sets(&mut s);
    }
    let _ = writeln!(s, "\tchain output {{");
    let _ = writeln!(
        s,
        "\t\ttype filter hook output priority filter; policy drop;"
    );
    let _ = writeln!(s, "\t\t# NP-U4 loopback");
    let _ = writeln!(s, "\t\toifname \"lo\" accept");
    if let Some(uid) = ks.captive_helper {
        render_captive_helper(&mut s, policy, tun, uid);
    }
    let _ = writeln!(s, "\t\t# NP-U1 DHCPv4 and DHCPv6");
    let _ = writeln!(s, "\t\tudp sport 68 udp dport 67 accept");
    let _ = writeln!(s, "\t\tudp sport 546 udp dport 547 accept");
    let _ = writeln!(
        s,
        "\t\t# NP-U2 IPv6 neighbour discovery (hop limit 255: never routed)"
    );
    let _ = writeln!(
        s,
        "\t\ticmpv6 type {{ nd-router-solicit, nd-neighbor-solicit, nd-neighbor-advert }} ip6 hoplimit 255 accept"
    );
    let _ = writeln!(s, "\t\t# NP-U3 Septuvo Server endpoints");
    for &(addr, port) in &ks.endpoints {
        let _ = writeln!(
            s,
            "\t\t{} {addr} udp dport {port} accept",
            daddr(family_of(addr))
        );
    }
    render_dns(&mut s, policy, tun, &ks.dns);
    let _ = writeln!(s, "\t\t# Policy rules, most specific first; each is final");
    for rule in policy.rules() {
        let m = format!("{} {}", daddr(rule.prefix.family()), rule.prefix);
        match rule.action {
            Action::Direct => {
                let _ = writeln!(s, "\t\t{m} accept");
            }
            Action::Block => {
                let _ = writeln!(s, "\t\t{m} drop");
            }
            Action::Tunnel => {
                let _ = writeln!(s, "\t\t{m} oifname \"{tun}\" accept");
                let _ = writeln!(s, "\t\t{m} drop");
            }
        }
    }
    match policy.mode() {
        Mode::FullTunnel => {
            let _ = writeln!(
                s,
                "\t\t# Default (full tunnel): only through the tunnel; otherwise dropped"
            );
            let _ = writeln!(s, "\t\toifname \"{tun}\" accept");
        }
        Mode::Split => {
            let _ = writeln!(
                s,
                "\t\t# Default (split): not protected by Septuvo, sent normally"
            );
            let _ = writeln!(s, "\t\taccept");
        }
    }
    let _ = writeln!(s, "\t}}");
    let _ = writeln!(s, "}}");
    Ok(s)
}

/// Checks every input that is written into the ruleset.
fn validate(policy: &Policy, ks: &KillSwitch) -> Result<(), RenderError> {
    if !valid_interface(&ks.tunnel_interface) {
        return Err(RenderError::BadInterfaceName);
    }
    if ks.endpoints.len() > MAX_ENDPOINTS {
        return Err(RenderError::TooManyEndpoints);
    }
    if ks.endpoints.iter().any(|&(_, port)| port == 0) {
        return Err(RenderError::BadEndpointPort);
    }
    if ks.captive_helper == Some(0) {
        return Err(RenderError::BadCaptiveHelper);
    }
    match &ks.dns {
        DnsRules::Resolvers {
            resolvers,
            lan_exceptions,
        } if resolvers.is_empty() || resolvers.len() + lan_exceptions.len() > MAX_DNS_ADDRESSES => {
            return Err(RenderError::BadDnsAddresses);
        }
        DnsRules::System if policy.mode() == Mode::FullTunnel => {
            return Err(RenderError::SystemDnsInFullTunnel);
        }
        _ => {}
    }
    Ok(())
}

fn family_of(addr: IpAddr) -> Family {
    match addr {
        IpAddr::V4(_) => Family::V4,
        IpAddr::V6(_) => Family::V6,
    }
}

/// The captive-portal timeout sets, empty until [`captive_script`] fills them.
fn render_captive_sets(s: &mut String) {
    let _ = writeln!(
        s,
        "\t# Captive-portal mode (Q1): empty unless opened; the kernel removes elements on timeout"
    );
    let _ = writeln!(
        s,
        "\tset {CAPTIVE_WEB_SET} {{ type inet_service; flags timeout; }}"
    );
    let _ = writeln!(
        s,
        "\tset {CAPTIVE_DNS4_SET} {{ type ipv4_addr; flags timeout; }}"
    );
    let _ = writeln!(
        s,
        "\tset {CAPTIVE_DNS6_SET} {{ type ipv6_addr; flags timeout; }}"
    );
}

/// The helper's rules, right after loopback and before every other rule, so
/// they are the **only** rules that apply to the helper: it is isolated from
/// the tunnel and from everything the policy names, may use DNS only to the
/// (normally empty) network-resolver sets and web ports only while the web
/// set is filled, and gets nothing else. No other program is affected.
fn render_captive_helper(s: &mut String, policy: &Policy, tun: &str, uid: u32) {
    let me = format!("meta skuid {uid}");
    let _ = writeln!(
        s,
        "\t\t# Captive-portal helper (uid {uid}): the only account the window applies to"
    );
    let _ = writeln!(s, "\t\t{me} oifname \"{tun}\" drop");
    let _ = writeln!(
        s,
        "\t\t{me} ip daddr @{CAPTIVE_DNS4_SET} meta l4proto {{ tcp, udp }} th dport 53 accept"
    );
    let _ = writeln!(
        s,
        "\t\t{me} ip6 daddr @{CAPTIVE_DNS6_SET} meta l4proto {{ tcp, udp }} th dport 53 accept"
    );
    for rule in policy.rules() {
        let _ = writeln!(
            s,
            "\t\t{me} {} {} drop",
            daddr(rule.prefix.family()),
            rule.prefix
        );
    }
    let _ = writeln!(s, "\t\t{me} tcp dport @{CAPTIVE_WEB_SET} accept");
    let _ = writeln!(s, "\t\t{me} drop");
}

/// DNS rules: after the underlay exceptions, before the policy rules, so no
/// policy rule (not even `direct`) can let DNS out another way.
fn render_dns(s: &mut String, policy: &Policy, tun: &str, dns: &DnsRules) {
    match dns {
        DnsRules::TunnelOnly => {
            if policy.mode() == Mode::FullTunnel {
                let _ = writeln!(s, "\t\t# NP-D1 DNS only through the tunnel");
                let _ = writeln!(s, "\t\toifname != \"{tun}\" udp dport 53 drop");
                let _ = writeln!(s, "\t\toifname != \"{tun}\" tcp dport {{ 53, 853 }} drop");
            }
        }
        DnsRules::Resolvers {
            resolvers,
            lan_exceptions,
        } => {
            let _ = writeln!(
                s,
                "\t\t# NP-D1 DNS only to the configured resolvers, through the tunnel"
            );
            for &r in resolvers {
                let d = daddr(family_of(r));
                let _ = writeln!(
                    s,
                    "\t\t{d} {r} oifname \"{tun}\" udp dport {{ 53, 853 }} accept"
                );
                let _ = writeln!(
                    s,
                    "\t\t{d} {r} oifname \"{tun}\" tcp dport {{ 53, 853 }} accept"
                );
            }
            if !lan_exceptions.is_empty() {
                let _ = writeln!(s, "\t\t# Explicit LAN DNS exceptions (direct)");
            }
            for &l in lan_exceptions {
                let d = daddr(family_of(l));
                let _ = writeln!(s, "\t\t{d} {l} oifname != \"{tun}\" udp dport 53 accept");
                let _ = writeln!(s, "\t\t{d} {l} oifname != \"{tun}\" tcp dport 53 accept");
            }
            let _ = writeln!(
                s,
                "\t\t# All other DNS, DNS over TLS and DNS over QUIC is dropped"
            );
            let _ = writeln!(s, "\t\tudp dport {{ 53, 853 }} drop");
            let _ = writeln!(s, "\t\ttcp dport {{ 53, 853 }} drop");
        }
        DnsRules::System => {
            let _ = writeln!(s, "\t\t# DNS: system resolver (split mode, not protected)");
        }
    }
}

/// Script that opens captive-portal mode for `seconds` (Q1, N3-02): web
/// traffic and DNS to `resolvers` may leave outside the tunnel until the
/// kernel removes the elements.
///
/// The sets are flushed first in the same transaction, so asking again
/// restarts the window from now; it never adds up. Requires the kill-switch
/// table from [`render`].
///
/// # Errors
/// [`RenderError::BadCaptiveDuration`] or [`RenderError::BadCaptiveResolvers`].
pub fn captive_script(seconds: u32, resolvers: &[IpAddr]) -> Result<String, RenderError> {
    if !(1..=MAX_CAPTIVE_SECONDS).contains(&seconds) {
        return Err(RenderError::BadCaptiveDuration);
    }
    if resolvers.len() > MAX_CAPTIVE_RESOLVERS
        || resolvers
            .iter()
            .any(|r| r.is_unspecified() || r.is_multicast())
    {
        return Err(RenderError::BadCaptiveResolvers);
    }
    let mut s = String::from("# Septuvo captive-portal window (kernel-enforced timeout)\n");
    for set in [CAPTIVE_WEB_SET, CAPTIVE_DNS4_SET, CAPTIVE_DNS6_SET] {
        let _ = writeln!(s, "flush set {TABLE} {set}");
    }
    let _ = writeln!(
        s,
        "add element {TABLE} {CAPTIVE_WEB_SET} {{ 80 timeout {seconds}s, 443 timeout {seconds}s }}"
    );
    for (set, family) in [
        (CAPTIVE_DNS4_SET, Family::V4),
        (CAPTIVE_DNS6_SET, Family::V6),
    ] {
        let list: Vec<String> = resolvers
            .iter()
            .filter(|r| family_of(**r) == family)
            .map(|r| format!("{r} timeout {seconds}s"))
            .collect();
        if !list.is_empty() {
            let _ = writeln!(s, "add element {TABLE} {set} {{ {} }}", list.join(", "));
        }
    }
    Ok(s)
}

/// Script that removes every Septuvo rule (explicit disconnect, uninstall).
#[must_use]
pub fn removal_script() -> String {
    format!("add table {TABLE}\ndelete table {TABLE}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_policy;

    fn ks() -> KillSwitch {
        KillSwitch {
            tunnel_interface: "septuvo0".into(),
            endpoints: vec![("203.0.113.7".parse().unwrap(), 51820)],
            dns: DnsRules::TunnelOnly,
            captive_helper: None,
        }
    }

    fn with_resolvers(resolvers: &[&str], lan: &[&str]) -> KillSwitch {
        KillSwitch {
            dns: DnsRules::Resolvers {
                resolvers: resolvers.iter().map(|a| a.parse().unwrap()).collect(),
                lan_exceptions: lan.iter().map(|a| a.parse().unwrap()).collect(),
            },
            ..ks()
        }
    }

    #[test]
    fn resolver_dns_allows_only_configured_resolvers_through_the_tunnel() {
        let p = parse_policy("mode full-tunnel\ndirect 192.168.1.53/32\n").unwrap();
        let s = render(
            &p,
            &with_resolvers(&["10.77.0.1", "fd77::1"], &["192.168.1.53"]),
        )
        .unwrap();
        assert!(s.contains("ip daddr 10.77.0.1 oifname \"septuvo0\" udp dport { 53, 853 } accept"));
        assert!(s.contains("ip6 daddr fd77::1 oifname \"septuvo0\" tcp dport { 53, 853 } accept"));
        assert!(s.contains("ip daddr 192.168.1.53 oifname != \"septuvo0\" udp dport 53 accept"));
        // Every other DNS packet is dropped, and before any policy rule, so
        // the direct rule for 192.168.1.53 cannot carry DNS except as listed.
        let drop_udp = position(&s, "udp dport { 53, 853 } drop");
        let drop_tcp = position(&s, "tcp dport { 53, 853 } drop");
        let first_policy = position(&s, "ip daddr 192.168.1.53/32 accept");
        assert!(drop_udp < first_policy && drop_tcp < first_policy);
        assert!(
            position(
                &s,
                "ip daddr 10.77.0.1 oifname \"septuvo0\" udp dport { 53, 853 } accept"
            ) < drop_udp
        );
        assert!(
            !s.contains("oifname != \"septuvo0\" udp dport 53 drop"),
            "N1 rule replaced"
        );
    }

    #[test]
    fn resolver_dns_also_locks_down_split_mode() {
        let p = parse_policy("mode split\ntunnel 192.168.50.0/24\n").unwrap();
        let s = render(&p, &with_resolvers(&["192.168.50.1"], &[])).unwrap();
        assert!(
            s.contains("ip daddr 192.168.50.1 oifname \"septuvo0\" udp dport { 53, 853 } accept")
        );
        assert!(s.contains("\t\tudp dport { 53, 853 } drop"));
        assert!(!s.contains("LAN DNS exceptions"));
    }

    #[test]
    fn dns_rule_limits_and_leaks_are_refused() {
        let full = parse_policy("mode full-tunnel\n").unwrap();
        let split = parse_policy("mode split\ntunnel 10.0.0.0/8\n").unwrap();
        assert_eq!(
            render(&full, &with_resolvers(&[], &[])),
            Err(RenderError::BadDnsAddresses)
        );
        let many: Vec<String> = (1..=17).map(|i| format!("10.77.0.{i}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        assert_eq!(
            render(&full, &with_resolvers(&many, &[])),
            Err(RenderError::BadDnsAddresses)
        );
        let system = KillSwitch {
            dns: DnsRules::System,
            ..ks()
        };
        assert_eq!(
            render(&full, &system),
            Err(RenderError::SystemDnsInFullTunnel)
        );
        let s = render(&split, &system).unwrap();
        assert!(
            !dns_rules_outside_captive(&s),
            "system DNS adds no DNS rules"
        );
    }

    /// True if a DNS rule other than the (normally empty) captive-portal
    /// rules is present.
    fn dns_rules_outside_captive(script: &str) -> bool {
        lines(script)
            .iter()
            .any(|l| l.contains("dport 53") && !l.contains("@captive"))
    }

    fn lines(script: &str) -> Vec<&str> {
        script.lines().map(str::trim).collect()
    }

    fn position(script: &str, needle: &str) -> usize {
        lines(script)
            .iter()
            .position(|l| *l == needle)
            .unwrap_or_else(|| panic!("missing line {needle:?} in\n{script}"))
    }

    #[test]
    fn full_tunnel_drops_by_default_and_allows_only_the_tunnel() {
        let p = parse_policy("mode full-tunnel\n").unwrap();
        let s = render(&p, &ks()).unwrap();
        assert!(s.contains("policy drop;"));
        let default = position(&s, "oifname \"septuvo0\" accept");
        assert_eq!(lines(&s)[default + 1], "}", "default must be the last rule");
        assert!(!lines(&s).contains(&"accept"), "no blanket accept");
    }

    #[test]
    fn underlay_exceptions_come_first_and_are_minimal() {
        let p = parse_policy("mode full-tunnel\nblock 203.0.113.0/24\n").unwrap();
        let s = render(&p, &ks()).unwrap();
        let endpoint = position(&s, "ip daddr 203.0.113.7 udp dport 51820 accept");
        let block = position(&s, "ip daddr 203.0.113.0/24 drop");
        assert!(endpoint < block, "endpoint exception precedes policy rules");
        assert!(s.contains("ip6 hoplimit 255 accept"));
        let accepts = lines(&s).iter().filter(|l| l.ends_with("accept")).count();
        // lo, DHCPv4, DHCPv6, ND, endpoint, default
        assert_eq!(accepts, 6);
    }

    #[test]
    fn dns_outside_the_tunnel_is_dropped_only_in_full_tunnel() {
        let full = render(&parse_policy("mode full-tunnel\n").unwrap(), &ks()).unwrap();
        assert!(full.contains("oifname != \"septuvo0\" udp dport 53 drop"));
        assert!(full.contains("oifname != \"septuvo0\" tcp dport { 53, 853 } drop"));
        let split = render(
            &parse_policy("mode split\ntunnel 192.168.50.0/24\n").unwrap(),
            &ks(),
        )
        .unwrap();
        assert!(!dns_rules_outside_captive(&split));
    }

    #[test]
    fn rules_are_most_specific_first_and_tunnel_rules_are_final() {
        let p = parse_policy(
            "mode split\ndirect 192.168.1.0/24\ntunnel 192.168.1.128/25\ntunnel 2001:db8:50::/64\n",
        )
        .unwrap();
        let s = render(&p, &ks()).unwrap();
        let tunnel = position(&s, "ip daddr 192.168.1.128/25 oifname \"septuvo0\" accept");
        let tunnel_drop = position(&s, "ip daddr 192.168.1.128/25 drop");
        let direct = position(&s, "ip daddr 192.168.1.0/24 accept");
        assert!(tunnel < tunnel_drop && tunnel_drop < direct);
        assert!(s.contains("ip6 daddr 2001:db8:50::/64 oifname \"septuvo0\" accept"));
        let last_rule = lines(&s).len() - 3;
        assert_eq!(
            lines(&s)[last_rule],
            "accept",
            "split default is the last rule"
        );
    }

    fn with_helper(policy: &str) -> String {
        let p = parse_policy(policy).unwrap();
        let k = KillSwitch {
            captive_helper: Some(995),
            ..with_resolvers(&["10.77.0.1"], &["192.168.1.53"])
        };
        render(&p, &k).unwrap()
    }

    #[test]
    fn without_a_helper_there_is_no_captive_rule_or_set() {
        let p = parse_policy("mode full-tunnel\n").unwrap();
        let s = render(&p, &with_resolvers(&["10.77.0.1"], &[])).unwrap();
        assert!(!s.contains("captive"), "captive-portal mode not configured");
        assert!(!s.contains("skuid"));
    }

    /// The core property of the revised design: every captive rule matches
    /// the helper account only, so for every other program the ruleset is
    /// exactly the kill switch without captive-portal mode.
    #[test]
    fn captive_rules_apply_to_the_helper_account_only() {
        let policy =
            "mode full-tunnel\ndirect 192.168.1.30/32\nblock 203.0.113.99/32\ntunnel 10.0.0.0/8\n";
        let with = with_helper(policy);
        let without = render(
            &parse_policy(policy).unwrap(),
            &with_resolvers(&["10.77.0.1"], &["192.168.1.53"]),
        )
        .unwrap();
        let others: Vec<&str> = lines(&with)
            .into_iter()
            .filter(|l| !l.contains("skuid 995") && !l.to_lowercase().contains("captive"))
            .collect();
        assert_eq!(
            others,
            lines(&without),
            "rules for ordinary applications are unchanged"
        );
        for l in lines(&with).iter().filter(|l| l.contains('@')) {
            assert!(
                l.starts_with("meta skuid 995 "),
                "unscoped captive rule: {l}"
            );
        }
    }

    #[test]
    fn the_helper_is_isolated_and_its_rules_come_first() {
        let s = with_helper(
            "mode full-tunnel\ndirect 192.168.1.30/32\nblock 203.0.113.99/32\ntunnel 10.0.0.0/8\n",
        );
        let me = "meta skuid 995";
        let lo = position(&s, "oifname \"lo\" accept");
        let tunnel = position(&s, &format!("{me} oifname \"septuvo0\" drop"));
        let dns4 = position(
            &s,
            &format!("{me} ip daddr @captive_dns4 meta l4proto {{ tcp, udp }} th dport 53 accept"),
        );
        let direct = position(&s, &format!("{me} ip daddr 192.168.1.30/32 drop"));
        let block = position(&s, &format!("{me} ip daddr 203.0.113.99/32 drop"));
        let tun_prefix = position(&s, &format!("{me} ip daddr 10.0.0.0/8 drop"));
        let web = position(&s, &format!("{me} tcp dport @captive_web accept"));
        let last = position(&s, &format!("{me} drop"));
        let endpoint = position(&s, "ip daddr 203.0.113.7 udp dport 51820 accept");
        assert!(lo < tunnel, "loopback first");
        assert!(
            tunnel < dns4,
            "never through the tunnel, before anything is accepted"
        );
        assert!(
            dns4 < direct && direct < web && block < web && tun_prefix < web,
            "nothing the policy names, not even direct hosts"
        );
        assert!(
            web < last && last < endpoint,
            "a final drop: no later rule (endpoints, LAN DNS, direct, default) applies to the helper"
        );
        assert!(
            !lines(&s)
                .iter()
                .any(|l| l.contains("@captive") && l.contains("853"))
        );
        assert!(
            !lines(&s)
                .iter()
                .any(|l| l.contains(me) && l.contains("udp dport @captive_web")),
            "web ports are TCP only (no QUIC)"
        );
        assert!(s.contains("set captive_web { type inet_service; flags timeout; }"));
        assert!(!s.contains("elements ="), "nothing is open by default");
    }

    #[test]
    fn root_can_never_be_the_helper() {
        let p = parse_policy("mode full-tunnel\n").unwrap();
        let k = KillSwitch {
            captive_helper: Some(0),
            ..ks()
        };
        assert_eq!(render(&p, &k), Err(RenderError::BadCaptiveHelper));
    }

    #[test]
    fn captive_script_restarts_the_window_and_is_bounded() {
        let s = captive_script(
            300,
            &["192.168.1.1".parse().unwrap(), "fe80::1".parse().unwrap()],
        )
        .unwrap();
        let l = lines(&s);
        assert_eq!(l[1], "flush set inet septuvo captive_web");
        assert_eq!(l[2], "flush set inet septuvo captive_dns4");
        assert_eq!(l[3], "flush set inet septuvo captive_dns6");
        assert_eq!(
            l[4],
            "add element inet septuvo captive_web { 80 timeout 300s, 443 timeout 300s }"
        );
        assert_eq!(
            l[5],
            "add element inet septuvo captive_dns4 { 192.168.1.1 timeout 300s }"
        );
        assert_eq!(
            l[6],
            "add element inet septuvo captive_dns6 { fe80::1 timeout 300s }"
        );
        let web_only = captive_script(60, &[]).unwrap();
        assert!(
            !web_only.contains("captive_dns4 {"),
            "no empty element lists"
        );
        assert_eq!(captive_script(0, &[]), Err(RenderError::BadCaptiveDuration));
        assert_eq!(
            captive_script(MAX_CAPTIVE_SECONDS + 1, &[]),
            Err(RenderError::BadCaptiveDuration)
        );
        let five: Vec<IpAddr> = (1..=5)
            .map(|i| format!("192.168.1.{i}").parse().unwrap())
            .collect();
        assert_eq!(
            captive_script(60, &five),
            Err(RenderError::BadCaptiveResolvers)
        );
        for bad in ["0.0.0.0", "::", "224.0.0.251", "ff02::fb"] {
            assert_eq!(
                captive_script(60, &[bad.parse().unwrap()]),
                Err(RenderError::BadCaptiveResolvers),
                "{bad}"
            );
        }
    }

    #[test]
    fn table_constants_agree() {
        assert_eq!(TABLE, format!("{TABLE_FAMILY} {TABLE_NAME}"));
    }

    /// Every value written into the script is either a validated interface
    /// name, a typed IP address or prefix, or a number: no free text, so no
    /// quote, semicolon, brace, comment or line break can reach nft.
    #[test]
    fn hostile_interface_names_are_rejected() {
        let p = parse_policy("mode full-tunnel\n").unwrap();
        for bad in [
            "",
            "a234567890123456",
            "tun0\"; flush ruleset; \"",
            "septuvo0\" accept; #",
            "tun0}",
            "tun0{",
            "tun0#",
            "tun0;",
            "tun0'",
            "tun0\\",
            "tun0\r",
            "tun0\u{0}",
            "tun 0",
            "tun0\n",
            "-tun",
            ".",
            "..",
            "tün0",
        ] {
            let mut k = ks();
            k.tunnel_interface = bad.into();
            assert_eq!(
                render(&p, &k),
                Err(RenderError::BadInterfaceName),
                "{bad:?}"
            );
        }
        let mut k = ks();
        k.tunnel_interface = "wg_test-1.2".into();
        assert!(render(&p, &k).is_ok());
    }

    #[test]
    fn endpoint_limits() {
        let p = parse_policy("mode full-tunnel\n").unwrap();
        let mut k = ks();
        k.endpoints = vec![("203.0.113.7".parse().unwrap(), 0)];
        assert_eq!(render(&p, &k), Err(RenderError::BadEndpointPort));
        k.endpoints = vec![("203.0.113.7".parse().unwrap(), 1); MAX_ENDPOINTS + 1];
        assert_eq!(render(&p, &k), Err(RenderError::TooManyEndpoints));
        k.endpoints = vec![("2001:db8:7::7".parse().unwrap(), 443)];
        assert!(
            render(&p, &k)
                .unwrap()
                .contains("ip6 daddr 2001:db8:7::7 udp dport 443 accept")
        );
    }

    #[test]
    fn output_is_deterministic_and_replaces_atomically() {
        let a =
            parse_policy("mode full-tunnel\ndirect 192.168.50.10/32\nblock 10.0.0.0/8\n").unwrap();
        let b =
            parse_policy("mode full-tunnel\nblock 10.0.0.0/8\ndirect 192.168.50.10/32\n").unwrap();
        let (sa, sb) = (render(&a, &ks()).unwrap(), render(&b, &ks()).unwrap());
        assert_eq!(sa, sb);
        let l = lines(&sa);
        assert_eq!(l[1], "add table inet septuvo");
        assert_eq!(l[2], "delete table inet septuvo");
        assert_eq!(
            removal_script(),
            "add table inet septuvo\ndelete table inet septuvo\n"
        );
    }
}
