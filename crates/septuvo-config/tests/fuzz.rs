//! Fuzz targets for `septuvo-config` (N3-03): the TOML subset, the device
//! schema (`septuvo.toml`) and the server schema (`septuvo-server.toml`).
//! Longer runs: `SEPTUVO_FUZZ_SECONDS=60 cargo test -p septuvo-config --test fuzz -- --nocapture`.

// The check functions are public so the cargo-fuzz targets (fuzz/) can call
// them; they panic on purpose when a property fails.
#![allow(clippy::must_use_candidate, clippy::missing_panics_doc)]

#[path = "../../septuvo-policy/tests/support/fuzzer.rs"]
mod fuzzer;

use fuzzer::{Corpus, Rng, ip, ip_text, prefix_text, run, word};
use septuvo_config::toml::{self, Document, MAX_LINE_CHARS, Value};
use septuvo_config::{DnsPolicy, MAX_RETENTION_DAYS, parse_device, parse_server};
use septuvo_policy::nft::{self, DnsRules, KillSwitch};
use std::fmt::Write as _;

const DEVICE_FULL: &str = include_str!("../examples/device-full-tunnel.toml");
const DEVICE_SPLIT: &str = include_str!("../examples/device-split.toml");
const SERVER: &str = include_str!("../examples/server.toml");

const DICT: &[&str] = &[
    "version",
    "=",
    "1",
    "[",
    "]",
    "\"",
    "'",
    "#",
    ",",
    "\\n",
    "\\u0000",
    "\\U0010FFFF",
    "\\x",
    "true",
    "false",
    "[server]",
    "[routing]",
    "[dns]",
    "[tunnel]",
    "[behavior]",
    "[logging]",
    "[listen]",
    "[network]",
    "[storage]",
    "endpoints",
    "mode",
    "full-tunnel",
    "split",
    "direct",
    "tunnel",
    "block",
    "resolvers",
    "lan_exceptions",
    "system",
    "interface",
    "mtu",
    "always_on",
    "[connection]",
    "name",
    "retention_days",
    "address",
    "port",
    "ipv4_range",
    "ipv6_range",
    "data_dir",
    "-1",
    "9223372036854775807",
    "9223372036854775808",
    "\u{feff}",
    "\r\n",
    "\t",
    "\u{7f}",
];

