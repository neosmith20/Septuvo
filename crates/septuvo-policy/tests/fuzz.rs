//! Fuzz targets for `septuvo-policy` (N3-03): the policy file format, address
//! prefixes and the nftables renderer. Run with more cases or for a time:
//! `SEPTUVO_FUZZ_SECONDS=60 cargo test -p septuvo-policy --test fuzz -- --nocapture`.

#[path = "support/fuzzer.rs"]
mod fuzzer;

use fuzzer::{Corpus, Rng, ip, prefix_text, run};
use septuvo_policy::nft::{self, DnsRules, KillSwitch, MAX_CAPTIVE_SECONDS};
use septuvo_policy::{Action, Mode, PathState, Policy, Prefix, parse_policy};
use std::fmt::Write as _;
use std::net::IpAddr;

const POLICY_SEEDS: &[&str] = &[
    "mode full-tunnel\n",
    "# Untrusted Wi-Fi\nmode full-tunnel\ndirect 192.168.1.20/32\nblock 10.0.0.0/8\n",
    "mode split\ntunnel 192.168.50.0/24\ntunnel 2001:db8:50::/64\ndirect 192.168.50.10/32\n",
    "mode split\r\ntunnel 10.0.0.0/8\r\n",
    "\tmode   full-tunnel  \n  block ::/0\n",
];
const POLICY_DICT: &[&str] = &[
    "mode",
    "full-tunnel",
    "split",
    "tunnel",
    "direct",
    "block",
    "#",
    "/",
    "/0",
    "/32",
    "/128",
    "/33",
    "/129",
    "::",
    "::ffff:",
    "0.0.0.0",
    "255.255.255.255",
    "10.0.0.0",
    "fe80::",
    " ",
    "\t",
    "\n",
    "\r\n",
    "%eth0",
    "01",
    "1.2.3",
    "1.2.3.4.5",
];

/// A policy written from its parts, the way a person would write it.
fn policy_text(mode: Mode, rules: impl IntoIterator<Item = (Action, Prefix)>) -> String {
    let mut s = String::from(match mode {
        Mode::FullTunnel => "mode full-tunnel\n",
        Mode::Split => "mode split\n",
    });
    for (action, prefix) in rules {
        let word = match action {
            Action::Tunnel => "tunnel",
            Action::Direct => "direct",
            Action::Block => "block",
        };
        let _ = writeln!(s, "{word} {prefix}");
    }
    s
}

fn structured_policy(rng: &mut Rng) -> Vec<u8> {
    let mut s = String::new();
    if !rng.chance(5) {
        s.push_str(rng.pick(&["mode full-tunnel\n", "mode split\n"]));
    }
    for _ in 0..rng.below(12) {
        let word = if rng.chance(95) {
            *rng.pick(&["tunnel", "direct", "block"])
        } else {
            *rng.pick(&["mode", "allow", "Tunnel"])
        };
        let _ = writeln!(s, "{word} {}", prefix_text(rng));
    }
    if rng.chance(50) {
        s.push_str("tunnel 10.0.0.0/8\n");
    }
    s.into_bytes()
}

#[test]
fn fuzz_policy_file() {
    let corpus = Corpus {
        seeds: POLICY_SEEDS,
        dict: POLICY_DICT,
    };
    run("fuzz_policy_file", &corpus, structured_policy, |data| {
        let Ok(text) = std::str::from_utf8(data) else {
            return false;
        };
        match parse_policy(text) {
            Ok(policy) => {
                // Round trip: the policy written back parses to the same policy.
                let again = policy_text(
                    policy.mode(),
                    policy.rules().iter().map(|r| (r.action, r.prefix)),
                );
                assert_eq!(parse_policy(&again).as_ref(), Ok(&policy), "round trip");
                // Deciding never panics, for any address and path state.
                for addr in [
                    "0.0.0.0",
                    "10.1.2.3",
                    "192.168.1.20",
                    "::",
                    "2001:db8:50::1",
                ] {
                    let addr: IpAddr = addr.parse().unwrap();
                    let _ = policy.decide(addr, PathState::Up);
                    let _ = policy.decide(addr, PathState::Down);
                }
                true
            }
            Err(e) => {
                // Errors are bounded and never echo the input back.
                let msg = e.to_string();
                assert!(msg.len() <= 200, "error message too long: {msg}");
                assert!(
                    e.line <= text.split('\n').count(),
                    "line number out of range"
                );
                false
            }
        }
    });
}

#[test]
fn fuzz_prefix() {
    let corpus = Corpus {
        seeds: &[
            "10.0.0.0/8",
            "192.168.1.20/32",
            "2001:db8::/32",
            "::/0",
            "fe80::1/128",
        ],
        dict: POLICY_DICT,
    };
    let structured = |rng: &mut Rng| prefix_text(rng).into_bytes();
    run("fuzz_prefix", &corpus, structured, |data| {
        let Ok(text) = std::str::from_utf8(data) else {
            return false;
        };
        if let Ok(p) = text.parse::<Prefix>() {
            let shown = p.to_string();
            assert_eq!(shown.parse::<Prefix>(), Ok(p), "round trip of {shown}");
            assert!(p.contains(p.addr()), "a prefix contains its own address");
            let max = match p.addr() {
                IpAddr::V4(_) => 32,
                IpAddr::V6(_) => 128,
            };
            assert!(p.prefix_len() <= max);
            return true;
        }
        false
    });
}

