//! Device configuration (`septuvo.toml`).
//!
//! The file says which server to use, which traffic goes through the tunnel
//! (the N1 policy model), how DNS is handled and a few local settings.
//! Security settings are never silently switched off: when DNS handling is
//! not configured it defaults to the tunnel, the kill switch cannot be
//! disabled (owner decision Q2, D-032, D-038) and nothing here can enroll a
//! device.
//!
//! A saved connection (N4-03, N4-D11) is this file with an optional
//! `[connection]` table naming it. The service stores the name separately
//! ([`split_connection`]) and adds it back when the connection is exported,
//! so an exported file imports again under the same name.

use crate::error::{ConfigError, Problem};
use crate::schema::{self, Section};
use crate::toml::{self, Document, Span};
use septuvo_policy::{Action, Mode, PathState, Policy, Rule, Verdict, nft};
use std::fmt::{self, Write as _};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;

/// Most DNS resolvers in one configuration.
pub const MAX_RESOLVERS: usize = 4;
/// Most LAN DNS exceptions in one configuration.
pub const MAX_LAN_DNS_EXCEPTIONS: usize = 8;
/// Longest connection name, in characters (DEFAULT-UI §4.1).
pub const MAX_NAME_CHARS: usize = 64;
/// Default tunnel interface name.
pub const DEFAULT_INTERFACE: &str = "septuvo0";
/// Default inner MTU (wire-format.md section 5.2).
pub const DEFAULT_MTU: u16 = 1420;
/// Smallest inner MTU: IPv6 needs at least 1280 bytes.
pub const MIN_MTU: i64 = 1280;
/// Largest inner MTU accepted.
pub const MAX_MTU: i64 = 9000;

/// The server identity the device would pin at enrollment.
///
/// **Placeholder.** Enrollment does not exist before the G1 review, so the
/// only value is [`ServerIdentity::NotEnrolled`]. It cannot be set from a
/// file and authenticates nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerIdentity {
    /// No server identity is pinned (enrollment is not available before G1).
    NotEnrolled,
}

impl fmt::Display for ServerIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not enrolled (enrollment is not available before the G1 review)")
    }
}

/// How the device resolves names. Only tunnel DNS exists: the system's
/// DNS is refused until per-domain DNS exists (NP-D10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsPolicy {
    /// DNS goes only to these resolvers, through the tunnel; all other DNS
    /// is blocked, except queries to the listed LAN resolvers.
    Tunnel {
        /// Resolvers reached through the tunnel.
        resolvers: Vec<IpAddr>,
        /// LAN resolvers explicitly allowed to receive DNS directly.
        lan_exceptions: Vec<IpAddr>,
    },
}

/// Local tunnel interface settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelSettings {
    /// Interface name.
    pub interface: String,
    /// Inner MTU in bytes.
    pub mtu: u16,
}

/// A validated device configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceConfig {
    /// Server endpoints (IP address and UDP port), in preference order.
    pub endpoints: Vec<SocketAddr>,
    /// Pinned server identity (placeholder before G1).
    pub server_identity: ServerIdentity,
    /// Which destinations use the tunnel.
    pub policy: Policy,
    /// DNS handling.
    pub dns: DnsPolicy,
    /// Tunnel interface settings.
    pub tunnel: TunnelSettings,
    /// The connection's friendly name, from the optional `[connection]`
    /// table (N4-03).
    pub name: Option<String>,
    /// Log retention in days (Q10).
    pub log_retention_days: u16,
}

const TABLES: &[&str] = &[
    "server",
    "routing",
    "dns",
    "tunnel",
    "logging",
    "connection",
];
const FORBIDDEN: &[(&str, &str)] = &[
    (
        "kill_switch",
        "the kill switch is always on and cannot be configured (owner decision Q2): \
         it is lifted only by a deliberate disconnect",
    ),
    (
        "behavior",
        "behavior.always_on was retired (D-038): blocking without the VPN is Lockdown Mode, \
         a service setting, and connecting at startup is an app setting; remove the [behavior] table",
    ),
];

