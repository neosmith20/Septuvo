//! Device records: what the server knows about each enrolled device.
//!
//! **Before the G1 review no device can connect.** A record holds a name, a
//! scope and tunnel addresses, plus a [`PlaceholderKey`] where the device's
//! public key will go once reviewed enrollment exists. The placeholder holds
//! no bytes and authenticates nothing.
//!
//! The on-disk format is a small, strict text format (one record per line),
//! parsed as hostile input: anything unexpected makes the whole file
//! unreadable, and an unreadable file means the server refuses every device
//! (RV-2) rather than starting empty.

use septuvo_policy::Prefix;
use std::fmt::{self, Write as _};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Most devices one server keeps.
pub const MAX_DEVICES: usize = 4096;
/// Largest records file accepted, in bytes.
pub const MAX_FILE_BYTES: usize = 1024 * 1024;
/// First line of the format after comments.
pub const HEADER: &str = "septuvo-devices 1";
/// The placeholder key's on-disk form.
pub const PLACEHOLDER_KEY: &str = "placeholder-not-a-key";

/// What a device may reach (owner decision Q6 context: home network, internet
/// exit or both).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The home network only.
    Home,
    /// Internet exit only.
    Exit,
    /// Both.
    Both,
}

impl Scope {
    /// The scope's word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Home => "home",
            Scope::Exit => "exit",
            Scope::Both => "both",
        }
    }

    /// Parses `home`, `exit` or `both`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Scope> {
        match text {
            "home" => Some(Scope::Home),
            "exit" => Some(Scope::Exit),
            "both" => Some(Scope::Both),
            _ => None,
        }
    }
}

/// Where a device's public key will be once reviewed enrollment exists.
///
/// **Not a key.** It has no bytes, cannot be constructed from data and is
/// accepted by nothing that authenticates. It exists so that the record
/// format and the administration commands can be built and tested now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceholderKey;

impl fmt::Display for PlaceholderKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not a key (enrollment is not available before the G1 security review)")
    }
}

/// One device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Unique name chosen by the administrator.
    pub name: String,
    /// What the device may reach.
    pub scope: Scope,
    /// Tunnel IPv4 address.
    pub ipv4: Ipv4Addr,
    /// Tunnel IPv6 address.
    pub ipv6: Ipv6Addr,
    /// When the record was created (seconds since 1970, UTC).
    pub created: u64,
    /// When the device was revoked, if it was.
    pub revoked: Option<u64>,
    /// The device key: a placeholder before G1.
    pub key: PlaceholderKey,
}

/// A device name: 1 to 32 characters from `a-z 0-9 - _ .`, starting with a
/// letter or digit. Names appear in logs and terminals, so nothing else is
/// allowed (no spaces, no control characters, no upper case look-alikes).
#[must_use]
pub fn valid_name(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && name.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.')
        })
}

/// Why the records could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError {
    /// Line number (1-based), or 0 for the whole file.
    pub line: usize,
    /// What is wrong.
    pub reason: String,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: ", self.line)?;
        }
        f.write_str(&self.reason)
    }
}

impl std::error::Error for FormatError {}

fn bad<T>(line: usize, reason: impl Into<String>) -> Result<T, FormatError> {
    Err(FormatError {
        line,
        reason: reason.into(),
    })
}

/// All device records, checked against the server's tunnel ranges.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Records {
    devices: Vec<Device>,
}