/// Values without their positions, so documents can be compared after a
/// round trip through [`write_toml`].
fn shape(doc: &Document) -> Vec<(String, Vec<(String, String)>)> {
    let table = |t: &toml::Table| {
        t.entries
            .iter()
            .map(|e| {
                let v = match &e.value {
                    Value::Array(items) => {
                        let items: Vec<&str> = items.iter().map(|(s, _)| s.as_str()).collect();
                        format!("{items:?}")
                    }
                    other => format!("{other:?}"),
                };
                (e.key.clone(), v)
            })
            .collect()
    };
    std::iter::once((String::new(), table(&doc.root)))
        .chain(doc.tables.iter().map(|t| (t.name.clone(), table(t))))
        .collect()
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\t' => out.push('\t'),
            c if c.is_control() || c == '\u{7f}' => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A minimal writer for the subset (test code), one item per line.
fn write_toml(doc: &Document) -> String {
    let mut s = String::new();
    let table = |t: &toml::Table, s: &mut String| {
        for e in &t.entries {
            let v = match &e.value {
                Value::String(v) => quote(v),
                Value::Integer(i) => i.to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::Array(items) => {
                    let mut a = String::from("[\n");
                    for (item, _) in items {
                        let _ = writeln!(a, "  {},", quote(item));
                    }
                    a.push(']');
                    a
                }
            };
            let _ = writeln!(s, "{} = {v}", e.key);
        }
    };
    table(&doc.root, &mut s);
    for t in &doc.tables {
        let _ = writeln!(s, "[{}]", t.name);
        table(t, &mut s);
    }
    s
}

fn structured_toml(rng: &mut Rng) -> Vec<u8> {
    const BARE: &[u8] = b"abcdefghijklmnopqrstuvwxyz_-0123456789";
    let mut s = String::from("version = 1\n");
    for _ in 0..rng.below(6) {
        let _ = writeln!(s, "[{}]", word(rng, BARE, 10));
        for _ in 0..rng.below(6) {
            let key = word(rng, BARE, 12);
            let value = match rng.below(5) {
                0 => format!("\"{}\"", word(rng, b"abc 0123./:[]\\\"'#", 20)),
                1 => format!("'{}'", word(rng, b"abc \\\"#", 20)),
                2 => rng.interesting_number().to_string(),
                3 => (*rng.pick(&["true", "false", "True", "yes"])).to_string(),
                _ => {
                    let items: Vec<String> = (0..rng.below(5))
                        .map(|_| format!("\"{}\"", ip_text(rng)))
                        .collect();
                    format!("[{}]", items.join(", "))
                }
            };
            let _ = writeln!(s, "{key} = {value}");
        }
    }
    s.into_bytes()
}

#[test]
fn fuzz_toml_subset() {
    let corpus = Corpus {
        seeds: &[
            DEVICE_FULL,
            DEVICE_SPLIT,
            SERVER,
            "a = 'x'\nb = [\"1\", # c\n \"2\",\n]\n",
        ],
        dict: DICT,
    };
    run(
        "fuzz_toml_subset",
        &corpus,
        structured_toml,
        check_toml_subset,
    );
}

/// The kill switch the service would build from a device configuration.
fn kill_switch(c: &septuvo_config::DeviceConfig) -> KillSwitch {
    KillSwitch {
        tunnel_interface: c.tunnel.interface.clone(),
        endpoints: c.endpoints.iter().map(|e| (e.ip(), e.port())).collect(),
        dns: match &c.dns {
            DnsPolicy::Tunnel {
                resolvers,
                lan_exceptions,
            } => DnsRules::Resolvers {
                resolvers: resolvers.clone(),
                lan_exceptions: lan_exceptions.clone(),
            },
        },
        captive_helper: None,
    }
}

/// Usually an ordinary unicast address; sometimes a special one.
fn unicast(rng: &mut Rng) -> String {
    loop {
        let a = ip(rng);
        if rng.chance(10) || !(a.is_loopback() || a.is_unspecified() || a.is_multicast()) {
            return a.to_string();
        }
    }
}

fn structured_device(rng: &mut Rng) -> Vec<u8> {
    let mut s = String::from("version = 1\n[server]\n");
    let count = if rng.chance(90) {
        1 + rng.below(3)
    } else {
        rng.below(20)
    };
    let eps: Vec<String> = (0..count)
        .map(|_| {
            let ip = if rng.chance(85) {
                unicast(rng)
            } else {
                ip_text(rng)
            };
            let port = if rng.chance(85) {
                1 + rng.below(65_535)
            } else {
                rng.below(70_000)
            };
            if ip.contains(':') {
                format!("\"[{ip}]:{port}\"")
            } else {
                format!("\"{ip}:{port}\"")
            }
        })
        .collect();
    let _ = writeln!(s, "endpoints = [{}]", eps.join(", "));
    let mode = if rng.chance(90) {
        *rng.pick(&["full-tunnel", "split"])
    } else {
        *rng.pick(&["other", "Split", ""])
    };
    let _ = writeln!(s, "[routing]\nmode = \"{mode}\"");
    for key in ["direct", "tunnel", "block"] {
        if rng.chance(if key == "tunnel" { 90 } else { 50 }) {
            let items: Vec<String> = (0..rng.below(4))
                .map(|_| format!("\"{}\"", prefix_text(rng)))
                .chain((key == "tunnel").then(|| "\"10.0.0.0/8\"".to_string()))
                .collect();
            let _ = writeln!(s, "{key} = [{}]", items.join(", "));
        }
    }
    if rng.chance(95) {
        s.push_str("[dns]\n");
        if rng.chance(10) {
            s.push_str("mode = \"system\"\n");
        } else {
            let items: Vec<String> = (0..=rng.below(3))
                .map(|_| format!("\"{}\"", unicast(rng)))
                .collect();
            let _ = writeln!(s, "resolvers = [{}]", items.join(", "));
        }
    }
    if rng.chance(50) {
        let name = *rng.pick(&[
            "septuvo0",
            "wg0",
            "septuvo0",
            "tun0;",
            "",
            "a234567890123456",
        ]);
        let mtu = if rng.chance(70) {
            1280 + rng.below(220)
        } else {
            usize::try_from(rng.interesting_number() & 0xffff).unwrap_or(0)
        };
        let _ = writeln!(s, "[tunnel]\ninterface = \"{name}\"\nmtu = {mtu}");
    }
    if rng.chance(30) {
        let days = if rng.chance(70) {
            rng.below(366)
        } else {
            usize::try_from(rng.interesting_number() & 0xffff).unwrap_or(0)
        };
        let _ = writeln!(s, "[logging]\nretention_days = {days}");
    }
    s.into_bytes()
}

#[test]
fn fuzz_device_schema() {
    let corpus = Corpus {
        seeds: &[DEVICE_FULL, DEVICE_SPLIT],
        dict: DICT,
    };
    run(
        "fuzz_device_schema",
        &corpus,
        structured_device,
        check_device_schema,
    );
}

fn structured_server(rng: &mut Rng) -> Vec<u8> {
    let mut s = String::from("version = 1\n[listen]\n");
    let port = if rng.chance(85) {
        1 + rng.below(65_535)
    } else {
        usize::try_from(rng.interesting_number() & 0x1ffff).unwrap_or(0)
    };
    let _ = writeln!(s, "address = \"{}\"\nport = {port}", ip(rng));
    let (mut v4, mut v6) = (prefix_text(rng), prefix_text(rng));
    if rng.chance(70) {
        v4 = format!("10.{}.0.0/{}", rng.below(256), 16 + rng.below(13));
        v6 = format!("fd{:02x}::/{}", rng.below(256), 48 + rng.below(73));
    }
    let _ = writeln!(s, "[network]\nipv4_range = \"{v4}\"\nipv6_range = \"{v6}\"");
    let dir = *rng.pick(&[
        "/var/lib/septuvo-server",
        "relative",
        "/a/../b",
        "/",
        "/x//y",
    ]);
    let _ = writeln!(s, "[storage]\ndata_dir = \"{dir}\"");
    if rng.chance(50) {
        let _ = writeln!(
            s,
            "[logging]\nretention_days = {}",
            rng.interesting_number()
        );
    }
    s.into_bytes()
}

#[test]
fn fuzz_server_schema() {
    let corpus = Corpus {
        seeds: &[SERVER],
        dict: DICT,
    };
    run(
        "fuzz_server_schema",
        &corpus,
        structured_server,
        check_server_schema,
    );
}

/// Property check for `fuzz_toml_subset` (also called by the cargo-fuzz target).
pub fn check_toml_subset(data: &[u8]) -> bool {
    let first = toml::parse_bytes(data);
    assert_eq!(first, toml::parse_bytes(data), "deterministic");
    match first {
        Ok(doc) => {
            assert!(data.len() <= toml::MAX_INPUT_BYTES);
            assert!(doc.tables.len() <= toml::MAX_TABLES);
            for t in std::iter::once(&doc.root).chain(&doc.tables) {
                assert!(t.entries.len() <= toml::MAX_KEYS_PER_TABLE);
                assert!(t.name.chars().count() <= toml::MAX_NAME_CHARS);
                for e in &t.entries {
                    assert!(e.key.chars().count() <= toml::MAX_NAME_CHARS);
                    match &e.value {
                        Value::String(v) => {
                            assert!(v.chars().count() <= toml::MAX_STRING_CHARS);
                        }
                        Value::Array(items) => {
                            assert!(items.len() <= toml::MAX_ARRAY_ITEMS);
                            for (v, _) in items {
                                assert!(v.chars().count() <= toml::MAX_STRING_CHARS);
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Round trip: written back and parsed again, the same document.
            let text = write_toml(&doc);
            if text.lines().all(|l| l.chars().count() <= MAX_LINE_CHARS)
                && text.len() <= toml::MAX_INPUT_BYTES
            {
                let again =
                    toml::parse(&text).unwrap_or_else(|e| panic!("re-parse failed: {e}\n{text}"));
                assert_eq!(shape(&again), shape(&doc), "round trip");
            }
            true
        }
        Err(e) => {
            let lines = data.split(|&b| b == b'\n').count();
            assert!(
                e.span.line >= 1 && e.span.line <= lines.max(1),
                "error line in range"
            );
            assert!(e.to_string().len() <= 200, "bounded error message");
            false
        }
    }
}

/// Property check for `fuzz_device_schema` (also called by the cargo-fuzz target).
pub fn check_device_schema(data: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(data) else {
        return false;
    };
    let first = parse_device(text);
    assert_eq!(
        format!("{first:?}"),
        format!("{:?}", parse_device(text)),
        "deterministic"
    );
    match first {
        Ok(c) => {
            assert_ne!(c.endpoints.len(), 0, "at least one endpoint");
            assert!(c.endpoints.iter().all(|e| e.port() != 0));
            assert!(i64::from(c.log_retention_days) <= MAX_RETENTION_DAYS);
            let _ = c.summary();
            // A configuration the validator accepts must also render: the
            // kill switch never fails at connect time for a valid file.
            let ks = kill_switch(&c);
            if let Err(e) = nft::render(&c.policy, &ks) {
                panic!(
                    "accepted configuration does not render: {e} (interface {:?})",
                    c.tunnel.interface
                );
            }
            true
        }
        Err(e) => {
            assert!(e.to_string().len() <= 4096, "bounded error report");
            false
        }
    }
}

/// Property check for `fuzz_server_schema` (also called by the cargo-fuzz target).
pub fn check_server_schema(data: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(data) else {
        return false;
    };
    match parse_server(text) {
        Ok(c) => {
            assert!(i64::from(c.log_retention_days) <= MAX_RETENTION_DAYS);
            assert_eq!(c.ipv4_range.family(), septuvo_policy::Family::V4);
            assert_eq!(c.ipv6_range.family(), septuvo_policy::Family::V6);
            assert!(c.data_dir.is_absolute());
            let _ = c.summary();
            true
        }
        Err(e) => {
            assert!(e.to_string().len() <= 4096, "bounded error report");
            false
        }
    }
}