/// Parses and validates a device configuration.
///
/// # Errors
/// The first problem found, with its location.
pub fn parse_device(input: &str) -> Result<DeviceConfig, ConfigError> {
    device_from_document(&toml::parse(input)?)
}

/// Reads, parses and validates a device configuration file.
///
/// # Errors
/// A read error, or the first problem found in the file.
pub fn load_device(path: &Path) -> Result<DeviceConfig, ConfigError> {
    let bytes = schema::read_bounded(path)?;
    device_from_document(&toml::parse_bytes(&bytes)?)
}

/// Validates a parsed document as a device configuration.
///
/// # Errors
/// The first problem found, with its location.
pub fn device_from_document(doc: &Document) -> Result<DeviceConfig, ConfigError> {
    schema::check_document(doc, TABLES, FORBIDDEN)?;
    let policy = routing(doc)?;
    let endpoints = endpoints(doc, &policy)?;
    let dns = dns(doc, &policy)?;
    let tunnel = tunnel(doc)?;
    let name = match doc.table("connection") {
        None => None,
        Some(table) => {
            let s = Section { table };
            s.only(&["name"])?;
            match s.string("name")? {
                None => return Err(s.missing("name")),
                Some((text, span)) => {
                    Some(connection_name(text).map_err(|why| s.invalid("name", span, why))?)
                }
            }
        }
    };
    Ok(DeviceConfig {
        endpoints,
        server_identity: ServerIdentity::NotEnrolled,
        policy,
        dns,
        tunnel,
        name,
        log_retention_days: schema::retention_days(doc)?,
    })
}

fn routing(doc: &Document) -> Result<Policy, ConfigError> {
    let s = schema::required_table(doc, "routing")?;
    s.only(&["mode", "tunnel", "direct", "block"])?;
    let (mode_text, mode_span) = s.string("mode")?.ok_or_else(|| s.missing("mode"))?;
    let mode = match mode_text {
        "full-tunnel" => Mode::FullTunnel,
        "split" => Mode::Split,
        _ => return Err(s.invalid("mode", mode_span, "must be \"full-tunnel\" or \"split\"")),
    };
    let mut rules = Vec::new();
    for (key, action) in [
        ("tunnel", Action::Tunnel),
        ("direct", Action::Direct),
        ("block", Action::Block),
    ] {
        for (text, span) in s.strings(key)?.unwrap_or_default() {
            let prefix = schema::prefix(text).map_err(|why| s.invalid(key, *span, why))?;
            rules.push(Rule { prefix, action });
        }
    }
    Policy::new(mode, rules).map_err(|e| s.conflict("mode", mode_span, e.to_string()))
}

fn endpoints(doc: &Document, policy: &Policy) -> Result<Vec<SocketAddr>, ConfigError> {
    let s = schema::required_table(doc, "server")?;
    s.only(&["endpoints", "identity"])?;
    if let Some(e) = s.table.get("identity") {
        return Err(ConfigError::new(
            Some(e.key_span),
            s.path("identity"),
            Problem::NotAvailableBeforeG1,
        ));
    }
    let items = s
        .strings("endpoints")?
        .ok_or_else(|| s.missing("endpoints"))?;
    if items.is_empty() || items.len() > nft::MAX_ENDPOINTS {
        return Err(s.invalid(
            "endpoints",
            items.first().map_or(s.table.span, |(_, sp)| *sp),
            format!(
                "between 1 and {} endpoints are required",
                nft::MAX_ENDPOINTS
            ),
        ));
    }
    let mut out: Vec<SocketAddr> = Vec::new();
    for (text, span) in items {
        let addr = endpoint(text).map_err(|why| s.invalid("endpoints", *span, why))?;
        if out.contains(&addr) {
            return Err(s.invalid("endpoints", *span, format!("'{text}' is listed twice")));
        }
        // An endpoint inside an explicit tunnel or block rule could never be
        // reached (it would have to go through the tunnel it creates).
        if let Some(rule) = policy.rules().iter().find(|r| r.prefix.contains(addr.ip()))
            && rule.action != Action::Direct
        {
            return Err(s.conflict(
                "endpoints",
                *span,
                format!("endpoint {addr} is inside the routing rule {}", rule.prefix),
            ));
        }
        out.push(addr);
    }
    Ok(out)
}