impl Records {
    /// The devices, in the order they were added.
    #[must_use]
    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    /// The device called `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.name == name)
    }

    /// Serializes the records.
    #[must_use]
    pub fn encode(&self) -> String {
        let mut s = format!(
            "# Septuvo Server device records. Managed by septuvo-server; do not edit.\n{HEADER}\n"
        );
        for d in &self.devices {
            let revoked = d.revoked.map_or("-".to_string(), |t| t.to_string());
            let _ = writeln!(
                s,
                "device {} {} {} {} {} {revoked} {PLACEHOLDER_KEY}",
                d.name,
                d.scope.as_str(),
                d.ipv4,
                d.ipv6,
                d.created
            );
        }
        s
    }

    /// Parses records and checks every address against the configured ranges.
    ///
    /// # Errors
    /// The first problem; the caller must then refuse to serve any device.
    pub fn decode(text: &str, v4: Prefix, v6: Prefix) -> Result<Records, FormatError> {
        if text.len() > MAX_FILE_BYTES {
            return bad(0, format!("file is larger than {MAX_FILE_BYTES} bytes"));
        }
        let mut records = Records::default();
        let mut seen_header = false;
        for (index, line) in text.split('\n').enumerate() {
            let n = index + 1;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if !line.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
                return bad(n, "unexpected characters");
            }
            if !seen_header {
                if line != HEADER {
                    return bad(
                        n,
                        format!("expected '{HEADER}' (unknown or damaged format)"),
                    );
                }
                seen_header = true;
                continue;
            }
            let device = parse_device(n, line, v4, v6)?;
            records
                .check_new(&device)
                .map_err(|reason| FormatError { line: n, reason })?;
            if records.devices.len() == MAX_DEVICES {
                return bad(n, format!("more than {MAX_DEVICES} devices"));
            }
            records.devices.push(device);
        }
        if !seen_header {
            return bad(0, "the format header is missing");
        }
        Ok(records)
    }

    /// Checks that `device` does not clash with an existing record.
    fn check_new(&self, device: &Device) -> Result<(), String> {
        for d in &self.devices {
            if d.name == device.name {
                return Err(format!("device '{}' appears twice", device.name));
            }
            if d.ipv4 == device.ipv4 || d.ipv6 == device.ipv6 {
                return Err(format!(
                    "device '{}' reuses an address of '{}'",
                    device.name, d.name
                ));
            }
        }
        Ok(())
    }

    /// Adds a new device with the lowest free addresses.
    ///
    /// # Errors
    /// An invalid or existing name, or no free address.
    pub fn add(
        &mut self,
        name: &str,
        scope: Scope,
        now: u64,
        v4: Prefix,
        v6: Prefix,
    ) -> Result<&Device, String> {
        if !valid_name(name) {
            return Err(
                "names are 1-32 characters from a-z, 0-9, '-', '_' and '.', starting with a letter or digit"
                    .into(),
            );
        }
        if self.get(name).is_some() {
            return Err(format!("a device called '{name}' already exists"));
        }
        if self.devices.len() >= MAX_DEVICES {
            return Err(format!("at most {MAX_DEVICES} devices"));
        }
        let ipv4 = free_v4(v4, &self.devices).ok_or("no free IPv4 tunnel address in the range")?;
        let ipv6 = free_v6(v6, &self.devices).ok_or("no free IPv6 tunnel address in the range")?;
        let device = Device {
            name: name.to_string(),
            scope,
            ipv4,
            ipv6,
            created: now,
            revoked: None,
            key: PlaceholderKey,
        };
        self.check_new(&device)?;
        self.devices.push(device);
        Ok(&self.devices[self.devices.len() - 1])
    }

    /// Marks a device revoked. Its addresses stay reserved until it is
    /// removed, so they are never given to a new device by mistake.
    ///
    /// # Errors
    /// Unknown or already revoked device.
    pub fn revoke(&mut self, name: &str, now: u64) -> Result<(), String> {
        let d = self
            .devices
            .iter_mut()
            .find(|d| d.name == name)
            .ok_or(format!("no device called '{name}'"))?;
        if d.revoked.is_some() {
            return Err(format!("'{name}' is already revoked"));
        }
        d.revoked = Some(now);
        Ok(())
    }

    /// Deletes a record. Only revoked devices may be removed, so a device
    /// is never deleted while it is still allowed.
    ///
    /// # Errors
    /// Unknown device, or a device that has not been revoked.
    pub fn remove(&mut self, name: &str) -> Result<(), String> {
        let i = self
            .devices
            .iter()
            .position(|d| d.name == name)
            .ok_or(format!("no device called '{name}'"))?;
        if self.devices[i].revoked.is_none() {
            return Err(format!("revoke '{name}' before removing it"));
        }
        self.devices.remove(i);
        Ok(())
    }
}

