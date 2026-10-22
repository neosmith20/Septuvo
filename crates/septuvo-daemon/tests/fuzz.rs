//! Fuzz targets for `septuvo-daemon` (N3-03): the service settings, the
//! resolver file, the system account files and hostile requests on the
//! control socket.
//! Longer runs: `SEPTUVO_FUZZ_SECONDS=60 cargo test -p septuvo-daemon --test fuzz -- --nocapture`.

// The check functions are public so the cargo-fuzz targets (fuzz/) can call
// them; they panic on purpose when a property fails.
#![allow(clippy::must_use_candidate, clippy::missing_panics_doc)]

#[path = "../../septuvo-policy/tests/support/fuzzer.rs"]
mod fuzzer;

use fuzzer::{Corpus, Rng, ip_text, run, word};
use septuvo_daemon::engine::{Engine, Firewall, Interfaces};
use septuvo_daemon::resolver::{Resolver, nameservers};
use septuvo_daemon::{guard, settings, sockets};
use septuvo_ipc::{Command, Request, parse_framed};
use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::{IpAddr, Shutdown};
use std::os::unix::net::UnixStream;

const SETTINGS: &str = "version = 1\n[daemon]\nmode = \"lab\"\nlab_namespace = \"septuvo-lab\"\n\
                        runtime_dir = \"/run/septuvo\"\ncontrol_group = \"septuvo\"\n\
                        device_config = \"/etc/septuvo/septuvo.toml\"\n\
                        captive_helper_user = \"septuvo-portal\"\n";
const DICT: &[&str] = &[
    "version",
    "=",
    "1",
    "[daemon]",
    "mode",
    "\"lab\"",
    "\"production\"",
    "lab_namespace",
    "runtime_dir",
    "control_group",
    "device_config",
    "captive_helper_user",
    "connections_dir",
    "\"root\"",
    "\"/\"",
    "\"/run//x\"",
    "\"/etc/../shadow\"",
    "\"septuvo\"",
    "\"-bad\"",
    "nameserver",
    "options",
    "127.0.0.53",
    "::1",
    "0.0.0.0",
    "224.0.0.251",
    "ff02::fb",
    "%eth0",
    "Uid:",
    "\t",
    ":",
    "x",
];

fn structured_settings(rng: &mut Rng) -> Vec<u8> {
    let mut s = String::from("version = 1\n[daemon]\n");
    let ns = if rng.chance(80) {
        "septuvo-lab"
    } else {
        *rng.pick(&["sv-c-1", "-bad", "a/b", "", "x.y"])
    };
    let mode = if rng.chance(90) { "lab" } else { "production" };
    let _ = std::fmt::Write::write_fmt(
        &mut s,
        format_args!("mode = \"{mode}\"\nlab_namespace = \"{ns}\"\n"),
    );
    for (key, options) in [
        (
            "runtime_dir",
            &["/run/septuvo", "/tmp/x", "relative", "/run//x", "/a/./b"][..],
        ),
        (
            "control_group",
            &["septuvo", "root", "Bad Group", "_x", "a-b"][..],
        ),
        (
            "device_config",
            &["/etc/septuvo/septuvo.toml", "x.toml", "/etc/../shadow"][..],
        ),
        (
            "captive_helper_user",
            &["septuvo-portal", "root", "svch1", "", "Portal"][..],
        ),
    ] {
        if rng.chance(70) || key == "device_config" {
            let value = if rng.chance(80) {
                options[0]
            } else {
                *rng.pick(options)
            };
            let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{key} = \"{value}\"\n"));
        }
    }
    s.into_bytes()
}

#[test]
fn fuzz_daemon_settings() {
    let corpus = Corpus {
        seeds: &[SETTINGS],
        dict: DICT,
    };
    run(
        "fuzz_daemon_settings",
        &corpus,
        structured_settings,
        check_daemon_settings,
    );
}