/// `203.0.113.10:51820` or `[2001:db8::10]:51820`; host names are refused
/// (no underlay DNS is needed to connect, network-policies NP-U3).
fn endpoint(text: &str) -> Result<SocketAddr, String> {
    let addr: SocketAddr = text.parse().map_err(|_| {
        format!("'{text}' is not an address and port such as 203.0.113.10:51820 or [2001:db8::10]:51820 (host names are not accepted)")
    })?;
    schema::unicast_address(&addr.ip().to_string())?;
    if addr.port() == 0 {
        return Err(format!("'{text}': port 0 is not a valid port"));
    }
    Ok(addr)
}

fn address_list(s: Section<'_>, key: &str, max: usize) -> Result<Vec<(IpAddr, Span)>, ConfigError> {
    let items = s.strings(key)?.unwrap_or_default();
    if items.len() > max {
        return Err(s.invalid(key, items[max].1, format!("at most {max} entries")));
    }
    let mut out: Vec<(IpAddr, Span)> = Vec::new();
    for (text, span) in items {
        let addr = schema::unicast_address(text).map_err(|why| s.invalid(key, *span, why))?;
        if out.iter().any(|(a, _)| *a == addr) {
            return Err(s.invalid(key, *span, format!("'{text}' is listed twice")));
        }
        out.push((addr, *span));
    }
    Ok(out)
}

fn dns(doc: &Document, policy: &Policy) -> Result<DnsPolicy, ConfigError> {
    let Some(table) = doc.table("dns") else {
        // No [dns] table: tunnel DNS is the safe default, but it needs resolvers.
        return Err(ConfigError::new(
            Some(Span { line: 1, column: 1 }),
            "dns",
            Problem::Missing,
        ));
    };
    let s = Section { table };
    s.only(&["mode", "resolvers", "lan_exceptions"])?;
    let mode = s.string("mode")?;
    match mode {
        None | Some(("tunnel", _)) => {}
        Some(("system", span)) => {
            // NP-D10: refused in both modes until the protected domains are
            // always sent through the tunnel (per-domain DNS, NP-D2).
            let why = if policy.mode() == Mode::FullTunnel {
                "\"system\" DNS would send queries outside the tunnel in full-tunnel mode"
            } else {
                "\"system\" DNS is not available until names in the protected networks are \
                 always resolved through the tunnel (NP-D10); use mode = \"tunnel\""
            };
            return Err(s.conflict("mode", span, why));
        }
        Some((_, span)) => {
            return Err(s.invalid("mode", span, "must be \"tunnel\""));
        }
    }
    let resolvers = address_list(s, "resolvers", MAX_RESOLVERS)?;
    if resolvers.is_empty() {
        return Err(s.missing("resolvers"));
    }
    for (addr, span) in &resolvers {
        if policy.decide(*addr, PathState::Up).verdict != Verdict::Tunnel {
            return Err(s.conflict(
                "resolvers",
                *span,
                format!("resolver {addr} would not be reached through the tunnel under [routing]"),
            ));
        }
    }
    let lan = address_list(s, "lan_exceptions", MAX_LAN_DNS_EXCEPTIONS)?;
    for (addr, span) in &lan {
        if policy.decide(*addr, PathState::Down).verdict != Verdict::Direct {
            return Err(s.conflict(
                "lan_exceptions",
                *span,
                format!("{addr} must also be covered by a [routing] direct rule"),
            ));
        }
    }
    Ok(DnsPolicy::Tunnel {
        resolvers: resolvers.into_iter().map(|(a, _)| a).collect(),
        lan_exceptions: lan.into_iter().map(|(a, _)| a).collect(),
    })
}

