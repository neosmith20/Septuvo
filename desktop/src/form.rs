//! The **VPN Configuration** editor's form (DEFAULT-UI §4.2; N4-05).
//!
//! Plain Rust, no drawing. The form holds what the user typed, as text, so
//! nothing is lost while it is invalid. [`Form::to_text`] writes a device
//! configuration (`septuvo.toml`, with a `[connection]` name table) and
//! [`Form::check`] validates it with the **same parser the service uses**
//! (`septuvo-config`), so the app never accepts what the service would
//! refuse. The service validates again when it receives it (N4-D11).
//!
//! **DNS is not edited here** (D-039): the editor shows which DNS settings
//! apply and keeps an imported or saved connection's `[dns]` table exactly
//! as it was. A new configuration has no DNS settings until Settings → DNS
//! Protection exists (N4-07), so before then it can be saved only as a
//! draft.

use septuvo_config::toml::{self, Document, Value};
use septuvo_config::{DeviceConfig, connection_name, parse_device, quote};
use std::fmt::Write as _;

/// Largest configuration text (the protocol's body limit).
pub const MAX_TEXT_BYTES: usize = septuvo_ipc::MAX_BODY_BYTES;

/// A field of the form, for placing an error next to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Connection Name.
    Name,
    /// Server Address.
    Server,
    /// Routing Mode.
    Mode,
    /// Networks Through VPN.
    Tunnel,
    /// Direct Network Access.
    Direct,
    /// Blocked Networks.
    Block,
    /// DNS (read-only here).
    Dns,
    /// Interface Name.
    Interface,
    /// MTU.
    Mtu,
    /// Log Retention (Days).
    Retention,
    /// Anything else in the file.
    Other,
}

impl Field {
    /// The field's label as shown (DEFAULT-UI §4.2, exact).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Field::Name => "Connection Name",
            Field::Server => "Server Address",
            Field::Mode => "Routing Mode",
            Field::Tunnel => "Networks Through VPN",
            Field::Direct => "Direct Network Access",
            Field::Block => "Blocked Networks",
            Field::Dns => "DNS",
            Field::Interface => "Interface Name",
            Field::Mtu => "MTU",
            Field::Retention => "Log Retention (Days)",
            Field::Other => "Configuration",
        }
    }

    /// True for the fields inside **Advanced Configuration**.
    #[must_use]
    pub fn advanced(self) -> bool {
        !matches!(self, Field::Name | Field::Server | Field::Other)
    }
}

/// A connection's DNS settings, kept unchanged (D-039).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DnsKept {
    /// `dns.resolvers`, as written.
    pub resolvers: Vec<String>,
    /// `dns.lan_exceptions`, as written.
    pub lan_exceptions: Vec<String>,
}

/// What the user is editing. Every field is text, exactly as typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    /// Connection Name.
    pub name: String,
    /// Server Address (the first endpoint).
    pub server: String,
    /// Further endpoints of an imported or saved configuration, kept.
    pub more_servers: Vec<String>,
    /// Full Tunnel (true) or Split Tunnel.
    pub full_tunnel: bool,
    /// Networks Through VPN: prefixes, one per line or comma separated.
    pub tunnel: String,
    /// Direct Network Access.
    pub direct: String,
    /// Blocked Networks.
    pub block: String,
    /// The connection's DNS settings, if it has any (never edited here).
    pub dns: Option<DnsKept>,
    /// Interface Name; empty means the default `septuvo0`.
    pub interface: String,
    /// MTU; empty means the default 1420.
    pub mtu: String,
    /// Log Retention (Days); empty means the default 7.
    pub retention: String,
}

impl Default for Form {
    fn default() -> Self {
        Form {
            name: String::new(),
            server: String::new(),
            more_servers: Vec::new(),
            full_tunnel: true,
            tunnel: String::new(),
            direct: String::new(),
            block: String::new(),
            dns: None,
            interface: String::new(),
            mtu: String::new(),
            retention: String::new(),
        }
    }
}

/// The result of checking a form.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Checked {
    /// Problems, each next to its field (the parser reports the first one
    /// it finds in the file; the name is always checked).
    pub errors: Vec<(Field, String)>,
    /// The name is valid and unique: the form can be saved, at least as a
    /// draft.
    pub name_ok: bool,
    /// The configuration is complete and valid: it can be saved to the
    /// service. Its text is [`Form::to_text`].
    pub complete: bool,
}