#[test]
fn fuzz_resolver_file() {
    let corpus = Corpus {
        seeds: &[
            "nameserver 192.168.1.1\nnameserver fe80::1\n",
            "# comment\nnameserver 127.0.0.53\noptions edns0 trust-ad\nsearch lan\n",
        ],
        dict: DICT,
    };
    let structured = |rng: &mut Rng| {
        let mut s = String::new();
        for _ in 0..rng.below(10) {
            let word = *rng.pick(&[
                "nameserver",
                "nameserver",
                "options",
                "search",
                "#nameserver",
            ]);
            let _ = writeln!(s, "{word}{}{}", rng.pick(&[" ", "\t", "  "]), ip_text(rng));
        }
        s.into_bytes()
    };
    run(
        "fuzz_resolver_file",
        &corpus,
        structured,
        check_resolver_file,
    );
}

#[test]
fn fuzz_account_files() {
    let corpus = Corpus {
        seeds: &[
            "root:x:0:0::/root:/bin/sh\nseptuvo-portal:x:995:995::/:/usr/sbin/nologin\n",
            "septuvo:x:990:\nwheel:x:10:alex\n",
            "Name:\tseptuvod\nUid:\t0\t0\t0\t0\nGid:\t0\t0\t0\t0\n",
        ],
        dict: DICT,
    };
    let structured = |rng: &mut Rng| {
        let mut s = String::new();
        for _ in 0..rng.below(8) {
            let _ = writeln!(
                s,
                "{}:x:{}:{}:{}",
                word(rng, b"abcdefgh-_", 12),
                rng.interesting_number(),
                rng.interesting_number(),
                word(rng, b"/abc:", 10)
            );
        }
        if rng.chance(30) {
            let _ = writeln!(
                s,
                "Uid:\t{}\t{}",
                rng.interesting_number(),
                word(rng, b"0123456789\t ", 10)
            );
        }
        s.into_bytes()
    };
    run(
        "fuzz_account_files",
        &corpus,
        structured,
        check_account_files,
    );
}

/// Counts every change the service asks of the firewall.
#[derive(Default)]
struct CountingFirewall {
    installed: bool,
    applied: usize,
}

impl Firewall for CountingFirewall {
    fn apply(&mut self, script: &str) -> Result<(), String> {
        self.applied += 1;
        if script.contains("chain output") {
            self.installed = true;
        } else if script.contains("delete table") {
            self.installed = false;
        }
        Ok(())
    }
    fn active(&mut self) -> bool {
        self.installed
    }
    fn captive_open(&mut self) -> bool {
        false
    }
}

struct NoInterfaces;
impl Interfaces for NoInterfaces {
    fn is_up(&self, _: &str) -> bool {
        false
    }
}

#[derive(Default)]
struct MemoryResolver(Option<Vec<IpAddr>>);
impl Resolver for MemoryResolver {
    fn install(&mut self, servers: &[IpAddr]) -> Result<(), String> {
        self.0 = Some(servers.to_vec());
        Ok(())
    }
    fn restore(&mut self) -> Result<(), String> {
        self.0 = None;
        Ok(())
    }
    fn installed(&self) -> bool {
        self.0.is_some()
    }
    fn network_resolvers(&self) -> Vec<IpAddr> {
        vec!["192.168.1.1".parse().unwrap()]
    }
}

/// A fresh service engine with test doubles (nothing touches the system).
fn test_engine() -> Engine<CountingFirewall, NoInterfaces, MemoryResolver> {
    let config = septuvo_config::parse_device(
        "version = 1\n[server]\nendpoints = [\"203.0.113.7:51820\"]\n[routing]\nmode = \"full-tunnel\"\n\
         [dns]\nresolvers = [\"10.77.0.1\"]\n",
    )
    .unwrap();
    Engine::new(
        config,
        CountingFirewall::default(),
        NoInterfaces,
        MemoryResolver::default(),
    )
    .with_captive_helper(Some(995))
    .with_log(|_| {})
}

