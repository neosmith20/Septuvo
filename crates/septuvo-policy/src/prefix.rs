//! IP address prefixes (CIDR blocks) with strict, canonical parsing.

use std::fmt;
use std::net::IpAddr;
use std::str::FromStr;

/// Address family of a prefix or address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// IPv4.
    V4,
    /// IPv6.
    V6,
}

impl Family {
    /// Family of an address. IPv4-mapped IPv6 addresses (`::ffff:a.b.c.d`)
    /// are IPv6: they are never matched against IPv4 rules.
    #[must_use]
    pub fn of(addr: IpAddr) -> Self {
        match addr {
            IpAddr::V4(_) => Family::V4,
            IpAddr::V6(_) => Family::V6,
        }
    }

    /// Number of bits in an address of this family.
    #[must_use]
    pub fn bits(self) -> u8 {
        match self {
            Family::V4 => 32,
            Family::V6 => 128,
        }
    }
}

/// A canonical IP prefix such as `192.168.1.0/24` or `2001:db8::/32`.
///
/// Canonical means the host bits are zero, so every prefix has exactly one
/// textual form and `192.168.1.7/24` is rejected instead of being guessed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Prefix {
    addr: IpAddr,
    len: u8,
}

/// Why a prefix was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixError {
    /// No `/length` part.
    MissingLength,
    /// The address part is not a valid IPv4 or IPv6 address.
    BadAddress,
    /// The length is not a plain decimal number.
    BadLength,
    /// The length is larger than the family allows (32 or 128).
    LengthTooLong,
    /// Bits beyond the prefix length are set (for example `10.0.0.1/8`).
    HostBitsSet,
}

impl fmt::Display for PrefixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            PrefixError::MissingLength => "missing '/length'",
            PrefixError::BadAddress => "not a valid IPv4 or IPv6 address",
            PrefixError::BadLength => "length must be a plain decimal number",
            PrefixError::LengthTooLong => "length is too long for the address family",
            PrefixError::HostBitsSet => "address has bits set beyond the prefix length",
        };
        f.write_str(text)
    }
}

impl std::error::Error for PrefixError {}

/// The address as an unsigned integer (IPv4 in the low 32 bits).
fn to_bits(addr: IpAddr) -> u128 {
    match addr {
        IpAddr::V4(a) => u128::from(u32::from(a)),
        IpAddr::V6(a) => u128::from(a),
    }
}

/// The address for `bits` in `family`.
fn from_bits(bits: u128, family: Family) -> IpAddr {
    match family {
        Family::V4 => IpAddr::V4(std::net::Ipv4Addr::from(
            u32::try_from(bits & 0xffff_ffff).unwrap_or(0),
        )),
        Family::V6 => IpAddr::V6(std::net::Ipv6Addr::from(bits)),
    }
}

/// Network mask with `len` leading one bits within a `width`-bit address.
fn mask(len: u8, width: u8) -> u128 {
    debug_assert!(len <= width && (width == 32 || width == 128));
    let all = if width == 128 {
        u128::MAX
    } else {
        (1u128 << width) - 1
    };
    if len == 0 {
        0
    } else {
        all & (u128::MAX << (width - len))
    }
}

impl Prefix {
    /// Builds a prefix, rejecting lengths that are too long and set host bits.
    ///
    /// # Errors
    /// [`PrefixError::LengthTooLong`] or [`PrefixError::HostBitsSet`].
    pub fn new(addr: IpAddr, len: u8) -> Result<Self, PrefixError> {
        let width = Family::of(addr).bits();
        if len > width {
            return Err(PrefixError::LengthTooLong);
        }
        if to_bits(addr) & !mask(len, width) != 0 {
            return Err(PrefixError::HostBitsSet);
        }
        Ok(Prefix { addr, len })
    }

    /// The network address.
    #[must_use]
    pub fn addr(&self) -> IpAddr {
        self.addr
    }

    /// The prefix length in bits.
    #[must_use]
    pub fn prefix_len(&self) -> u8 {
        self.len
    }

    /// The address family.
    #[must_use]
    pub fn family(&self) -> Family {
        Family::of(self.addr)
    }

    /// The prefix of length `len` that contains `addr` (host bits cleared),
    /// for example `192.168.1.30` and 24 give `192.168.1.0/24`.
    ///
    /// # Errors
    /// [`PrefixError::LengthTooLong`].
    pub fn containing(addr: IpAddr, len: u8) -> Result<Self, PrefixError> {
        let width = Family::of(addr).bits();
        if len > width {
            return Err(PrefixError::LengthTooLong);
        }
        let bits = to_bits(addr) & mask(len, width);
        Ok(Prefix {
            addr: from_bits(bits, Family::of(addr)),
            len,
        })
    }

    /// The last address of the prefix (for IPv4, the subnet broadcast
    /// address).
    #[must_use]
    pub fn last(&self) -> IpAddr {
        let width = self.family().bits();
        let all = if width == 128 {
            u128::MAX
        } else {
            (1u128 << width) - 1
        };
        from_bits(
            to_bits(self.addr) | (all & !mask(self.len, width)),
            self.family(),
        )
    }

    /// True if the two prefixes share any address (one contains the other).
    #[must_use]
    pub fn overlaps(&self, other: &Prefix) -> bool {
        self.family() == other.family() && (self.contains(other.addr) || other.contains(self.addr))
    }

    /// True if `addr` is inside this prefix. Addresses of the other family
    /// never match.
    #[must_use]
    pub fn contains(&self, addr: IpAddr) -> bool {
        Family::of(addr) == self.family()
            && to_bits(addr) & mask(self.len, self.family().bits()) == to_bits(self.addr)
    }
}