impl Checked {
    /// The error shown next to `field`, if any.
    #[must_use]
    pub fn error(&self, field: Field) -> Option<&str> {
        self.errors
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, e)| e.as_str())
    }
}

/// The message for a configuration without DNS settings.
pub const NO_DNS: &str = "This connection has no DNS settings yet. DNS is set in Settings → \
DNS Protection, which is not available yet in this development build: save it as a draft for now.";

/// Items of a list field: split on commas, spaces and new lines.
#[must_use]
pub fn items(text: &str) -> Vec<String> {
    text.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn array(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| quote(s)).collect();
    format!("[{}]", quoted.join(", "))
}

fn strings(doc: &Document, table: &str, key: &str) -> Vec<String> {
    match doc.table(table).and_then(|t| t.get(key)).map(|e| &e.value) {
        Some(Value::Array(a)) => a.iter().map(|(s, _)| s.clone()).collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn scalar(doc: &Document, table: &str, key: &str) -> String {
    match doc.table(table).and_then(|t| t.get(key)).map(|e| &e.value) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Integer(n)) => n.to_string(),
        Some(Value::Boolean(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// Which field a parser error belongs to (`ConfigError::path`).
fn field_of(path: &str) -> Field {
    match path {
        "server.endpoints" | "server" | "server.identity" => Field::Server,
        "routing.mode" | "routing" => Field::Mode,
        "routing.tunnel" => Field::Tunnel,
        "routing.direct" => Field::Direct,
        "routing.block" => Field::Block,
        "tunnel.interface" => Field::Interface,
        "tunnel.mtu" => Field::Mtu,
        "logging.retention_days" => Field::Retention,
        "connection.name" | "connection" => Field::Name,
        p if p == "dns" || p.starts_with("dns.") => Field::Dns,
        _ => Field::Other,
    }
}

impl Form {
    /// The form for a configuration text (a saved connection, an import or
    /// a draft). Lenient: whatever is readable is filled in; an unreadable
    /// file gives an empty form. Nothing here is trusted: the text is
    /// checked again by [`Form::check`] before it is used.
    #[must_use]
    pub fn from_text(text: &str) -> Form {
        let Ok(doc) = toml::parse(text) else {
            return Form::default();
        };
        let endpoints = strings(&doc, "server", "endpoints");
        let mut it = endpoints.into_iter();
        let server = it.next().unwrap_or_default();
        let dns = doc.table("dns").map(|_| DnsKept {
            resolvers: strings(&doc, "dns", "resolvers"),
            lan_exceptions: strings(&doc, "dns", "lan_exceptions"),
        });
        Form {
            name: scalar(&doc, "connection", "name"),
            server,
            more_servers: it.collect(),
            full_tunnel: scalar(&doc, "routing", "mode") != "split",
            tunnel: strings(&doc, "routing", "tunnel").join("\n"),
            direct: strings(&doc, "routing", "direct").join("\n"),
            block: strings(&doc, "routing", "block").join("\n"),
            dns,
            interface: scalar(&doc, "tunnel", "interface"),
            mtu: scalar(&doc, "tunnel", "mtu"),
            retention: scalar(&doc, "logging", "retention_days"),
        }
    }

    /// The device configuration text, with the `[connection]` name table.
    /// Numbers that are not numbers are written as text, so a draft keeps
    /// exactly what was typed and the parser says what is wrong.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut s = String::from("version = 1\n");
        let mut endpoints: Vec<String> = Vec::new();
        if !self.server.trim().is_empty() {
            endpoints.push(self.server.trim().to_string());
        }
        endpoints.extend(self.more_servers.iter().cloned());
        let _ = writeln!(s, "\n[server]\nendpoints = {}", array(&endpoints));
        let mode = if self.full_tunnel {
            "full-tunnel"
        } else {
            "split"
        };
        let _ = writeln!(s, "\n[routing]\nmode = {}", quote(mode));
        for (key, text) in [
            ("tunnel", &self.tunnel),
            ("direct", &self.direct),
            ("block", &self.block),
        ] {
            let list = items(text);
            if !list.is_empty() {
                let _ = writeln!(s, "{key} = {}", array(&list));
            }
        }
        if let Some(dns) = &self.dns {
            let _ = writeln!(s, "\n[dns]\nresolvers = {}", array(&dns.resolvers));
            if !dns.lan_exceptions.is_empty() {
                let _ = writeln!(s, "lan_exceptions = {}", array(&dns.lan_exceptions));
            }
        }
        let number = |text: &str| {
            let t = text.trim();
            if !t.is_empty() && t.len() <= 9 && t.bytes().all(|b| b.is_ascii_digit()) {
                t.to_string()
            } else {
                quote(t)
            }
        };
        if !self.interface.trim().is_empty() || !self.mtu.trim().is_empty() {
            s.push_str("\n[tunnel]\n");
            if !self.interface.trim().is_empty() {
                let _ = writeln!(s, "interface = {}", quote(self.interface.trim()));
            }
            if !self.mtu.trim().is_empty() {
                let _ = writeln!(s, "mtu = {}", number(&self.mtu));
            }
        }
        if !self.retention.trim().is_empty() {
            let _ = writeln!(
                s,
                "\n[logging]\nretention_days = {}",
                number(&self.retention)
            );
        }
        let _ = writeln!(s, "\n[connection]\nname = {}", quote(self.name.trim()));
        s
    }

    /// Checks the form. `taken` says whether a (trimmed) name is already
    /// used by another connection or draft, regardless of case.
    #[must_use]
    pub fn check(&self, taken: impl Fn(&str) -> bool) -> Checked {
        let mut out = Checked::default();
        match connection_name(&self.name) {
            Err(why) => out
                .errors
                .push((Field::Name, format!("Enter a connection name: {why}."))),
            Ok(name) if taken(&name) => out.errors.push((
                Field::Name,
                "Another connection already has this name. Choose a different name.".into(),
            )),
            Ok(_) => out.name_ok = true,
        }
        let text = self.to_text();
        if text.len() > MAX_TEXT_BYTES {
            out.errors
                .push((Field::Other, "The configuration is too large.".into()));
            return out;
        }
        if self.server.trim().is_empty() {
            out.errors
                .push((Field::Server, "Enter a server address and port.".into()));
            return out;
        }
        if self.dns.is_none() {
            // Checked first so the user sees why it cannot be complete.
            out.errors.push((Field::Dns, NO_DNS.into()));
        }
        match parse_device(&text) {
            Ok(_) => out.complete = out.name_ok,
            Err(e) => {
                let field = field_of(&e.path);
                if field == Field::Dns && self.dns.is_none() {
                    return out;
                }
                if out.error(field).is_none() {
                    out.errors.push((field, e.problem.to_string()));
                }
            }
        }
        out
    }

    /// The validated configuration, if the form is complete.
    #[must_use]
    pub fn config(&self) -> Option<DeviceConfig> {
        parse_device(&self.to_text()).ok()
    }

    /// A short, read-only description of the DNS settings that apply.
    #[must_use]
    pub fn dns_summary(&self) -> String {
        match &self.dns {
            None => "None yet. DNS is set in Settings → DNS Protection.".into(),
            Some(d) => format!(
                "Custom DNS for This Connection: {}. Changed only in Settings → DNS Protection.",
                d.resolvers.join(", ")
            ),
        }
    }

    /// The form for **Duplicate**: the same non-secret settings under a new
    /// unique name `Copy of <Name>` (with a number if needed). Nothing
    /// secret exists in this format to leave out (no keys before G1).
    #[must_use]
    pub fn duplicate(&self, taken: impl Fn(&str) -> bool) -> Form {
        let base = format!("Copy of {}", self.name.trim());
        let mut name = truncate(&base);
        let mut n = 2;
        while taken(&name) && n < 1000 {
            let suffix = format!(" ({n})");
            name = format!(
                "{}{suffix}",
                truncate_to(&base, 64 - suffix.chars().count())
            );
            n += 1;
        }
        Form {
            name,
            ..self.clone()
        }
    }
}

fn truncate_to(s: &str, chars: usize) -> String {
    s.chars().take(chars).collect()
}

fn truncate(s: &str) -> String {
    truncate_to(s, septuvo_config::device::MAX_NAME_CHARS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = "version = 1\n[server]\nendpoints = [\"203.0.113.10:51820\"]\n\
        [routing]\nmode = \"split\"\ntunnel = [\"10.0.0.0/8\"]\n[dns]\nresolvers = [\"10.77.0.1\"]\n\
        [tunnel]\nmtu = 1400\n[connection]\nname = \"Home VPN\"\n";

    fn free(_: &str) -> bool {
        false
    }

    #[test]
    fn a_saved_configuration_round_trips_through_the_form() {
        let f = Form::from_text(FULL);
        assert_eq!(f.name, "Home VPN");
        assert_eq!(f.server, "203.0.113.10:51820");
        assert!(!f.full_tunnel);
        assert_eq!(f.tunnel, "10.0.0.0/8");
        assert_eq!(f.mtu, "1400");
        let c = f.check(free);
        assert_eq!(c.errors, vec![]);
        assert!(c.complete && c.name_ok);
        let again = Form::from_text(&f.to_text());
        assert_eq!(again, f);
        assert_eq!(
            parse_device(&f.to_text()).unwrap(),
            parse_device(FULL).unwrap()
        );
    }

    #[test]
    fn a_new_blank_form_is_a_draft_only() {
        let mut f = Form::default();
        let c = f.check(free);
        assert!(!c.name_ok && !c.complete);
        assert!(c.error(Field::Name).is_some());
        f.name = "Office".into();
        f.server = "203.0.113.7:51820".into();
        let c = f.check(free);
        assert!(c.name_ok, "a named form can be saved as a draft");
        assert!(!c.complete, "no DNS yet: never complete");
        assert_eq!(c.error(Field::Dns), Some(NO_DNS));
    }

    #[test]
    fn errors_land_next_to_their_fields() {
        for (change, field) in [
            (
                ("203.0.113.10:51820", "vpn.example.com:51820"),
                Field::Server,
            ),
            (("10.0.0.0/8", "10.0.0.0/33"), Field::Tunnel),
            (("mtu = 1400", "mtu = 99999"), Field::Mtu),
        ] {
            let f = Form::from_text(&FULL.replace(change.0, change.1));
            let c = f.check(free);
            assert!(!c.complete, "{change:?}");
            assert!(c.error(field).is_some(), "{change:?}: {:?}", c.errors);
        }
        let mut f = Form::from_text(FULL);
        f.mtu = "abc".into();
        assert!(f.check(free).error(Field::Mtu).is_some());
        assert_eq!(Form::from_text(&f.to_text()).mtu, "abc", "kept as typed");
    }

    #[test]
    fn names_are_required_bounded_and_unique() {
        let mut f = Form::from_text(FULL);
        let c = f.check(|n| n.eq_ignore_ascii_case("home vpn"));
        assert!(!c.name_ok && !c.complete);
        f.name = "x".repeat(65);
        assert!(!f.check(free).name_ok);
        f.name = "  Büro  ".into();
        assert!(f.check(free).name_ok);
    }

    #[test]
    fn duplicate_suggests_a_unique_copy_name() {
        let f = Form::from_text(FULL);
        assert_eq!(f.duplicate(free).name, "Copy of Home VPN");
        let d = f.duplicate(|n| n == "Copy of Home VPN" || n == "Copy of Home VPN (2)");
        assert_eq!(d.name, "Copy of Home VPN (3)");
        assert_eq!(d.server, f.server);
        let long = Form {
            name: "n".repeat(64),
            ..f.clone()
        };
        assert!(long.duplicate(free).name.chars().count() <= 64);
    }

    #[test]
    fn hostile_text_never_escapes_its_field() {
        let mut f = Form::from_text(FULL);
        f.name = "a\"]\n[server]\nendpoints = [\"1.2.3.4:1\"]".into();
        let text = f.to_text();
        let back = Form::from_text(&text);
        assert_eq!(back.server, "203.0.113.10:51820");
        f.tunnel = "10.0.0.0/8\"]\nblock = [\"0.0.0.0/0".into();
        assert!(!f.check(free).complete);
        assert_eq!(Form::from_text("not toml ["), Form::default());
    }
}