/// Hostile bytes on the service's sockets: only a well-formed request ever
/// reaches the engine, and a well-formed one always does.
#[test]
fn fuzz_control_requests() {
    let corpus = Corpus {
        seeds: &[
            "SEPTUVO/1 STATUS\n",
            "SEPTUVO/1 CONNECT\n",
            "SEPTUVO/1 DISCONNECT\n",
            "SEPTUVO/1 CAPTIVE 5\n",
            "\u{3}SEPTUVO/1 CAPTIVE 2 NETWORK-DNS\n",
            "\u{2}SEPTUVO/1 DISCONNECT\n",
            "\u{1}SEPTUVO/1 CONNECT\n",
        ],
        dict: DICT,
    };
    let structured = |rng: &mut Rng| {
        let mut v = Vec::new();
        if rng.chance(50) {
            v.push(u8::try_from(rng.below(4)).unwrap_or(0));
        }
        v.extend(
            format!(
                "SEPTUVO/1 {}\n",
                rng.pick(&[
                    "STATUS",
                    "CONNECT",
                    "DISCONNECT",
                    "CAPTIVE 3",
                    "CAPTIVE 9",
                    "REBOOT"
                ])
            )
            .into_bytes(),
        );
        if rng.chance(20) {
            v.extend(std::iter::repeat_n(b'A', rng.below(600)));
        }
        v
    };
    run(
        "fuzz_control_requests",
        &corpus,
        structured,
        check_control_requests,
    );
}

/// Change commands on the status socket are always refused.
#[test]
fn fuzz_status_socket_changes_nothing() {
    let corpus = Corpus {
        seeds: &[
            "SEPTUVO/1 CONNECT\n",
            "SEPTUVO/1 CAPTIVE 5\n",
            "SEPTUVO/1 DISCONNECT\n",
        ],
        dict: DICT,
    };
    let structured = |rng: &mut Rng| {
        format!(
            "SEPTUVO/1 {}\n",
            rng.pick(&[
                "CONNECT",
                "DISCONNECT",
                "CAPTIVE 1",
                "CAPTIVE 5 NETWORK-DNS"
            ])
        )
        .into_bytes()
    };
    run(
        "fuzz_status_socket_changes_nothing",
        &corpus,
        structured,
        check_status_socket_changes_nothing,
    );
}

/// Property check for `fuzz_daemon_settings` (also called by the cargo-fuzz target).
pub fn check_daemon_settings(data: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(data) else {
        return false;
    };
    let Ok(s) = settings::parse(text) else {
        return false;
    };
    assert_ne!(s.lab_namespace.len(), 0);
    assert!(!s.lab_namespace.contains('/'));
    assert!(s.runtime_dir.is_absolute() && s.device_config.is_absolute());
    assert_ne!(
        s.captive_helper_user.as_deref(),
        Some("root"),
        "root is never the helper"
    );
    // Round trip: written back (with TOML escapes), the same settings.
    let q = |v: &str| v.replace('\\', "\\\\").replace('"', "\\\"");
    let mut again = format!(
        "version = 1\n[daemon]\nmode = \"lab\"\nlab_namespace = \"{}\"\nruntime_dir = \"{}\"\n\
             control_group = \"{}\"\ndevice_config = \"{}\"\n",
        q(&s.lab_namespace),
        q(&s.runtime_dir.to_string_lossy()),
        q(&s.control_group),
        q(&s.device_config.to_string_lossy())
    );
    if let Some(u) = &s.captive_helper_user {
        let _ = writeln!(again, "captive_helper_user = \"{}\"", q(u));
    }
    assert_eq!(settings::parse(&again).as_ref(), Ok(&s), "round trip");
    true
}

