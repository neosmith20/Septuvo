//! Negative tests: invalid policies and hostile policy text are rejected.

use septuvo_policy::parse::{MAX_INPUT_BYTES, MAX_LINE_BYTES};
use septuvo_policy::policy::MAX_RULES;
use septuvo_policy::{
    Action, Mode, ParseErrorKind, PathState, Policy, PolicyError, Prefix, PrefixError, Rule,
    Verdict, parse_policy,
};

fn rule(prefix: &str, action: Action) -> Rule {
    Rule {
        prefix: prefix.parse().unwrap(),
        action,
    }
}

fn kind(text: &str) -> (usize, ParseErrorKind) {
    let e = parse_policy(text).expect_err("input must be rejected");
    (e.line, e.kind)
}

#[test]
fn same_prefix_twice_is_rejected_even_with_different_actions() {
    // Home network and cafe network both 192.168.1.0/24: the policy must not
    // guess which one the user meant.
    let err = Policy::new(
        Mode::Split,
        vec![
            rule("192.168.1.0/24", Action::Tunnel),
            rule("192.168.1.0/24", Action::Direct),
        ],
    )
    .unwrap_err();
    assert_eq!(
        err,
        PolicyError::DuplicatePrefix("192.168.1.0/24".parse().unwrap())
    );

    let err = Policy::new(
        Mode::FullTunnel,
        vec![
            rule("10.0.0.0/8", Action::Block),
            rule("10.0.0.0/8", Action::Block),
        ],
    )
    .unwrap_err();
    assert!(matches!(err, PolicyError::DuplicatePrefix(_)));
}

#[test]
fn direct_default_route_is_rejected() {
    for prefix in ["0.0.0.0/0", "::/0"] {
        for mode in [Mode::FullTunnel, Mode::Split] {
            let rules = vec![
                rule("192.168.50.0/24", Action::Tunnel),
                rule(prefix, Action::Direct),
            ];
            assert!(
                matches!(
                    Policy::new(mode, rules),
                    Err(PolicyError::DirectDefaultRoute(_))
                ),
                "{prefix} in {mode:?}"
            );
        }
    }
}

#[test]
fn block_and_tunnel_default_routes_are_allowed() {
    let policy = Policy::new(
        Mode::Split,
        vec![
            rule("0.0.0.0/0", Action::Tunnel),
            rule("::/0", Action::Block),
        ],
    )
    .unwrap();
    let v6 = "2001:db8::1".parse().unwrap();
    assert_eq!(policy.decide(v6, PathState::Up).verdict, Verdict::Drop);
}

#[test]
fn split_mode_without_tunnel_rule_is_rejected() {
    assert_eq!(
        Policy::new(Mode::Split, vec![]),
        Err(PolicyError::NothingProtected)
    );
    assert_eq!(
        Policy::new(Mode::Split, vec![rule("192.168.1.0/24", Action::Direct)]),
        Err(PolicyError::NothingProtected)
    );
}

#[test]
fn empty_full_tunnel_protects_everything() {
    let policy = Policy::new(Mode::FullTunnel, vec![]).unwrap();
    let dst = "198.51.100.1".parse().unwrap();
    let decision = policy.decide(dst, PathState::Down);
    assert_eq!(decision.verdict, Verdict::Drop);
    assert_eq!(decision.matched, None);
}

#[test]
fn rule_count_is_bounded() {
    let make = |n: usize| -> Vec<Rule> {
        (0..n)
            .map(|i| {
                let i = u32::try_from(i).unwrap();
                let addr = std::net::Ipv4Addr::from(0x0a00_0000 | (i << 8));
                Rule {
                    prefix: Prefix::new(addr.into(), 24).unwrap(),
                    action: Action::Block,
                }
            })
            .collect()
    };
    assert!(Policy::new(Mode::FullTunnel, make(MAX_RULES)).is_ok());
    assert_eq!(
        Policy::new(Mode::FullTunnel, make(MAX_RULES + 1)),
        Err(PolicyError::TooManyRules)
    );
}

#[test]
fn decision_reports_the_matching_rule() {
    let policy = parse_policy("mode full-tunnel\ndirect 192.168.50.10/32\n").unwrap();
    let decision = policy.decide("192.168.50.10".parse().unwrap(), PathState::Up);
    assert_eq!(decision.matched, Some("192.168.50.10/32".parse().unwrap()));
}

