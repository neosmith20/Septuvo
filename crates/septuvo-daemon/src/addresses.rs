//! The device's own interface addresses, for Local Network Access (N4-04,
//! NP-L2, NP-L6).
//!
//! Read with the `ip` program (iproute2) at a fixed path, never `PATH`,
//! with fixed arguments and no shell, like `nft`. The service runs inside
//! the lab namespace, so these are that namespace's interfaces. Changes are
//! followed with `ip -o monitor address` ([`watch`]); every change makes the
//! service recompute the opened subnets at once (NP-L6).

use septuvo_policy::lan::InterfaceAddress;
use std::io::{BufRead, BufReader};
use std::net::IpAddr;
use std::path::Path;
use std::process::{Command, Stdio};

/// Most addresses read (more are ignored: nothing extra is opened).
pub const MAX_ADDRESSES: usize = 256;

/// The source of interface addresses.
pub trait Addresses: Send {
    /// The current addresses of every interface.
    fn current(&mut self) -> Vec<InterfaceAddress>;
}

/// No addresses (tests, and Local Network Access before it is set up).
#[derive(Debug, Default)]
pub struct NoAddresses;

impl Addresses for NoAddresses {
    fn current(&mut self) -> Vec<InterfaceAddress> {
        Vec::new()
    }
}

const IP_PATHS: [&str; 4] = ["/usr/sbin/ip", "/sbin/ip", "/usr/bin/ip", "/bin/ip"];

fn ip_binary() -> Option<&'static str> {
    IP_PATHS.into_iter().find(|p| Path::new(p).is_file())
}

/// The real addresses, from `ip -o addr show`.
#[derive(Debug, Default)]
pub struct IpCommand;

impl Addresses for IpCommand {
    fn current(&mut self) -> Vec<InterfaceAddress> {
        let Some(ip) = ip_binary() else {
            return Vec::new();
        };
        Command::new(ip)
            .args(["-o", "addr", "show"])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| parse(&String::from_utf8_lossy(&o.stdout)))
            .unwrap_or_default()
    }
}

/// Parses `ip -o addr show` output: one address per line,
/// `INDEX: NAME FAMILY ADDRESS/LEN ...`. Anything that does not have exactly
/// that shape is skipped (this is text from another program: never trusted
/// to be well formed).
#[must_use]
pub fn parse(text: &str) -> Vec<InterfaceAddress> {
    let mut out = Vec::new();
    for line in text.lines() {
        if out.len() == MAX_ADDRESSES {
            break;
        }
        let mut words = line.split_whitespace();
        let (Some(index), Some(name), Some(family), Some(cidr)) =
            (words.next(), words.next(), words.next(), words.next())
        else {
            continue;
        };
        if !index.ends_with(':') || !matches!(family, "inet" | "inet6") {
            continue;
        }
        // `eth0@if5` names the peer; the interface is `eth0`.
        let name = name.split('@').next().unwrap_or(name);
        if name.is_empty() || name.len() > 15 {
            continue;
        }
        let Some((addr, len)) = cidr.split_once('/') else {
            continue;
        };
        let (Ok(addr), Ok(prefix_len)) = (addr.parse::<IpAddr>(), len.parse::<u8>()) else {
            continue;
        };
        let max = if addr.is_ipv4() { 32 } else { 128 };
        if prefix_len > max || (family == "inet") != addr.is_ipv4() {
            continue;
        }
        out.push(InterfaceAddress {
            interface: name.to_string(),
            addr,
            prefix_len,
        });
    }
    out
}

/// Calls `on_change` whenever an address is added or removed, until `ip`
/// stops (then returns). Runs `ip -o monitor address`.
pub fn watch(mut on_change: impl FnMut()) {
    let Some(ip) = ip_binary() else {
        return;
    };
    let Ok(mut child) = Command::new(ip)
        .args(["-o", "monitor", "address"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return;
    };
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines() {
            if line.is_err() {
                break;
            }
            on_change();
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ip_output() {
        let text = "1: lo    inet 127.0.0.1/8 scope host lo\\       valid_lft forever preferred_lft forever\n\
                    2: eth0@if7    inet 10.200.0.10/24 brd 10.200.0.255 scope global eth0\\       valid_lft forever\n\
                    2: eth0@if7    inet6 2001:db8:1::10/64 scope global \\       valid_lft forever\n\
                    2: eth0@if7    inet6 fe80::1/64 scope link \\       valid_lft forever\n";
        let a = parse(text);
        assert_eq!(a.len(), 4);
        assert_eq!(a[1].interface, "eth0");
        assert_eq!(a[1].addr, "10.200.0.10".parse::<IpAddr>().unwrap());
        assert_eq!(a[1].prefix_len, 24);
        assert_eq!(a[2].prefix_len, 64);
    }

    #[test]
    fn malformed_lines_are_skipped() {
        for bad in [
            "",
            "garbage",
            "2: eth0 inet 10.0.0.1",
            "2: eth0 inet 10.0.0.1/33 scope global",
            "2: eth0 inet6 10.0.0.1/24",
            "2: eth0 inet ::1/64",
            "2: eth0 link/ether 00:11:22:33:44:55 brd ff:ff:ff:ff:ff:ff",
            "2 eth0 inet 10.0.0.1/24",
            "2: averyveryverylongname inet 10.0.0.1/24",
            "2: eth0 inet 10.0.0.1/-1",
        ] {
            assert_eq!(parse(bad), vec![], "{bad:?}");
        }
        let many = "2: eth0 inet 10.0.0.1/24\n".repeat(MAX_ADDRESSES + 10);
        assert_eq!(parse(&many).len(), MAX_ADDRESSES);
    }
}