/// Exactly the kill-switch renderer's rule (one shared function): 1 to 15
/// characters from `A-Z a-z 0-9 _ - .`, not `.` or `..`, not starting with
/// `-`. The rules had drifted apart, so a configuration the validator
/// accepted could fail at connect (found by the N3-03 fuzzing, case seed
/// 0xc8c5a99ba1125cd2).
fn interface_name(name: &str) -> bool {
    septuvo_policy::nft::valid_interface(name)
}

fn tunnel(doc: &Document) -> Result<TunnelSettings, ConfigError> {
    let mut settings = TunnelSettings {
        interface: DEFAULT_INTERFACE.to_string(),
        mtu: DEFAULT_MTU,
    };
    let Some(table) = doc.table("tunnel") else {
        return Ok(settings);
    };
    let s = Section { table };
    s.only(&["interface", "mtu"])?;
    if let Some((name, span)) = s.string("interface")? {
        if !interface_name(name) {
            return Err(s.invalid(
                "interface",
                span,
                "1 to 15 characters from A-Z, a-z, 0-9, '_', '-' and '.', not starting with '-'",
            ));
        }
        settings.interface = name.to_string();
    }
    if let Some((mtu, _)) = s.int("mtu", MIN_MTU, MAX_MTU)? {
        settings.mtu = u16::try_from(mtu).unwrap_or(DEFAULT_MTU);
    }
    Ok(settings)
}

impl DeviceConfig {
    /// A short human-readable summary, for `septuvo-config check`.
    #[must_use]
    pub fn summary(&self) -> String {
        let mode = match self.policy.mode() {
            Mode::FullTunnel => "full tunnel",
            Mode::Split => "split (only listed networks are protected)",
        };
        let dns = match &self.dns {
            DnsPolicy::Tunnel {
                resolvers,
                lan_exceptions,
            } => format!(
                "through the tunnel to {}; other DNS blocked{}",
                join(resolvers),
                if lan_exceptions.is_empty() {
                    String::new()
                } else {
                    format!("; LAN exceptions {}", join(lan_exceptions))
                }
            ),
        };
        format!(
            "name:             {}\nserver endpoints: {}\nserver identity:  {}\n\
             routing:          {mode}, {} rule(s)\ndns:              {dns}\n\
             interface:        {} (MTU {})\nlog retention:    {} day(s)\n\
             kill switch:      always on (cannot be configured)",
            self.name.as_deref().unwrap_or("(none)"),
            join(&self.endpoints),
            self.server_identity,
            self.policy.rules().len(),
            self.tunnel.interface,
            self.tunnel.mtu,
            self.log_retention_days,
        )
    }
}

/// A valid connection name, trimmed: 1 to [`MAX_NAME_CHARS`] characters,
/// no control characters (DEFAULT-UI §4.1). Uniqueness is the store's job.
///
/// # Errors
/// Why the name cannot be used.
pub fn connection_name(text: &str) -> Result<String, String> {
    let name = text.trim();
    if name.is_empty() {
        return Err("the name is empty".into());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!(
            "the name is longer than {MAX_NAME_CHARS} characters"
        ));
    }
    if name
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
    {
        return Err("the name contains control characters".into());
    }
    Ok(name.to_string())
}