#[test]
fn parses_comments_blank_lines_tabs_and_crlf() {
    let text = "# comment\r\n\r\n\tmode   split \r\n  tunnel\t192.168.50.0/24\r\n";
    let policy = parse_policy(text).unwrap();
    assert_eq!(policy.mode(), Mode::Split);
    assert_eq!(policy.rules().len(), 1);
}

#[test]
fn rejects_malformed_statements_with_line_numbers() {
    let cases: &[(&str, (usize, ParseErrorKind))] = &[
        ("", (0, ParseErrorKind::MissingMode)),
        ("# only a comment\n", (0, ParseErrorKind::MissingMode)),
        ("tunnel 10.0.0.0/8\n", (0, ParseErrorKind::MissingMode)),
        (
            "mode split\nmode split\n",
            (2, ParseErrorKind::DuplicateMode),
        ),
        ("mode everything\n", (1, ParseErrorKind::UnknownMode)),
        ("mode FULL-TUNNEL\n", (1, ParseErrorKind::UnknownMode)),
        (
            "mode full-tunnel\nallow 10.0.0.0/8\n",
            (2, ParseErrorKind::UnknownStatement),
        ),
        (
            "mode full-tunnel\nTunnel 10.0.0.0/8\n",
            (2, ParseErrorKind::UnknownStatement),
        ),
        (
            "mode full-tunnel\nnonsense\n",
            (2, ParseErrorKind::UnknownStatement),
        ),
        (
            "mode full-tunnel\ndirect\n",
            (2, ParseErrorKind::WrongArgumentCount),
        ),
        (
            "mode full-tunnel\ndirect 10.0.0.0/8 extra\n",
            (2, ParseErrorKind::WrongArgumentCount),
        ),
        (
            "mode full-tunnel\ndirect 10.0.0.0/8 # trailing comment\n",
            (2, ParseErrorKind::WrongArgumentCount),
        ),
        (
            "mode full-tunnel\n\ndirect 10.0.0.1/8\n",
            (3, ParseErrorKind::Prefix(PrefixError::HostBitsSet)),
        ),
        (
            "mode full-tunnel\ndirect printer.local/32\n",
            (2, ParseErrorKind::Prefix(PrefixError::BadAddress)),
        ),
        (
            "mode full-tunnel\ndirect 0.0.0.0/0\n",
            (
                0,
                ParseErrorKind::Policy(PolicyError::DirectDefaultRoute(
                    Prefix::new(std::net::Ipv4Addr::UNSPECIFIED.into(), 0).unwrap(),
                )),
            ),
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(kind(text), *expected, "input {text:?}");
    }
}

#[test]
fn rejects_control_and_non_ascii_characters() {
    for text in [
        "mode split\ntunnel 192.168.50.0/24\u{0}\n",
        "mode split\ntunnel 192.168.50.0/24\u{1b}[2J\n",
        "mode split\ntunnel 192.168.50.0/24 \u{00a0}\n",
        "mode split\ntunnel\u{2003}192.168.50.0/24\n",
        "mode split\u{7f}\n",
        "mode split\rtunnel 192.168.50.0/24\n",
    ] {
        let (line, kind) = kind(text);
        assert_eq!(kind, ParseErrorKind::InvalidCharacter, "input {text:?}");
        assert!(line == 1 || line == 2, "input {text:?}");
    }
}

#[test]
fn rejects_oversized_input_and_lines() {
    let long_line = format!("mode split\n#{}\n", "x".repeat(MAX_LINE_BYTES));
    assert_eq!(kind(&long_line), (2, ParseErrorKind::LineTooLong));

    let exact_line = format!(
        "mode split\ntunnel 10.0.0.0/8\n#{}\n",
        "x".repeat(MAX_LINE_BYTES - 1)
    );
    assert!(parse_policy(&exact_line).is_ok());

    let huge = format!(
        "mode split\ntunnel 10.0.0.0/8\n{}",
        "#\n".repeat(MAX_INPUT_BYTES)
    );
    assert_eq!(kind(&huge), (0, ParseErrorKind::InputTooLarge));
}

#[test]
fn error_messages_are_readable() {
    let e = parse_policy("mode split\ntunnel 10.0.0.1/8\n").unwrap_err();
    assert_eq!(
        e.to_string(),
        "line 2: invalid prefix: address has bits set beyond the prefix length"
    );
    let e = parse_policy("mode split\n").unwrap_err();
    assert_eq!(
        e.to_string(),
        "invalid policy: split mode needs at least one tunnel rule"
    );
}
