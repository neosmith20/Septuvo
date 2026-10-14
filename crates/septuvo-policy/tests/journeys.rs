//! Table-driven tests for the version 1 journeys (D-027) and the fail-closed,
//! no-implicit-LAN-trust rules (D-006). Public addresses use the
//! documentation ranges (RFC 5737, RFC 3849).

use septuvo_policy::{PathState, Verdict, parse_policy};
use std::net::IpAddr;

use PathState::{Down, Up};
use Verdict::{Direct, Drop, Tunnel};

struct Case {
    name: &'static str,
    dst: &'static str,
    path: PathState,
    expect: Verdict,
}

fn run(policy_text: &str, cases: &[Case]) {
    let policy = parse_policy(policy_text).expect("test policy must be valid");
    for case in cases {
        let dst: IpAddr = case.dst.parse().unwrap();
        let got = policy.decide(dst, case.path).verdict;
        assert_eq!(
            got, case.expect,
            "{}: {} with path {:?}",
            case.name, case.dst, case.path
        );
    }
}

/// Journey A: reach the home NAS from a café, split tunnel.
#[test]
fn journey_a_remote_home_nas_split_tunnel() {
    let policy = "
        # Journey A: only the home network goes through the tunnel
        mode split
        tunnel 192.168.50.0/24
        tunnel 2001:db8:50::/48
    ";
    run(
        policy,
        &[
            Case {
                name: "home NAS via tunnel",
                dst: "192.168.50.10",
                path: Up,
                expect: Tunnel,
            },
            Case {
                name: "home NAS fails closed",
                dst: "192.168.50.10",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "home IPv6 via tunnel",
                dst: "2001:db8:50::10",
                path: Up,
                expect: Tunnel,
            },
            Case {
                name: "home IPv6 fails closed",
                dst: "2001:db8:50::10",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "internet stays direct",
                dst: "198.51.100.20",
                path: Up,
                expect: Direct,
            },
            Case {
                name: "internet direct when down",
                dst: "198.51.100.20",
                path: Down,
                expect: Direct,
            },
            Case {
                name: "cafe LAN not protected in split",
                dst: "10.20.0.5",
                path: Up,
                expect: Direct,
            },
            Case {
                name: "neighbouring /24 not home",
                dst: "192.168.51.10",
                path: Up,
                expect: Direct,
            },
        ],
    );
}

/// Journey B: untrusted Wi-Fi, everything through the user's own server.
#[test]
fn journey_b_untrusted_wifi_full_tunnel() {
    let policy = "
        mode full-tunnel
        # underlay exception: the user's own Septuvo Server endpoint
        direct 203.0.113.7/32
        direct 2001:db8:7::7/128
    ";
    run(
        policy,
        &[
            Case {
                name: "internet via tunnel",
                dst: "198.51.100.20",
                path: Up,
                expect: Tunnel,
            },
            Case {
                name: "kill switch IPv4",
                dst: "198.51.100.20",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "internet IPv6 via tunnel",
                dst: "2001:db8:99::1",
                path: Up,
                expect: Tunnel,
            },
            Case {
                name: "kill switch IPv6",
                dst: "2001:db8:99::1",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "no implicit trust 10/8",
                dst: "10.20.0.1",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "no implicit trust 192.168/16",
                dst: "192.168.1.1",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "no implicit trust 172.16/12",
                dst: "172.16.0.1",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "no implicit trust IPv6 ULA",
                dst: "fd00::1",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "server endpoint direct",
                dst: "203.0.113.7",
                path: Down,
                expect: Direct,
            },
            Case {
                name: "server endpoint IPv6 direct",
                dst: "2001:db8:7::7",
                path: Up,
                expect: Direct,
            },
            Case {
                name: "endpoint exception is exact",
                dst: "203.0.113.8",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "mapped IPv6 is not IPv4",
                dst: "::ffff:203.0.113.7",
                path: Down,
                expect: Drop,
            },
        ],
    );
}

/// At home with the VPN exiting elsewhere: only the approved NAS is direct.
#[test]
fn approved_lan_exception_is_the_only_bypass() {
    let policy = "
        mode full-tunnel
        direct 192.168.50.10/32
    ";
    run(
        policy,
        &[
            Case {
                name: "approved NAS direct",
                dst: "192.168.50.10",
                path: Down,
                expect: Direct,
            },
            Case {
                name: "approved NAS direct when up",
                dst: "192.168.50.10",
                path: Up,
                expect: Direct,
            },
            Case {
                name: "other LAN host fails closed",
                dst: "192.168.50.11",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "other LAN host tunnelled",
                dst: "192.168.50.11",
                path: Up,
                expect: Tunnel,
            },
        ],
    );
}

/// Overlapping prefixes: the most specific rule wins.
#[test]
fn most_specific_prefix_wins() {
    let policy = "
        mode split
        tunnel 192.168.0.0/16
        direct 192.168.1.0/24
        tunnel 192.168.1.128/25
        block 192.168.1.200/32
    ";
    run(
        policy,
        &[
            Case {
                name: "/16 tunnel",
                dst: "192.168.7.1",
                path: Up,
                expect: Tunnel,
            },
            Case {
                name: "/24 direct",
                dst: "192.168.1.5",
                path: Down,
                expect: Direct,
            },
            Case {
                name: "/25 tunnel",
                dst: "192.168.1.130",
                path: Up,
                expect: Tunnel,
            },
            Case {
                name: "/25 fails closed",
                dst: "192.168.1.130",
                path: Down,
                expect: Drop,
            },
            Case {
                name: "/32 block beats all",
                dst: "192.168.1.200",
                path: Up,
                expect: Drop,
            },
            Case {
                name: "outside all rules",
                dst: "198.51.100.1",
                path: Up,
                expect: Direct,
            },
        ],
    );
}

/// The same rules in any order give the same decisions.
#[test]
fn decisions_do_not_depend_on_rule_order() {
    let lines = [
        "tunnel 192.168.0.0/16",
        "direct 192.168.1.0/24",
        "tunnel 192.168.1.128/25",
        "block 192.168.1.200/32",
        "direct 2001:db8::/32",
        "tunnel 2001:db8:1::/48",
    ];
    let probes = [
        "192.168.7.1",
        "192.168.1.5",
        "192.168.1.130",
        "192.168.1.200",
        "2001:db8::1",
        "2001:db8:1::1",
        "198.51.100.1",
    ];
    let build = |order: &[usize]| {
        let mut text = String::from("mode full-tunnel\n");
        for &i in order {
            text.push_str(lines[i]);
            text.push('\n');
        }
        parse_policy(&text).unwrap()
    };
    let reference = build(&[0, 1, 2, 3, 4, 5]);
    for order in [[5, 4, 3, 2, 1, 0], [2, 0, 5, 3, 1, 4], [3, 5, 1, 4, 0, 2]] {
        let other = build(&order);
        assert_eq!(reference, other, "validated policies differ for {order:?}");
        for dst in probes {
            let dst: IpAddr = dst.parse().unwrap();
            for path in [Up, Down] {
                assert_eq!(reference.decide(dst, path), other.decide(dst, path));
            }
        }
    }
}