/// Validates a saved connection's text and separates its name: returns the
/// configuration (with `name` set if the text had one) and the text with
/// the `[connection]` table removed, as the service stores it.
///
/// # Errors
/// The first problem found, as [`parse_device`].
pub fn split_connection(input: &str) -> Result<(DeviceConfig, String), ConfigError> {
    let doc = toml::parse(input)?;
    let config = device_from_document(&doc)?;
    let Some(pos) = doc.tables.iter().position(|t| t.name == "connection") else {
        return Ok((config, input.to_string()));
    };
    // Tables are line-based: the table runs from its header line to the line
    // before the next header (or the end).
    let first = doc.tables[pos].span.line;
    let next = doc.tables.get(pos + 1).map(|t| t.span.line);
    let body = input.strip_prefix('\u{feff}').unwrap_or(input);
    let kept: Vec<&str> = body
        .split_inclusive('\n')
        .enumerate()
        .filter(|(i, _)| {
            let line = i + 1;
            line < first || next.is_some_and(|n| line >= n)
        })
        .map(|(_, l)| l)
        .collect();
    Ok((config, kept.concat()))
}

/// `text` as a TOML basic string, quotes included. Control characters
/// (which a valid name never has) are written as `\uXXXX` escapes.
#[must_use]
pub fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The `[connection]` table to append to stored text for export.
#[must_use]
pub fn name_table(name: &str) -> String {
    format!("\n[connection]\nname = {}\n", quote(name))
}