/// Property check for `fuzz_resolver_file` (also called by the cargo-fuzz target).
pub fn check_resolver_file(data: &[u8]) -> bool {
    let text = String::from_utf8_lossy(data);
    let found = nameservers(&text);
    assert!(found.len() <= septuvo_policy::nft::MAX_CAPTIVE_RESOLVERS);
    for a in &found {
        assert!(
            !a.is_loopback() && !a.is_unspecified() && !a.is_multicast(),
            "{a}"
        );
        let from_file = text.lines().any(|l| {
            let mut w = l.split_whitespace();
            w.next() == Some("nameserver")
                && w.next().and_then(|x| x.parse::<IpAddr>().ok()) == Some(*a)
        });
        assert!(from_file, "{a} comes from a nameserver line");
    }
    // Whatever the file holds, the captive-portal renderer accepts the
    // result (the service never builds an invalid window from it).
    assert!(septuvo_policy::nft::captive_script(60, &found).is_ok());
    !found.is_empty()
}

/// Property check for `fuzz_account_files` (also called by the cargo-fuzz target).
pub fn check_account_files(data: &[u8]) -> bool {
    let text = String::from_utf8_lossy(data);
    let user = guard::user_id(&text, "septuvo-portal");
    let group = guard::group_id(&text, "septuvo");
    let euid = guard::effective_uid(&text);
    // An ID is only ever read from a line that names exactly that account.
    if let Some(uid) = user {
        assert!(
            text.lines()
                .any(|l| l.starts_with("septuvo-portal:") && l.contains(&uid.to_string()))
        );
    }
    if let Some(gid) = group {
        assert!(
            text.lines()
                .any(|l| l.starts_with("septuvo:") && l.contains(&gid.to_string()))
        );
    }
    user.is_some() || group.is_some() || euid.is_some()
}

/// Property check for `fuzz_control_requests` (also called by the cargo-fuzz
/// target). An optional first byte 0 to 3 picks the case: bit 0 = the control
/// socket (otherwise the status socket), bit 1 = the service is connected
/// first. Any other first byte: the whole input arrives on the control socket.
pub fn check_control_requests(data: &[u8]) -> bool {
    let (mode, data) = match data.first() {
        Some(&m) if m < 4 => (m, &data[1..]),
        _ => (1, data),
    };
    let control = mode & 1 == 1;
    let mut engine = test_engine();
    if mode & 2 == 2 {
        let _ = engine.handle(Command::Connect, true);
    }
    let data = &data[..data.len().min(1024)];
    let (mut client, mut server) = UnixStream::pair().expect("socket pair");
    client.write_all(data).expect("write request");
    let _ = client.shutdown(Shutdown::Write);
    let mut called: Option<Request> = None;
    let mut handle = |r: &Request| {
        called = Some(r.clone());
        engine.handle_request(r, control)
    };
    sockets::serve_one(&mut server, &mut handle);
    drop(server);
    let mut reply = Vec::new();
    let _ = client.read_to_end(&mut reply);
    if let Ok(exact) = parse_framed(data) {
        assert_eq!(
            called,
            Some(exact),
            "a well-formed request reaches the engine"
        );
    }
    // Whatever happened, the service still answers and never claims a peer.
    let r = engine.handle(Command::Status, false);
    assert_eq!(r.field("peer"), Some("fail"), "never connected");
    match called {
        Some(r) => {
            assert!(data.starts_with(&r.encode()), "only what was sent");
            true
        }
        None => false,
    }
}

/// Property check for `fuzz_status_socket_changes_nothing` (also called by
/// the cargo-fuzz target).
pub fn check_status_socket_changes_nothing(data: &[u8]) -> bool {
    let mut engine = test_engine();
    let accepted = match parse_framed(data) {
        Ok(request) => {
            let c = request.command;
            let r = engine.handle_request(&request, false);
            if c.needs_control() {
                assert!(matches!(r.outcome, septuvo_ipc::Outcome::Err { .. }));
            }
            true
        }
        Err(_) => false,
    };
    let r = engine.handle(Command::Status, false);
    assert_eq!(r.field("kill_switch"), Some("off"), "nothing was installed");
    assert_eq!(r.field("state"), Some("disconnected"));
    accepted
}
