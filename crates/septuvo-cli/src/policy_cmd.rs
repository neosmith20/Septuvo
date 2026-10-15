//! `septuvo policy check` (N1-04): validate a policy file and explain it.
//!
//! Reads at most `MAX_INPUT_BYTES` + 1 bytes (anything larger is rejected by
//! the parser), never touches the network or the system configuration.

use septuvo_policy::parse::MAX_INPUT_BYTES;
use septuvo_policy::{Action, Mode, PathState, Policy, Verdict, parse_policy};
use std::fmt::Write as _;
use std::io::Read;
use std::net::IpAddr;

/// Options after `septuvo policy check <file>`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CheckArgs {
    /// Destination to evaluate.
    pub dst: Option<IpAddr>,
    /// VPN path state for the evaluation (default: both).
    pub path: Option<PathState>,
}

/// Parses `[--dst ADDRESS] [--path up|down]`.
///
/// # Errors
/// A message for unknown, repeated or malformed options.
pub fn parse_args(args: &[String]) -> Result<CheckArgs, String> {
    let mut out = CheckArgs::default();
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let value = it
            .next()
            .ok_or_else(|| format!("{} needs a value", safe(flag)))?;
        match flag.as_str() {
            "--dst" if out.dst.is_none() => {
                out.dst = Some(
                    value
                        .parse()
                        .map_err(|_| format!("not an IP address: {}", safe(value)))?,
                );
            }
            "--path" if out.path.is_none() => {
                out.path = Some(match value.as_str() {
                    "up" => PathState::Up,
                    "down" => PathState::Down,
                    _ => return Err("--path must be up or down".into()),
                });
            }
            _ => return Err(format!("unexpected argument: {}", safe(flag))),
        }
    }
    if out.path.is_some() && out.dst.is_none() {
        return Err("--path needs --dst".into());
    }
    Ok(out)
}

/// Removes control characters and limits length before echoing user input.
#[must_use]
pub fn safe(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).take(80).collect()
}

/// Reads and validates a policy file.
///
/// # Errors
/// A message if the file cannot be read or the policy is invalid.
pub fn load(path: &str) -> Result<Policy, String> {
    let mut text = String::new();
    std::fs::File::open(path)
        .map_err(|e| format!("{}: {e}", safe(path)))?
        .take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|e| format!("{}: {e}", safe(path)))?;
    parse_policy(&text).map_err(|e| format!("{}: {e}", safe(path)))
}

fn verdict_text(v: Verdict) -> &'static str {
    match v {
        Verdict::Tunnel => "through the tunnel",
        Verdict::Direct => "directly, outside the tunnel",
        Verdict::Drop => "dropped",
    }
}

/// Human-readable explanation of a valid policy, and optionally a decision.
#[must_use]
pub fn explain(policy: &Policy, args: &CheckArgs) -> String {
    let mut s = String::from("Policy is valid.\n\n");
    match policy.mode() {
        Mode::FullTunnel => s.push_str(
            "Mode: full tunnel. Everything goes through the tunnel unless a rule says otherwise;\n\
             with the tunnel down, that traffic is dropped (kill switch).\n",
        ),
        Mode::Split => s.push_str(
            "Mode: split tunnel. Only destinations in `tunnel` rules are protected;\n\
             all other traffic goes directly and is NOT protected by Septuvo.\n",
        ),
    }
    if policy.rules().is_empty() {
        s.push_str("\nNo rules.\n");
    } else {
        s.push_str("\nRules, most specific first:\n");
        for rule in policy.rules() {
            let action = match rule.action {
                Action::Tunnel => "tunnel  (dropped when the tunnel is down)",
                Action::Direct => "direct  (approved exception, outside the tunnel)",
                Action::Block => "block   (always dropped)",
            };
            let _ = writeln!(s, "  {:<22} {action}", rule.prefix.to_string());
        }
    }
    if let Some(dst) = args.dst {
        s.push('\n');
        let paths: &[PathState] = match args.path {
            Some(PathState::Up) => &[PathState::Up],
            Some(PathState::Down) => &[PathState::Down],
            None => &[PathState::Up, PathState::Down],
        };
        for &path in paths {
            let d = policy.decide(dst, path);
            let why = d.matched.map_or_else(
                || "default for this mode".to_string(),
                |p| format!("rule {p}"),
            );
            let state = if path == PathState::Up { "up" } else { "down" };
            let _ = writeln!(
                s,
                "{dst} with the tunnel {state}: {} ({why})",
                verdict_text(d.verdict)
            );
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn parses_options() {
        let a = parse_args(&strings(&["--dst", "192.168.50.10", "--path", "down"])).unwrap();
        assert_eq!(a.dst, Some("192.168.50.10".parse().unwrap()));
        assert_eq!(a.path, Some(PathState::Down));
        assert_eq!(parse_args(&[]).unwrap(), CheckArgs::default());
    }

    #[test]
    fn rejects_bad_options() {
        for bad in [
            vec!["--dst"],
            vec!["--dst", "nas.local"],
            vec!["--path", "sideways", "--dst", "10.0.0.1"],
            vec!["--path", "up"],
            vec!["--dst", "10.0.0.1", "--dst", "10.0.0.2"],
            vec!["--color", "red"],
        ] {
            assert!(parse_args(&strings(&bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn split_mode_states_what_is_not_protected() {
        let p = parse_policy("mode split\ntunnel 192.168.50.0/24\n").unwrap();
        let text = explain(&p, &CheckArgs::default());
        assert!(text.contains("NOT protected by Septuvo"));
    }

    #[test]
    fn explains_decisions_for_both_path_states() {
        let p = parse_policy("mode full-tunnel\ndirect 192.168.50.10/32\n").unwrap();
        let args = CheckArgs {
            dst: Some("198.51.100.20".parse().unwrap()),
            path: None,
        };
        let text = explain(&p, &args);
        assert!(text.contains(
            "198.51.100.20 with the tunnel up: through the tunnel (default for this mode)"
        ));
        assert!(
            text.contains("198.51.100.20 with the tunnel down: dropped (default for this mode)")
        );
        let nas = CheckArgs {
            dst: Some("192.168.50.10".parse().unwrap()),
            path: Some(PathState::Down),
        };
        assert!(explain(&p, &nas).contains("directly, outside the tunnel (rule 192.168.50.10/32)"));
    }

    #[test]
    fn safe_strips_control_characters() {
        assert_eq!(safe("a\u{1b}[2Jb\nc"), "a[2Jbc");
        assert_eq!(safe(&"x".repeat(500)).len(), 80);
    }
}
