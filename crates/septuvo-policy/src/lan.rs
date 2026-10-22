//! Local Network Access (D-040, network-policies NP-L1 to NP-L7): which
//! directly attached subnets may be opened when the user sets it to
//! **Allowed**.
//!
//! The subnets are computed from the interfaces' **own addresses and prefix
//! lengths** (NP-L2), never from a list of "private" ranges (NP-E7). Those
//! prefix lengths come from DHCP or router advertisements, which anyone on
//! the network can fake (NP-6.2), so:
//!
//! - **size caps** (NP-L3): IPv4 prefixes shorter than /16 and IPv6
//!   prefixes shorter than /64 are never opened (a fake `/1` cannot open
//!   half the internet); a /32 or /128 opens nothing;
//! - **the tunnel wins** (NP-L4): a subnet that overlaps a tunnel or
//!   blocked prefix, or contains a server endpoint, is not opened;
//! - loopback, the tunnel interface, multicast and unspecified addresses are
//!   ignored; at most [`MAX_LOCAL_NETWORKS`] subnets are opened;
//! - IPv6 link-local (`fe80::/10`) is opened only if an underlay interface
//!   has a link-local address (it is never routed).
//!
//! Discovery (mDNS, LLMNR, SSDP, broadcast) stays blocked by the renderer
//! (NP-L5). The caller recomputes on every address change (NP-L6). Pure
//! computation: no system calls.

use crate::prefix::{Family, Prefix};
use std::net::IpAddr;

/// Shortest IPv4 prefix that is opened (NP-L3; confirmed by the
/// independent review).
pub const MIN_IPV4_PREFIX: u8 = 16;
/// Shortest IPv6 prefix that is opened (NP-L3).
pub const MIN_IPV6_PREFIX: u8 = 64;
/// Most subnets opened at once.
pub const MAX_LOCAL_NETWORKS: usize = 8;

/// One address on one interface, as the system reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceAddress {
    /// Interface name.
    pub interface: String,
    /// The address.
    pub addr: IpAddr,
    /// Its prefix length.
    pub prefix_len: u8,
}

/// What Local Network Access opens.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalAccess {
    /// Directly attached subnets, after the caps and the overlap rules.
    pub subnets: Vec<Prefix>,
    /// Open IPv6 link-local (`fe80::/10`).
    pub ipv6_link_local: bool,
}

impl LocalAccess {
    /// True if nothing is opened.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subnets.is_empty() && !self.ipv6_link_local
    }
}

/// Why an attached subnet was not opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Shorter than the size cap (NP-L3).
    TooLarge,
    /// Overlaps a tunnel or blocked prefix (NP-L4).
    OverlapsPolicy,
    /// Contains a server endpoint (NP-L4).
    ContainsEndpoint,
    /// More than [`MAX_LOCAL_NETWORKS`].
    TooMany,
}

/// The computed result, with what was refused (for status and the lab).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Computed {
    /// What is opened.
    pub access: LocalAccess,
    /// Subnets that were attached but not opened, and why.
    pub refused: Vec<(Prefix, Refusal)>,
}

fn ignored(a: &InterfaceAddress, tunnel_interface: &str) -> bool {
    a.interface == "lo"
        || a.interface == tunnel_interface
        || a.addr.is_loopback()
        || a.addr.is_unspecified()
        || a.addr.is_multicast()
}

fn link_local_v6(addr: IpAddr) -> bool {
    matches!(addr, IpAddr::V6(a) if (a.segments()[0] & 0xffc0) == 0xfe80)
}