fn parse_device(n: usize, line: &str, v4: Prefix, v6: Prefix) -> Result<Device, FormatError> {
    let parts: Vec<&str> = line.split(' ').collect();
    let ["device", name, scope, ipv4, ipv6, created, revoked, key] = parts.as_slice() else {
        return bad(n, "malformed device record");
    };
    if !valid_name(name) {
        return bad(n, "invalid device name");
    }
    let scope = Scope::parse(scope).ok_or(FormatError {
        line: n,
        reason: "invalid scope".into(),
    })?;
    let ipv4: Ipv4Addr = ipv4.parse().or_else(|_| bad(n, "invalid IPv4 address"))?;
    let ipv6: Ipv6Addr = ipv6.parse().or_else(|_| bad(n, "invalid IPv6 address"))?;
    if !usable_v4(v4, ipv4) || !usable_v6(v6, ipv6) {
        return bad(
            n,
            format!("'{name}' has addresses outside the configured tunnel ranges {v4} / {v6}"),
        );
    }
    let created: u64 = created
        .parse()
        .or_else(|_| bad(n, "invalid creation time"))?;
    let revoked = match *revoked {
        "-" => None,
        t => Some(t.parse().or_else(|_| bad(n, "invalid revocation time"))?),
    };
    if *key != PLACEHOLDER_KEY {
        // Real keys do not exist before G1; anything else is not ours.
        return bad(
            n,
            "unexpected key field (enrollment does not exist before G1)",
        );
    }
    Ok(Device {
        name: (*name).to_string(),
        scope,
        ipv4,
        ipv6,
        created,
        revoked,
        key: PlaceholderKey,
    })
}

fn host_count(p: Prefix, bits: u32) -> u128 {
    1u128 << (bits - u32::from(p.prefix_len()))
}

/// Offsets 0 (network), 1 (the server itself) and, for IPv4, the last
/// (broadcast) are never given to devices.
fn usable_v4(range: Prefix, addr: Ipv4Addr) -> bool {
    let IpAddr::V4(base) = range.addr() else {
        return false;
    };
    let offset = u128::from(u32::from(addr).wrapping_sub(u32::from(base)));
    range.contains(IpAddr::V4(addr)) && offset >= 2 && offset < host_count(range, 32) - 1
}

fn usable_v6(range: Prefix, addr: Ipv6Addr) -> bool {
    let IpAddr::V6(base) = range.addr() else {
        return false;
    };
    let offset = u128::from(addr).wrapping_sub(u128::from(base));
    range.contains(IpAddr::V6(addr)) && offset >= 2
}

/// The search for a free IPv6 address is bounded: [`MAX_DEVICES`] plus the
/// reserved offsets always contain a free one if fewer than the maximum
/// devices exist.
fn free_v4(range: Prefix, used: &[Device]) -> Option<Ipv4Addr> {
    let IpAddr::V4(base) = range.addr() else {
        return None;
    };
    let last = host_count(range, 32) - 1;
    (2..last)
        .take(MAX_DEVICES + 1)
        .filter_map(|off| u32::try_from(off).ok())
        .map(|off| Ipv4Addr::from(u32::from(base) + off))
        .find(|a| used.iter().all(|d| d.ipv4 != *a))
}

