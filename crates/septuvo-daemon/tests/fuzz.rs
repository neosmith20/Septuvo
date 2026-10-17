//! Fuzz targets for `septuvo-daemon` (N3-03): the service settings, the
//! resolver file, the system account files and hostile requests on the
//! control socket.
//! Longer runs: `SEPTUVO_FUZZ_SECONDS=60 cargo test -p septuvo-daemon --test fuzz -- --nocapture`.

#[path = "../../septuvo-policy/tests/support/fuzzer.rs"]
mod fuzzer;

use fuzzer::{Corpus, Rng, ip_text, run, word};
use septuvo_daemon::engine::{Engine, Firewall, Interfaces};
use septuvo_daemon::resolver::{Resolver, nameservers};
use septuvo_daemon::{guard, settings, sockets};
use septuvo_ipc::{Command, parse_request};
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
        |data| {
            let Ok(text) = std::str::from_utf8(data) else {
                return false;
            };
            let Ok(s) = settings::parse(text) else {
                return false;
            };
            assert!(!s.lab_namespace.is_empty() && !s.lab_namespace.contains('/'));
            assert!(s.runtime_dir.is_absolute() && s.device_config.is_absolute());
            assert_ne!(
                s.captive_helper_user.as_deref(),
                Some("root"),
                "root is never the helper"
            );
            // Round trip: written back, the same settings.
            let mut again = format!(
                "version = 1\n[daemon]\nmode = \"lab\"\nlab_namespace = \"{}\"\nruntime_dir = \"{}\"\n\
             control_group = \"{}\"\ndevice_config = \"{}\"\n",
                s.lab_namespace,
                s.runtime_dir.display(),
                s.control_group,
                s.device_config.display()
            );
            if let Some(u) = &s.captive_helper_user {
                let _ = writeln!(again, "captive_helper_user = \"{u}\"");
            }
            assert_eq!(settings::parse(&again).as_ref(), Ok(&s), "round trip");
            true
        },
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
    run("fuzz_resolver_file", &corpus, structured, |data| {
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
    });
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
    run("fuzz_account_files", &corpus, structured, |data| {
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
    });
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

/// Hostile bytes on the service's sockets: only a well-formed request ever
/// reaches the engine, a request on the status socket never changes
/// anything, and nothing a client sends stops the service.
#[test]
fn fuzz_control_requests() {
    let config = septuvo_config::parse_device(
        "version = 1\n[server]\nendpoints = [\"203.0.113.7:51820\"]\n[routing]\nmode = \"full-tunnel\"\n\
         [dns]\nresolvers = [\"10.77.0.1\"]\n",
    )
    .unwrap();
    let mut engine = Engine::new(
        config,
        CountingFirewall::default(),
        NoInterfaces,
        MemoryResolver::default(),
    )
    .with_captive_helper(Some(995))
    .with_log(|_| {});
    let corpus = Corpus {
        seeds: &[
            "SEPTUVO/1 STATUS\n",
            "SEPTUVO/1 CONNECT\n",
            "SEPTUVO/1 DISCONNECT\n",
            "SEPTUVO/1 CAPTIVE 5\n",
            "SEPTUVO/1 CAPTIVE 2 NETWORK-DNS\n",
        ],
        dict: DICT,
    };
    let structured = |rng: &mut Rng| {
        let mut v = format!(
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
        .into_bytes();
        if rng.chance(20) {
            v.extend(std::iter::repeat_n(b'A', rng.below(600)));
        }
        v
    };
    let mut case = 0u64;
    run("fuzz_control_requests", &corpus, structured, |data| {
        case += 1;
        let control = case.is_multiple_of(2);
        let data = &data[..data.len().min(1024)];
        let (mut client, mut server) = UnixStream::pair().expect("socket pair");
        client.write_all(data).expect("write request");
        let _ = client.shutdown(Shutdown::Write);
        let mut called = None;
        let mut handle = |c: Command| {
            called = Some(c);
            engine.handle(c, control)
        };
        sockets::serve_one(&mut server, &mut handle);
        drop(server);
        let mut reply = Vec::new();
        let _ = client.read_to_end(&mut reply);
        if let Ok(exact) = parse_request(data) {
            assert_eq!(
                called,
                Some(exact),
                "a well-formed request reaches the engine"
            );
        }
        match called {
            Some(c) => {
                assert!(
                    data.starts_with(c.encode().as_bytes()),
                    "only what was sent"
                );
                true
            }
            None => false,
        }
    });
    // The status socket never changed anything; whatever happened, the
    // engine still answers.
    let r = engine.handle(Command::Status, false);
    assert_eq!(r.field("peer"), Some("fail"), "never connected");
}

/// Requests on the status socket alone never change the firewall.
#[test]
fn fuzz_status_socket_changes_nothing() {
    let config = septuvo_config::parse_device(
        "version = 1\n[server]\nendpoints = [\"203.0.113.7:51820\"]\n[routing]\nmode = \"full-tunnel\"\n\
         [dns]\nresolvers = [\"10.77.0.1\"]\n",
    )
    .unwrap();
    let mut engine = Engine::new(
        config,
        CountingFirewall::default(),
        NoInterfaces,
        MemoryResolver::default(),
    )
    .with_captive_helper(Some(995))
    .with_log(|_| {});
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
        |data| {
            let accepted = match parse_request(data) {
                Ok(c) => {
                    let r = engine.handle(c, false);
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
        },
    );
}