impl FromStr for Prefix {
    type Err = PrefixError;

    /// Parses `address/length`. The length must be decimal digits only, without
    /// a sign or leading zeros; IPv6 zone identifiers (`%eth0`) are rejected.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (addr_text, len_text) = text.split_once('/').ok_or(PrefixError::MissingLength)?;
        let addr = IpAddr::from_str(addr_text).map_err(|_| PrefixError::BadAddress)?;
        let digits_ok = !len_text.is_empty()
            && len_text.len() <= 3
            && len_text.bytes().all(|b| b.is_ascii_digit())
            && !(len_text.len() > 1 && len_text.starts_with('0'));
        if !digits_ok {
            return Err(PrefixError::BadLength);
        }
        let len: u8 = len_text.parse().map_err(|_| PrefixError::LengthTooLong)?;
        Prefix::new(addr, len)
    }
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.addr, self.len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn p(text: &str) -> Prefix {
        text.parse().unwrap()
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn parses_canonical_prefixes() {
        for text in [
            "0.0.0.0/0",
            "10.0.0.0/8",
            "192.168.1.0/24",
            "203.0.113.7/32",
            "::/0",
            "2001:db8::/32",
            "fe80::/10",
            "2001:db8::1/128",
        ] {
            assert_eq!(p(text).to_string(), text);
        }
    }

    #[test]
    fn rejects_bad_input() {
        let cases: &[(&str, PrefixError)] = &[
            ("", PrefixError::MissingLength),
            ("192.168.1.0", PrefixError::MissingLength),
            ("192.168.1.0/", PrefixError::BadLength),
            ("192.168.1.0/+24", PrefixError::BadLength),
            ("192.168.1.0/024", PrefixError::BadLength),
            ("192.168.1.0/-1", PrefixError::BadLength),
            ("192.168.1.0/2 4", PrefixError::BadLength),
            ("192.168.1.0/1000", PrefixError::BadLength),
            ("192.168.1.0/33", PrefixError::LengthTooLong),
            ("2001:db8::/129", PrefixError::LengthTooLong),
            ("192.168.1.0/256", PrefixError::LengthTooLong),
            ("192.168.1.7/24", PrefixError::HostBitsSet),
            ("10.0.0.1/8", PrefixError::HostBitsSet),
            ("2001:db8::1/64", PrefixError::HostBitsSet),
            ("192.168.01.0/24", PrefixError::BadAddress),
            ("192.168.1/24", PrefixError::BadAddress),
            ("fe80::1%eth0/128", PrefixError::BadAddress),
            (" 192.168.1.0/24", PrefixError::BadAddress),
            ("example.com/24", PrefixError::BadAddress),
            ("192.168.1.0/24/8", PrefixError::BadLength),
        ];
        for (text, expected) in cases {
            assert_eq!(text.parse::<Prefix>(), Err(*expected), "input {text:?}");
        }
    }

    #[test]
    fn contains_respects_boundaries() {
        let net = p("192.168.1.0/24");
        assert!(net.contains(ip("192.168.1.0")));
        assert!(net.contains(ip("192.168.1.255")));
        assert!(!net.contains(ip("192.168.0.255")));
        assert!(!net.contains(ip("192.168.2.0")));

        let v6 = p("2001:db8::/32");
        assert!(v6.contains(ip("2001:db8:ffff::1")));
        assert!(!v6.contains(ip("2001:db9::1")));
    }

    #[test]
    fn default_routes_cover_only_their_family() {
        assert!(p("0.0.0.0/0").contains(ip("198.51.100.1")));
        assert!(!p("0.0.0.0/0").contains(ip("2001:db8::1")));
        assert!(p("::/0").contains(ip("2001:db8::1")));
        assert!(!p("::/0").contains(ip("198.51.100.1")));
    }

    #[test]
    fn ipv4_mapped_ipv6_never_matches_ipv4_prefix() {
        let mapped = IpAddr::V6(Ipv4Addr::new(192, 168, 1, 10).to_ipv6_mapped());
        assert!(!p("192.168.1.0/24").contains(mapped));
        assert!(!p("0.0.0.0/0").contains(mapped));
        assert!(p("::/0").contains(mapped));
    }

    /// Compares `contains` with a straightforward reference for every IPv4
    /// prefix length and a spread of addresses.
    #[test]
    fn ipv4_contains_matches_reference_for_every_length() {
        let base = u32::from(Ipv4Addr::new(172, 16, 128, 0));
        let probes = [
            0u32,
            1,
            0x7f,
            0x80,
            0xff,
            0x100,
            0xffff,
            0x1_0000,
            0x00ff_ffff,
            0x0100_0000,
        ];
        for len in 0..=32u8 {
            let net_mask = if len == 0 { 0 } else { u32::MAX << (32 - len) };
            let net = base & net_mask;
            let prefix = Prefix::new(IpAddr::V4(Ipv4Addr::from(net)), len).unwrap();
            for delta in probes {
                for addr in [net.wrapping_add(delta), net.wrapping_sub(delta)] {
                    let expected = addr & net_mask == net;
                    assert_eq!(
                        prefix.contains(IpAddr::V4(Ipv4Addr::from(addr))),
                        expected,
                        "len {len} addr {addr:#x}"
                    );
                }
            }
        }
    }

    #[test]
    fn ipv6_full_and_empty_lengths() {
        let host = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let full = Prefix::new(IpAddr::V6(host), 128).unwrap();
        assert!(full.contains(IpAddr::V6(host)));
        assert!(!full.contains(ip("2001:db8::2")));
        assert_eq!(
            Prefix::new(IpAddr::V6(host), 0),
            Err(PrefixError::HostBitsSet)
        );
    }
}