fn join<T: fmt::Display>(items: &[T]) -> String {
    items
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = r#"
version = 1

[server]
endpoints = ["203.0.113.10:51820", "[2001:db8::10]:51820"]

[routing]
mode = "full-tunnel"
direct = ["192.168.1.30/32", "2001:db8:1::30/128"]

[dns]
resolvers = ["10.77.0.1", "fd77::1"]
lan_exceptions = ["192.168.1.30"]
"#;

    fn problem(input: &str) -> (String, Problem) {
        let e = parse_device(input).expect_err("configuration should be rejected");
        (e.path, e.problem)
    }

    fn with(replace: &str, by: &str) -> String {
        assert!(FULL.contains(replace), "fixture text {replace:?} not found");
        FULL.replace(replace, by)
    }

    #[test]
    fn full_tunnel_example_is_valid_with_safe_defaults() {
        let c = parse_device(FULL).expect("valid");
        assert_eq!(c.endpoints.len(), 2);
        assert_eq!(c.server_identity, ServerIdentity::NotEnrolled);
        assert_eq!(c.policy.mode(), Mode::FullTunnel);
        assert_eq!(c.tunnel.interface, DEFAULT_INTERFACE);
        assert_eq!(c.tunnel.mtu, DEFAULT_MTU);
        assert_eq!(c.name, None);
        assert_eq!(c.log_retention_days, 7);
        let DnsPolicy::Tunnel {
            resolvers,
            lan_exceptions,
        } = &c.dns;
        assert_eq!(resolvers.len(), 2);
        assert_eq!(lan_exceptions.len(), 1);
        assert!(c.summary().contains("not enrolled"));
        assert!(c.summary().contains("kill switch:      always on"));
    }

    /// NP-D10: refused in split mode too, until per-domain DNS exists.
    #[test]
    fn system_dns_is_refused_in_both_modes() {
        let split = r#"version = 1
[server]
endpoints = ["198.51.100.7:4000"]
[routing]
mode = "split"
tunnel = ["192.168.50.0/24"]
[dns]
mode = "system"
"#;
        let (path, p) = problem(split);
        assert_eq!(path, "dns.mode");
        assert!(
            matches!(p, Problem::Conflict(ref why) if why.contains("NP-D10")),
            "{p:?}"
        );
        let (path, p) = problem(&with("[dns]\n", "[dns]\nmode = \"system\"\n"));
        assert_eq!(path, "dns.mode");
        assert!(matches!(p, Problem::Conflict(_)));
    }

    /// D-038: `behavior.always_on` is retired; the file says what replaced it.
    #[test]
    fn always_on_is_retired() {
        for extra in [
            "[behavior]\nalways_on = false\n",
            "[behavior]\nalways_on = true\n",
            "[behavior]\n",
        ] {
            let (path, p) = problem(&format!("{FULL}{extra}"));
            assert_eq!(path, "behavior");
            assert!(
                matches!(p, Problem::Conflict(ref why) if why.contains("Lockdown Mode")),
                "{p:?}"
            );
        }
    }

    #[test]
    fn connection_names() {
        let c =
            parse_device(&format!("{FULL}[connection]\nname = \"  Home VPN  \"\n")).expect("valid");
        assert_eq!(c.name.as_deref(), Some("Home VPN"));
        assert!(c.summary().contains("name:             Home VPN"));
        let unicode = "\u{1f3e0} Café – Büro";
        assert_eq!(connection_name(unicode).as_deref(), Ok(unicode));
        assert_eq!(
            connection_name(&"é".repeat(MAX_NAME_CHARS)).map(|n| n.chars().count()),
            Ok(MAX_NAME_CHARS)
        );
        for bad in [
            "",
            "   ",
            "a\u{7}b",
            "line\nbreak",
            "tab\there",
            "x\u{2028}y",
        ] {
            assert!(connection_name(bad).is_err(), "{bad:?}");
        }
        assert!(connection_name(&"a".repeat(MAX_NAME_CHARS + 1)).is_err());
        for extra in [
            "[connection]\n",
            "[connection]\nname = \"\"\n",
            "[connection]\nname = \"x\"\nother = 1\n",
            "[connection]\nname = 7\n",
        ] {
            assert!(
                parse_device(&format!("{FULL}{extra}")).is_err(),
                "{extra:?}"
            );
        }
    }

    #[test]
    fn split_connection_removes_only_the_name_table() {
        let text = format!(
            "{FULL}[connection]\nname = \"Home \\\"VPN\\\"\"  # note\n[tunnel]\nmtu = 1280\n"
        );
        let (c, stored) = split_connection(&text).expect("valid");
        assert_eq!(c.name.as_deref(), Some("Home \"VPN\""));
        assert!(
            !stored.contains("[connection]") && !stored.contains("name ="),
            "{stored}"
        );
        assert!(stored.contains("[tunnel]\nmtu = 1280"));
        let again = parse_device(&stored).expect("stored text is valid");
        assert_eq!(again.name, None);
        assert_eq!(again.tunnel.mtu, 1280);
        // Export adds the name back; importing that gives the same connection.
        let exported = format!(
            "{stored}{}",
            name_table(c.name.as_deref().unwrap_or_default())
        );
        let (back, stored2) = split_connection(&exported).expect("re-importable");
        assert_eq!(back, c);
        assert_eq!(stored2.trim_end(), stored.trim_end());
        // Without a name the text is kept as it is.
        let (plain, kept) = split_connection(FULL).expect("valid");
        assert_eq!((plain.name, kept.as_str()), (None, FULL));
    }

    #[test]
    fn optional_settings_are_read() {
        let input = format!(
            "{FULL}[tunnel]\ninterface = \"wg-test.1\"\nmtu = 1280\n[logging]\nretention_days = 0\n"
        );
        let c = parse_device(&input).expect("valid");
        assert_eq!(
            (
                c.tunnel.interface.as_str(),
                c.tunnel.mtu,
                c.log_retention_days
            ),
            ("wg-test.1", 1280, 0)
        );
    }

    #[test]
    fn missing_required_settings() {
        assert_eq!(
            problem(&with("version = 1", "")),
            ("version".into(), Problem::Missing)
        );
        assert_eq!(
            problem(&with("[dns]\nresolvers", "[dnsx]\nresolvers")),
            ("dnsx".into(), Problem::UnknownTable)
        );
        assert_eq!(
            problem(&with("resolvers = [\"10.77.0.1\", \"fd77::1\"]\n", "")),
            ("dns.resolvers".into(), Problem::Missing)
        );
        assert_eq!(
            problem(&with("mode = \"full-tunnel\"", "")),
            ("routing.mode".into(), Problem::Missing)
        );
    }

    #[test]
    fn missing_dns_table_is_an_error_not_a_leak() {
        let input = FULL.split("[dns]").next().expect("prefix").to_string();
        assert_eq!(problem(&input), ("dns".into(), Problem::Missing));
    }

    #[test]
    fn unknown_keys_tables_and_versions() {
        assert_eq!(
            problem(&with("[server]", "[server]\nretries = 3")),
            ("server.retries".into(), Problem::UnknownKey)
        );
        assert_eq!(
            problem(&with("version = 1", "version = 2")).1,
            Problem::UnsupportedVersion(2)
        );
        assert_eq!(
            problem(&with("version = 1", "version = \"1\"")).1,
            Problem::WrongType {
                expected: "an integer",
                found: "string"
            }
        );
    }

    #[test]
    fn kill_switch_cannot_be_disabled() {
        let (path, p) = problem(&format!("{FULL}[kill_switch]\nenabled = false\n"));
        assert_eq!(path, "kill_switch");
        assert!(matches!(p, Problem::Conflict(_)));
    }

    #[test]
    fn identity_cannot_be_set_before_g1() {
        assert_eq!(
            problem(&with("[server]", "[server]\nidentity = \"abc\"")),
            ("server.identity".into(), Problem::NotAvailableBeforeG1)
        );
    }

    #[test]
    fn invalid_endpoints() {
        for bad in [
            "vpn.example.com:51820",
            "203.0.113.10",
            "203.0.113.10:0",
            "0.0.0.0:51820",
            "224.0.0.1:51820",
            "127.0.0.1:51820",
            "[::1]:51820",
            "2001:db8::10:51820",
        ] {
            let input = with(
                "endpoints = [\"203.0.113.10:51820\", \"[2001:db8::10]:51820\"]",
                &format!("endpoints = [\"{bad}\"]"),
            );
            assert!(
                matches!(problem(&input), (p, Problem::Invalid(_)) if p == "server.endpoints"),
                "{bad}"
            );
        }
        let dup = with("\"[2001:db8::10]:51820\"", "\"203.0.113.10:51820\"");
        assert!(matches!(problem(&dup).1, Problem::Invalid(_)));
        let none = with(
            "endpoints = [\"203.0.113.10:51820\", \"[2001:db8::10]:51820\"]",
            "endpoints = []",
        );
        assert!(matches!(problem(&none).1, Problem::Invalid(_)));
    }

    #[test]
    fn endpoint_inside_tunnel_rule_is_a_conflict() {
        let input = with("direct = [", "tunnel = [\"203.0.113.0/24\"]\ndirect = [");
        assert!(matches!(problem(&input), (p, Problem::Conflict(_)) if p == "server.endpoints"));
    }

    #[test]
    fn routing_errors() {
        assert!(matches!(
            problem(&with("\"full-tunnel\"", "\"everything\"")).1,
            Problem::Invalid(_)
        ));
        assert!(matches!(
            problem(&with("\"192.168.1.30/32\"", "\"192.168.1.30/24\"")).1,
            Problem::Invalid(_)
        ));
        // A direct default route would switch protection off.
        assert!(matches!(
            problem(&with("\"192.168.1.30/32\"", "\"0.0.0.0/0\"")).1,
            Problem::Conflict(_)
        ));
        assert!(matches!(
            problem(&with("\"192.168.1.30/32\", ", "\"2001:db8:1::30/128\", ")).1,
            Problem::Conflict(_)
        ));
    }

    #[test]
    fn dns_rules_prevent_leaks() {
        // System DNS in full-tunnel mode would leak.
        assert!(matches!(
            problem(&with("[dns]\n", "[dns]\nmode = \"system\"\n")),
            (p, Problem::Conflict(_)) if p == "dns.mode"
        ));
        // A resolver routed directly would leak.
        assert!(matches!(
            problem(&with("\"10.77.0.1\"", "\"192.168.1.30\"")),
            (p, Problem::Conflict(_)) if p == "dns.resolvers"
        ));
        // A LAN exception without a direct rule.
        assert!(matches!(
            problem(&with("lan_exceptions = [\"192.168.1.30\"]", "lan_exceptions = [\"192.168.1.1\"]")),
            (p, Problem::Conflict(_)) if p == "dns.lan_exceptions"
        ));
        // Loopback resolvers are local, not tunnel resolvers.
        assert!(matches!(
            problem(&with("\"10.77.0.1\"", "\"127.0.0.53\"")).1,
            Problem::Invalid(_)
        ));
        let five =
            "resolvers = [\"10.0.0.1\", \"10.0.0.2\", \"10.0.0.3\", \"10.0.0.4\", \"10.0.0.5\"]";
        assert!(matches!(
            problem(&with("resolvers = [\"10.77.0.1\", \"fd77::1\"]", five)).1,
            Problem::Invalid(_)
        ));
        assert!(matches!(
            problem(&with("[dns]\n", "[dns]\nmode = \"doh\"\n")).1,
            Problem::Invalid(_)
        ));
    }

    #[test]
    fn split_mode_resolver_must_be_tunnelled() {
        let input = r#"version = 1
[server]
endpoints = ["198.51.100.7:4000"]
[routing]
mode = "split"
tunnel = ["192.168.50.0/24"]
[dns]
resolvers = ["1.1.1.1"]
"#;
        assert!(matches!(problem(input).1, Problem::Conflict(_)));
    }

    #[test]
    fn tunnel_settings_are_bounded() {
        let base = format!("{FULL}[tunnel]\n");
        for (extra, expect_range) in [
            ("mtu = 1279\n", true),
            ("mtu = 9001\n", true),
            ("interface = \"this-name-is-too-long\"\n", false),
            ("interface = \"bad name\"\n", false),
            ("interface = \"..\"\n", false),
            ("interface = \"septuvo0\\\" accept; #\"\n", false),
            ("interface = \"tun0;\"\n", false),
        ] {
            let p = problem(&format!("{base}{extra}")).1;
            if expect_range {
                assert_eq!(
                    p,
                    Problem::OutOfRange {
                        min: 1280,
                        max: 9000
                    },
                    "{extra}"
                );
            } else {
                assert!(matches!(p, Problem::Invalid(_)), "{extra}");
            }
        }
        let p = problem(&format!("{FULL}[logging]\nretention_days = 366\n")).1;
        assert_eq!(p, Problem::OutOfRange { min: 0, max: 365 });
        let p = problem(&format!("{FULL}[connection]\nname = 1\n")).1;
        assert!(matches!(p, Problem::WrongType { .. }));
    }

    #[test]
    fn errors_carry_locations() {
        let e = parse_device(&with("\"10.77.0.1\"", "\"10.77.0.999\"")).expect_err("invalid");
        let span = e.span.expect("location");
        assert_eq!(span.line, 12);
        assert!(e.to_string().starts_with("line 12, column "));
        assert!(e.to_string().contains("dns.resolvers"));
    }

    /// Regression (N3-03 fuzzing, case seed 0xc8c5a99ba1125cd2): an interface
    /// name starting with '-' was accepted here but refused by the renderer.
    #[test]
    fn interface_names_follow_the_renderer_exactly() {
        let with = |name: &str| {
            parse_device(&format!(
                "version = 1\n[server]\nendpoints = [\"203.0.113.7:51820\"]\n\
                 [routing]\nmode = \"full-tunnel\"\n[dns]\nresolvers = [\"10.77.0.1\"]\n\
                 [tunnel]\ninterface = \"{name}\"\n"
            ))
        };
        for bad in ["-1septuvo0", "-", ".", "..", "a234567890123456", "tun 0"] {
            assert!(with(bad).is_err(), "{bad:?}");
            assert!(!septuvo_policy::nft::valid_interface(bad));
        }
        for good in ["septuvo0", "wg_test-1.2", "a-"] {
            assert!(with(good).is_ok(), "{good:?}");
        }
    }
}