fn free_v6(range: Prefix, used: &[Device]) -> Option<Ipv6Addr> {
    let IpAddr::V6(base) = range.addr() else {
        return None;
    };
    let count = host_count(range, 128);
    (2..count)
        .take(MAX_DEVICES + 1)
        .map(|off| Ipv6Addr::from(u128::from(base) + off))
        .find(|a| used.iter().all(|d| d.ipv6 != *a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranges() -> (Prefix, Prefix) {
        (
            "10.77.0.0/24".parse().unwrap(),
            "fd77:77:77::/64".parse().unwrap(),
        )
    }

    #[test]
    fn add_allocates_lowest_free_addresses_and_skips_reserved_ones() {
        let (v4, v6) = ranges();
        let mut r = Records::default();
        let d = r.add("laptop", Scope::Both, 100, v4, v6).unwrap().clone();
        assert_eq!(
            d.ipv4,
            Ipv4Addr::new(10, 77, 0, 2),
            ".0 network and .1 server are reserved"
        );
        assert_eq!(d.ipv6, "fd77:77:77::2".parse::<Ipv6Addr>().unwrap());
        assert_eq!(d.key, PlaceholderKey);
        let p = r.add("phone", Scope::Home, 101, v4, v6).unwrap().ipv4;
        assert_eq!(p, Ipv4Addr::new(10, 77, 0, 3));
    }

    #[test]
    fn small_ranges_run_out_cleanly() {
        let v4: Prefix = "10.77.0.0/30".parse().unwrap();
        let (_, v6) = ranges();
        let mut r = Records::default();
        assert_eq!(
            r.add("a", Scope::Home, 1, v4, v6).unwrap().ipv4,
            Ipv4Addr::new(10, 77, 0, 2)
        );
        let e = r.add("b", Scope::Home, 1, v4, v6).unwrap_err();
        assert!(
            e.contains("no free IPv4"),
            "/30: only .2 is usable (.3 is broadcast)"
        );
    }

    #[test]
    fn revoked_addresses_stay_reserved_until_removed() {
        let (v4, v6) = ranges();
        let mut r = Records::default();
        r.add("old", Scope::Home, 1, v4, v6).unwrap();
        r.revoke("old", 2).unwrap();
        assert_eq!(
            r.add("new", Scope::Home, 3, v4, v6).unwrap().ipv4,
            Ipv4Addr::new(10, 77, 0, 3)
        );
        assert!(r.remove("new").is_err(), "must revoke first");
        r.remove("old").unwrap();
        assert_eq!(
            r.add("newer", Scope::Home, 4, v4, v6).unwrap().ipv4,
            Ipv4Addr::new(10, 77, 0, 2)
        );
        assert!(r.revoke("missing", 5).is_err());
        r.revoke("new", 5).unwrap();
        assert!(r.revoke("new", 6).is_err(), "already revoked");
    }

    #[test]
    fn hostile_names_are_rejected() {
        let (v4, v6) = ranges();
        let mut r = Records::default();
        for bad in [
            "",
            "Laptop",
            "-x",
            ".x",
            "a b",
            "a\nb",
            "a\u{1b}[31m",
            "a/b",
            "ä",
            "abcdefghijklmnopqrstuvwxyz0123456",
        ] {
            assert!(r.add(bad, Scope::Home, 1, v4, v6).is_err(), "{bad:?}");
        }
        r.add("ok-name_1.x", Scope::Home, 1, v4, v6).unwrap();
        assert!(
            r.add("ok-name_1.x", Scope::Home, 1, v4, v6).is_err(),
            "duplicate"
        );
    }

    #[test]
    fn encode_decode_round_trip() {
        let (v4, v6) = ranges();
        let mut r = Records::default();
        r.add("laptop", Scope::Both, 1_760_000_000, v4, v6).unwrap();
        r.add("phone", Scope::Exit, 1_760_000_100, v4, v6).unwrap();
        r.revoke("phone", 1_760_000_200).unwrap();
        let text = r.encode();
        assert!(text.contains("device phone exit 10.77.0.3 fd77:77:77::3 1760000100 1760000200 placeholder-not-a-key\n"));
        assert_eq!(Records::decode(&text, v4, v6).unwrap(), r);
    }

    #[test]
    fn damaged_files_are_refused_never_treated_as_empty() {
        let (v4, v6) = ranges();
        let good =
            "septuvo-devices 1\ndevice a home 10.77.0.2 fd77:77:77::2 1 - placeholder-not-a-key\n";
        assert!(Records::decode(good, v4, v6).is_ok());
        for bad in [
            "",
            "garbage\n",
            "septuvo-devices 2\n",
            &good.replace("home", "admin"),
            &good.replace("10.77.0.2", "10.77.1.2"),
            &good.replace("10.77.0.2", "10.77.0.1"),
            &good.replace("10.77.0.2", "10.77.0.255"),
            &good.replace("fd77:77:77::2", "fd77:77:78::2"),
            &good.replace(" - ", " never "),
            &good.replace("placeholder-not-a-key", "AAAAC3NzaC1lZDI1NTE5"),
            &good.replace(" 1 ", "  1 "),
            &format!("{good}device a home 10.77.0.3 fd77:77:77::3 1 - placeholder-not-a-key\n"),
            &format!("{good}device b home 10.77.0.2 fd77:77:77::3 1 - placeholder-not-a-key\n"),
            &good.replace("device a", "device a\u{7}"),
            &"#".repeat(MAX_FILE_BYTES + 1),
        ] {
            assert!(
                Records::decode(bad, v4, v6).is_err(),
                "{:?}",
                &bad[..bad.len().min(60)]
            );
        }
    }

    #[test]
    fn placeholder_key_says_it_is_not_a_key() {
        assert!(PlaceholderKey.to_string().starts_with("not a key"));
    }
}
