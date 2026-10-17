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
//! Scope: locally generated traffic (the output hook). Forwarded traffic,
//! captive-portal mode (Q1) and routes are not rendered here.

use crate::policy::{Action, Mode, Policy};
use crate::prefix::Family;
use std::fmt::{self, Write};
use std::net::IpAddr;

/// nftables table that holds every Septuvo rule.
pub const TABLE: &str = "inet septuvo";
/// Maximum number of server endpoints in one ruleset.
pub const MAX_ENDPOINTS: usize = 16;
/// Maximum number of resolver plus LAN DNS exception addresses in one ruleset.
pub const MAX_DNS_ADDRESSES: usize = 16;

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
    if !valid_interface(&ks.tunnel_interface) {
        return Err(RenderError::BadInterfaceName);
    }
    if ks.endpoints.len() > MAX_ENDPOINTS {
        return Err(RenderError::TooManyEndpoints);
    }
    if ks.endpoints.iter().any(|&(_, port)| port == 0) {
        return Err(RenderError::BadEndpointPort);
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
    let _ = writeln!(s, "\tchain output {{");
    let _ = writeln!(
        s,
        "\t\ttype filter hook output priority filter; policy drop;"
    );
    let _ = writeln!(s, "\t\t# NP-U4 loopback");
    let _ = writeln!(s, "\t\toifname \"lo\" accept");
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

fn family_of(addr: IpAddr) -> Family {
    match addr {
        IpAddr::V4(_) => Family::V4,
        IpAddr::V6(_) => Family::V6,
    }
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
        assert!(!s.contains("dport 53"), "system DNS adds no DNS rules");
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
        assert!(!split.contains("dport 53"));
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

    #[test]
    fn hostile_interface_names_are_rejected() {
        let p = parse_policy("mode full-tunnel\n").unwrap();
        for bad in [
            "",
            "a234567890123456",
            "tun0\"; flush ruleset; \"",
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
