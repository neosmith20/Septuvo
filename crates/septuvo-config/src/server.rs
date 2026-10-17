//! Septuvo Server configuration (`septuvo-server.toml`).
//!
//! Before the G1 review the server opens no network port: the listen
//! address is stored and checked, never bound. Tunnel address ranges must be
//! private (owner decision Q13) and are used by device administration (N2-04).

use crate::error::ConfigError;
use crate::schema::{self, Section};
use crate::toml::{self, Document};
use septuvo_policy::Prefix;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Component, Path, PathBuf};

/// Default data directory.
pub const DEFAULT_DATA_DIR: &str = "/var/lib/septuvo-server";
/// Longest data directory path accepted, in bytes.
pub const MAX_PATH_BYTES: usize = 255;

/// A validated server configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    /// Address and UDP port the server will listen on once the protocol
    /// exists. **Nothing is opened before G1.**
    pub listen: SocketAddr,
    /// Private IPv4 range for device tunnel addresses.
    pub ipv4_range: Prefix,
    /// Unique-local IPv6 range for device tunnel addresses.
    pub ipv6_range: Prefix,
    /// Where the server keeps its records.
    pub data_dir: PathBuf,
    /// Log retention in days (Q10).
    pub log_retention_days: u16,
}

const TABLES: &[&str] = &["listen", "network", "storage", "logging"];

/// Parses and validates a server configuration.
///
/// # Errors
/// The first problem found, with its location.
pub fn parse_server(input: &str) -> Result<ServerConfig, ConfigError> {
    server_from_document(&toml::parse(input)?)
}

/// Reads, parses and validates a server configuration file.
///
/// # Errors
/// A read error, or the first problem found in the file.
pub fn load_server(path: &Path) -> Result<ServerConfig, ConfigError> {
    let bytes = schema::read_bounded(path)?;
    server_from_document(&toml::parse_bytes(&bytes)?)
}

/// Validates a parsed document as a server configuration.
///
/// # Errors
/// The first problem found, with its location.
pub fn server_from_document(doc: &Document) -> Result<ServerConfig, ConfigError> {
    schema::check_document(doc, TABLES, &[])?;
    let listen = listen(doc)?;
    let (ipv4_range, ipv6_range) = network(doc)?;
    let data_dir = data_dir(doc)?;
    Ok(ServerConfig {
        listen,
        ipv4_range,
        ipv6_range,
        data_dir,
        log_retention_days: schema::retention_days(doc)?,
    })
}

fn listen(doc: &Document) -> Result<SocketAddr, ConfigError> {
    let s = schema::required_table(doc, "listen")?;
    s.only(&["address", "port"])?;
    let address = match s.string("address")? {
        None => IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        Some((text, span)) => {
            let addr: IpAddr = text.parse().map_err(|_| {
                s.invalid("address", span, format!("'{text}' is not an IP address"))
            })?;
            if addr.is_multicast() {
                return Err(s.invalid("address", span, "a multicast address cannot be used"));
            }
            addr
        }
    };
    let (port, _) = s.int("port", 1, 65535)?.ok_or_else(|| s.missing("port"))?;
    let port = u16::try_from(port).unwrap_or(0);
    Ok(SocketAddr::new(address, port))
}

fn private_v4() -> [Prefix; 3] {
    [
        Prefix::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 8),
        Prefix::new(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 0)), 12),
        Prefix::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 0)), 16),
    ]
    .map(|p| p.expect("constant prefixes are canonical"))
}

fn unique_local_v6() -> Prefix {
    Prefix::new(IpAddr::V6(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 0)), 7)
        .expect("constant prefix is canonical")
}

fn range(
    s: Section<'_>,
    key: &str,
    allowed: &[Prefix],
    lengths: (u8, u8),
    what: &str,
) -> Result<Prefix, ConfigError> {
    let (text, span) = s.string(key)?.ok_or_else(|| s.missing(key))?;
    let p = schema::prefix(text).map_err(|why| s.invalid(key, span, why))?;
    if !allowed.iter().any(|a| schema::within(p, *a)) {
        return Err(s.invalid(key, span, format!("'{text}' must lie inside {what}")));
    }
    if p.prefix_len() < lengths.0 || p.prefix_len() > lengths.1 {
        return Err(s.invalid(
            key,
            span,
            format!(
                "prefix length must be between /{} and /{}",
                lengths.0, lengths.1
            ),
        ));
    }
    Ok(p)
}

fn network(doc: &Document) -> Result<(Prefix, Prefix), ConfigError> {
    let s = schema::required_table(doc, "network")?;
    s.only(&["ipv4_range", "ipv6_range"])?;
    let v4 = range(
        s,
        "ipv4_range",
        &private_v4(),
        (16, 30),
        "a private range (10.0.0.0/8, 172.16.0.0/12 or 192.168.0.0/16)",
    )?;
    let v6 = range(
        s,
        "ipv6_range",
        &[unique_local_v6()],
        (48, 120),
        "the unique-local range fc00::/7",
    )?;
    Ok((v4, v6))
}