/// Computes the subnets Local Network Access may open.
///
/// `exclude`: the tunnel and blocked prefixes of the policy (NP-L4);
/// `endpoints`: the server endpoints.
#[must_use]
pub fn attached_subnets(
    addresses: &[InterfaceAddress],
    tunnel_interface: &str,
    exclude: &[Prefix],
    endpoints: &[IpAddr],
) -> Computed {
    let mut out = Computed::default();
    let mut candidates: Vec<Prefix> = Vec::new();
    for a in addresses.iter().filter(|a| !ignored(a, tunnel_interface)) {
        if link_local_v6(a.addr) {
            out.access.ipv6_link_local = true;
            continue;
        }
        let (min, width) = match Family::of(a.addr) {
            Family::V4 => (MIN_IPV4_PREFIX, 32),
            Family::V6 => (MIN_IPV6_PREFIX, 128),
        };
        if a.prefix_len >= width {
            continue; // a single address: nothing to open
        }
        let Ok(subnet) = Prefix::containing(a.addr, a.prefix_len) else {
            continue;
        };
        if candidates.contains(&subnet) || out.refused.iter().any(|(p, _)| *p == subnet) {
            continue;
        }
        let refusal = if a.prefix_len < min {
            Some(Refusal::TooLarge)
        } else if exclude.iter().any(|p| p.overlaps(&subnet)) {
            Some(Refusal::OverlapsPolicy)
        } else if endpoints.iter().any(|e| subnet.contains(*e)) {
            Some(Refusal::ContainsEndpoint)
        } else {
            None
        };
        match refusal {
            Some(r) => out.refused.push((subnet, r)),
            None => candidates.push(subnet),
        }
    }
    // Deterministic: the most specific first, then by address.
    candidates.sort_by(|a, b| {
        b.prefix_len()
            .cmp(&a.prefix_len())
            .then_with(|| a.addr().cmp(&b.addr()))
    });
    for (i, subnet) in candidates.into_iter().enumerate() {
        if i < MAX_LOCAL_NETWORKS {
            out.access.subnets.push(subnet);
        } else {
            out.refused.push((subnet, Refusal::TooMany));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(interface: &str, text: &str) -> InterfaceAddress {
        let (addr, len) = text.split_once('/').expect("address/len");
        InterfaceAddress {
            interface: interface.into(),
            addr: addr.parse().expect("address"),
            prefix_len: len.parse().expect("length"),
        }
    }

    fn p(text: &str) -> Prefix {
        text.parse().expect("prefix")
    }

    #[test]
    fn opens_only_the_directly_attached_subnets() {
        let c = attached_subnets(
            &[
                a("lo", "127.0.0.1/8"),
                a("eth0", "192.168.1.30/24"),
                a("eth0", "2001:db8:1::30/64"),
                a("eth0", "fe80::1/64"),
                a("septuvo0", "10.77.0.2/24"),
            ],
            "septuvo0",
            &[],
            &[],
        );
        assert_eq!(
            c.access.subnets,
            [p("2001:db8:1::/64"), p("192.168.1.0/24")]
        );
        assert!(c.access.ipv6_link_local);
        assert_eq!(c.refused, vec![]);
    }

    /// NP-L3: hostile DHCP or router advertisements announcing huge
    /// prefixes open nothing.
    #[test]
    fn hostile_prefix_lengths_are_capped() {
        let c = attached_subnets(
            &[
                a("eth0", "10.1.2.3/8"),
                a("eth0", "100.64.1.1/10"),
                a("eth0", "1.2.3.4/1"),
                a("eth0", "2001:db8::1/32"),
                a("eth0", "2001:db8::1/0"),
                a("eth0", "172.16.5.5/16"),
                a("eth0", "192.0.2.7/32"),
                a("eth0", "2001:db8:9::1/128"),
            ],
            "septuvo0",
            &[],
            &[],
        );
        assert_eq!(c.access.subnets, [p("172.16.0.0/16")]);
        assert_eq!(
            c.refused
                .iter()
                .filter(|(_, r)| *r == Refusal::TooLarge)
                .count(),
            5
        );
    }

    /// NP-L4: the tunnel wins over the local network.
    #[test]
    fn overlaps_with_the_tunnel_or_an_endpoint_are_refused() {
        let c = attached_subnets(
            &[
                a("eth0", "192.168.50.20/24"),
                a("eth1", "192.168.7.9/24"),
                a("eth2", "10.200.0.10/24"),
            ],
            "septuvo0",
            &[p("192.168.50.0/24")],
            &["10.200.0.1".parse().unwrap()],
        );
        assert_eq!(c.access.subnets, [p("192.168.7.0/24")]);
        assert!(
            c.refused
                .contains(&(p("192.168.50.0/24"), Refusal::OverlapsPolicy))
        );
        assert!(
            c.refused
                .contains(&(p("10.200.0.0/24"), Refusal::ContainsEndpoint))
        );
        // A wider tunnel prefix covering the subnet also wins.
        let c = attached_subnets(
            &[a("eth0", "192.168.7.9/24")],
            "septuvo0",
            &[p("192.168.0.0/16")],
            &[],
        );
        assert_eq!(c.access.subnets, vec![]);
        // So does a narrower one inside it.
        let c = attached_subnets(
            &[a("eth0", "192.168.7.9/24")],
            "septuvo0",
            &[p("192.168.7.128/25")],
            &[],
        );
        assert_eq!(c.access.subnets, vec![]);
    }

    #[test]
    fn at_most_eight_subnets_and_no_duplicates() {
        let mut addrs: Vec<InterfaceAddress> = (0..12)
            .map(|i| a("eth0", &format!("192.168.{i}.1/24")))
            .collect();
        addrs.push(a("eth1", "192.168.0.99/24"));
        let c = attached_subnets(&addrs, "septuvo0", &[], &[]);
        assert_eq!(c.access.subnets.len(), MAX_LOCAL_NETWORKS);
        assert_eq!(
            c.refused
                .iter()
                .filter(|(_, r)| *r == Refusal::TooMany)
                .count(),
            4
        );
        // Same input, same output.
        assert_eq!(attached_subnets(&addrs, "septuvo0", &[], &[]), c);
    }

    #[test]
    fn multicast_unspecified_and_loopback_are_ignored() {
        let c = attached_subnets(
            &[
                a("eth0", "224.0.0.1/24"),
                a("eth0", "0.0.0.0/16"),
                a("eth0", "::1/64"),
                a("eth0", "ff02::1/64"),
            ],
            "septuvo0",
            &[],
            &[],
        );
        assert_eq!(c, Computed::default());
    }
}