/// Interface names, including hostile ones that must never reach nft.
const IFACES: &[&str] = &[
    "septuvo0",
    "wg_test-1.2",
    "",
    "a234567890123456",
    "tun0\"; flush ruleset; \"",
    "tun0}",
    "tun0\n",
    "-tun",
    ".",
    "..",
    "tün0",
    "tun 0",
    "tun0;",
    "tun0#",
];

fn structured_killswitch(rng: &mut Rng) -> Vec<u8> {
    // The bytes only seed a second generator: the renderer takes typed values.
    rng.next_u64().to_le_bytes().to_vec()
}

#[test]
fn fuzz_nft_render() {
    let corpus = Corpus {
        seeds: &[],
        dict: &[],
    };
    run("fuzz_nft_render", &corpus, structured_killswitch, |data| {
        let mut seed = [0u8; 8];
        for (i, b) in data.iter().take(8).enumerate() {
            seed[i] = *b;
        }
        let mut rng = Rng::new(u64::from_le_bytes(seed) ^ u64::try_from(data.len()).unwrap_or(0));
        let mut rules = Vec::new();
        for _ in 0..rng.below(20) {
            if let Ok(p) = prefix_text(&mut rng).parse::<Prefix>() {
                rules.push((
                    *rng.pick(&[Action::Tunnel, Action::Direct, Action::Block]),
                    p,
                ));
            }
        }
        rules.push((Action::Tunnel, "10.0.0.0/8".parse().unwrap()));
        let mode = *rng.pick(&[Mode::FullTunnel, Mode::Split]);
        let Ok(policy) = parse_policy(&policy_text(mode, rules)) else {
            return false;
        };
        let addrs = |rng: &mut Rng, max: usize| (0..rng.below(max)).map(|_| ip(rng)).collect();
        let ks = KillSwitch {
            tunnel_interface: if rng.chance(70) {
                (*rng.pick(&IFACES[..2])).to_string()
            } else {
                (*rng.pick(IFACES)).to_string()
            },
            endpoints: (0..{
                let max = if rng.chance(90) { 5 } else { 20 };
                rng.below(max)
            })
                .map(|_| {
                    let port = if rng.chance(90) {
                        1 + rng.below(65_535)
                    } else {
                        usize::try_from(rng.interesting_number() & 0xffff).unwrap_or(0)
                    };
                    (ip(&mut rng), u16::try_from(port).unwrap_or(0))
                })
                .collect(),
            dns: match rng.below(3) {
                0 => DnsRules::TunnelOnly,
                1 => DnsRules::Resolvers {
                    resolvers: {
                        let max = if rng.chance(90) { 4 } else { 20 };
                        let mut r: Vec<IpAddr> = addrs(&mut rng, max);
                        if rng.chance(90) {
                            r.push(ip(&mut rng));
                        }
                        r
                    },
                    lan_exceptions: addrs(&mut rng, 5),
                },
                _ => DnsRules::System,
            },
            captive_helper: match rng.below(10) {
                0..=3 => None,
                4 => Some(0),
                _ => Some(u32::try_from(rng.interesting_number() & 0xffff_ffff).unwrap_or(1)),
            },
        };
        let rendered = check_render(&policy, &ks);
        let seconds = u32::try_from(rng.interesting_number() & 0x3ff).unwrap_or(0);
        let resolvers: Vec<IpAddr> = addrs(&mut rng, 7);
        if let Ok(s) = nft::captive_script(seconds, &resolvers) {
            assert!((1..=MAX_CAPTIVE_SECONDS).contains(&seconds));
            assert!(resolvers.len() <= nft::MAX_CAPTIVE_RESOLVERS);
            assert!(s.len() <= 1024, "captive script bounded");
            assert!(!s.contains(';'));
        }
        rendered
    });
}

/// Properties of a rendered ruleset: only validated values reach it, its
/// size is bounded and no statement can be injected.
fn check_render(policy: &Policy, ks: &KillSwitch) -> bool {
    let Ok(s) = nft::render(policy, ks) else {
        return false;
    };
    let name = &ks.tunnel_interface;
    assert!(
        (1..=15).contains(&name.len())
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')),
        "accepted interface name {name:?}"
    );
    assert_ne!(ks.captive_helper, Some(0), "root is never the helper");
    assert!(ks.endpoints.iter().all(|&(_, p)| p != 0));
    assert!(
        s.bytes()
            .all(|b| b == b'\n' || b == b'\t' || (0x20..=0x7e).contains(&b)),
        "printable output only"
    );
    // Outside fixed comment lines, semicolons only come from the fixed
    // templates: the chain header and two per set declaration.
    let sets = if ks.captive_helper.is_some() { 3 } else { 0 };
    let semicolons: usize = s
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(|l| l.matches(';').count())
        .sum();
    assert_eq!(semicolons, 2 + 2 * sets, "no injected statements");
    let dns = match &ks.dns {
        DnsRules::Resolvers {
            resolvers,
            lan_exceptions,
        } => resolvers.len() + lan_exceptions.len(),
        _ => 0,
    };
    let bound = 40 + 3 * policy.rules().len() + ks.endpoints.len() + 2 * dns + 10;
    assert!(s.lines().count() <= bound, "ruleset size bounded");
    if let Some(uid) = ks.captive_helper {
        for line in s.lines().filter(|l| l.contains('@')) {
            assert!(
                line.trim_start().starts_with(&format!("meta skuid {uid} ")),
                "{line}"
            );
        }
    } else {
        assert!(!s.contains("captive") && !s.contains("skuid"));
    }
    true
}