fn data_dir(doc: &Document) -> Result<PathBuf, ConfigError> {
    let Some(table) = doc.table("storage") else {
        return Ok(PathBuf::from(DEFAULT_DATA_DIR));
    };
    let s = Section { table };
    s.only(&["data_dir"])?;
    let Some((text, span)) = s.string("data_dir")? else {
        return Ok(PathBuf::from(DEFAULT_DATA_DIR));
    };
    let path = Path::new(text);
    // Checked on the text, because `Path::components` silently drops "."
    // and repeated slashes.
    let plain = text.len() <= MAX_PATH_BYTES
        && !text.chars().any(char::is_control)
        && text.strip_prefix('/').is_some_and(|rest| {
            rest.split('/')
                .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
        })
        && path
            .components()
            .all(|c| matches!(c, Component::RootDir | Component::Normal(_)));
    if !plain {
        return Err(s.invalid(
            "data_dir",
            span,
            format!(
                "must be an absolute path below '/', without '.' or '..', at most {MAX_PATH_BYTES} bytes"
            ),
        ));
    }
    Ok(path.to_path_buf())
}

impl ServerConfig {
    /// A short human-readable summary, for `septuvo-config check`.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "listen:           {} (stored only: no port is opened before the G1 review)\n\
             tunnel IPv4:      {}\ntunnel IPv6:      {}\ndata directory:   {}\n\
             log retention:    {} day(s)",
            self.listen,
            self.ipv4_range,
            self.ipv6_range,
            self.data_dir.display(),
            self.log_retention_days,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Problem;

    const BASE: &str = r#"version = 1

[listen]
port = 51820

[network]
ipv4_range = "10.77.0.0/24"
ipv6_range = "fd77:77:77::/64"
"#;

    fn problem(input: &str) -> (String, Problem) {
        let e = parse_server(input).expect_err("configuration should be rejected");
        (e.path, e.problem)
    }

    fn with(replace: &str, by: &str) -> String {
        assert!(BASE.contains(replace), "fixture text {replace:?} not found");
        BASE.replace(replace, by)
    }

    #[test]
    fn minimal_config_uses_defaults() {
        let c = parse_server(BASE).expect("valid");
        assert_eq!(c.listen.to_string(), "[::]:51820");
        assert_eq!(c.data_dir, PathBuf::from(DEFAULT_DATA_DIR));
        assert_eq!(c.log_retention_days, 7);
        assert!(c.summary().contains("no port is opened"));
    }

    #[test]
    fn full_config() {
        let input = format!(
            "{}\n[storage]\ndata_dir = \"/srv/septuvo\"\n[logging]\nretention_days = 30\n",
            with("port = 51820", "address = \"192.0.2.1\"\nport = 4000")
        );
        let c = parse_server(&input).expect("valid");
        assert_eq!(c.listen.to_string(), "192.0.2.1:4000");
        assert_eq!(c.data_dir, PathBuf::from("/srv/septuvo"));
        assert_eq!(c.log_retention_days, 30);
    }

    #[test]
    fn port_is_required_and_bounded() {
        assert_eq!(
            problem(&with("port = 51820", "")),
            ("listen.port".into(), Problem::Missing)
        );
        assert_eq!(
            problem(&with("51820", "0")).1,
            Problem::OutOfRange { min: 1, max: 65535 }
        );
        assert_eq!(
            problem(&with("51820", "65536")).1,
            Problem::OutOfRange { min: 1, max: 65535 }
        );
    }

    #[test]
    fn listen_address_checked() {
        for bad in ["example.com", "224.0.0.1", "10.0.0.0/8"] {
            let input = with(
                "port = 51820",
                &format!("address = \"{bad}\"\nport = 51820"),
            );
            assert!(matches!(problem(&input).1, Problem::Invalid(_)), "{bad}");
        }
    }

    #[test]
    fn ranges_must_be_private_and_sized() {
        for bad in [
            "8.8.8.0/24",
            "100.64.0.0/24",
            "10.0.0.0/8",
            "10.77.0.0/31",
            "10.77.0.1/24",
            "fd77::",
        ] {
            assert!(
                matches!(problem(&with("10.77.0.0/24", bad)), (p, Problem::Invalid(_)) if p == "network.ipv4_range"),
                "{bad}"
            );
        }
        for bad in ["2001:db8::/64", "fd77::/32", "fd77::/121", "10.77.0.0/24"] {
            assert!(
                matches!(problem(&with("fd77:77:77::/64", bad)), (p, Problem::Invalid(_)) if p == "network.ipv6_range"),
                "{bad}"
            );
        }
        assert_eq!(
            problem(&with("ipv6_range = \"fd77:77:77::/64\"\n", "")),
            ("network.ipv6_range".into(), Problem::Missing)
        );
        // 172.16.0.0/12 and 192.168.0.0/16 are private too.
        for good in ["172.20.0.0/16", "192.168.77.0/24"] {
            assert!(parse_server(&with("10.77.0.0/24", good)).is_ok(), "{good}");
        }
    }

    #[test]
    fn data_dir_must_be_plain_absolute() {
        for bad in [
            "relative/dir",
            "/",
            "/var/../etc",
            "/var/./lib",
            "/var//lib",
            "/var/lib/",
            "/var/lib\\u0007",
        ] {
            let input = format!("{BASE}[storage]\ndata_dir = \"{bad}\"\n");
            assert!(matches!(problem(&input).1, Problem::Invalid(_)), "{bad:?}");
        }
    }

    #[test]
    fn unknown_tables_rejected() {
        let (path, p) = problem(&format!("{BASE}[devices]\nlaptop = \"x\"\n"));
        assert_eq!((path.as_str(), p), ("devices", Problem::UnknownTable));
    }
}
