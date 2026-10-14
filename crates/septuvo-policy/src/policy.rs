//! Route and LAN-exception policy: decides, for each destination address,
//! whether traffic must use the VPN path, may go directly, or is dropped.
//!
//! Security properties (D-006, S-08, S-09):
//! - **Fail closed:** traffic routed to the tunnel is dropped, never sent
//!   directly, while the VPN path is down.
//! - **No implicit LAN trust:** private ranges such as `192.168.0.0/16` get no
//!   special treatment; only explicit `Direct` rules bypass the tunnel.
//! - **Deterministic:** the most specific matching prefix wins; two rules for
//!   the same prefix are rejected instead of being resolved by order.
//! - **Bounded:** at most [`MAX_RULES`] rules.
//!
//! Scope: this model decides for traffic that leaves the device. Loopback,
//! link-local neighbour discovery and multicast handling belong to the
//! operating-system layer (N1-03) and later specifications (M1-02).

use crate::prefix::{Family, Prefix};
use std::fmt;
use std::net::IpAddr;

/// Maximum number of rules in one policy.
pub const MAX_RULES: usize = 1024;

/// What happens to destinations that no rule matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Everything uses the VPN path unless a rule says otherwise
    /// (journey B, untrusted Wi-Fi; D-027).
    FullTunnel,
    /// Only destinations matched by `Tunnel` rules use the VPN path; all other
    /// traffic goes directly and is **not** protected by Septuvo
    /// (journey A, home network access; D-027).
    Split,
}

/// What a rule does with the destinations it matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Send through the VPN path; drop while the path is down.
    Tunnel,
    /// Send directly, outside the VPN (an explicitly approved exception).
    Direct,
    /// Always drop.
    Block,
}

/// One rule: a destination prefix and its action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    /// Destination prefix.
    pub prefix: Prefix,
    /// What to do with matching traffic.
    pub action: Action,
}

/// Whether the VPN path is currently usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathState {
    /// The VPN path is available.
    Up,
    /// The VPN path is unavailable (not connected, failed, reconnecting).
    Down,
}

/// Final decision for one destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Send through the VPN path.
    Tunnel,
    /// Send directly.
    Direct,
    /// Drop.
    Drop,
}

/// A decision and the rule that produced it (`None` = the mode's default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    /// What to do with the traffic.
    pub verdict: Verdict,
    /// The matching rule's prefix, or `None` if the default applied.
    pub matched: Option<Prefix>,
}

/// Why a policy was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    /// More than [`MAX_RULES`] rules.
    TooManyRules,
    /// Two rules name the same prefix (whether or not their actions agree).
    DuplicatePrefix(Prefix),
    /// A `Direct` rule covers a whole address family (`0.0.0.0/0` or `::/0`),
    /// which would silently switch protection off.
    DirectDefaultRoute(Prefix),
    /// A split-mode policy without any `Tunnel` rule protects nothing.
    NothingProtected,
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyError::TooManyRules => write!(f, "more than {MAX_RULES} rules"),
            PolicyError::DuplicatePrefix(p) => {
                write!(f, "prefix {p} appears in more than one rule")
            }
            PolicyError::DirectDefaultRoute(p) => write!(
                f,
                "direct rule {p} covers every address and would switch protection off"
            ),
            PolicyError::NothingProtected => {
                write!(f, "split mode needs at least one tunnel rule")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

/// A validated policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    mode: Mode,
    /// Sorted by prefix length, longest first, so the first match wins.
    rules: Vec<Rule>,
}

impl Policy {
    /// Validates and builds a policy.
    ///
    /// # Errors
    /// See [`PolicyError`].
    pub fn new(mode: Mode, mut rules: Vec<Rule>) -> Result<Self, PolicyError> {
        if rules.len() > MAX_RULES {
            return Err(PolicyError::TooManyRules);
        }
        for rule in &rules {
            if rule.action == Action::Direct && rule.prefix.prefix_len() == 0 {
                return Err(PolicyError::DirectDefaultRoute(rule.prefix));
            }
        }
        if mode == Mode::Split && !rules.iter().any(|r| r.action == Action::Tunnel) {
            return Err(PolicyError::NothingProtected);
        }
        // Stable, total order: longest prefix first, then family and address,
        // so equal prefixes are adjacent and the result never depends on the
        // order the rules were written in.
        rules.sort_by_key(|r| {
            (
                std::cmp::Reverse(r.prefix.prefix_len()),
                r.prefix.family() == Family::V6,
                r.prefix.addr(),
            )
        });
        if let Some(pair) = rules.windows(2).find(|w| w[0].prefix == w[1].prefix) {
            return Err(PolicyError::DuplicatePrefix(pair[0].prefix));
        }
        Ok(Policy { mode, rules })
    }

    /// The policy's mode.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The rules, most specific first.
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Decides what to do with traffic to `dst` given the VPN path state.
    #[must_use]
    pub fn decide(&self, dst: IpAddr, path: PathState) -> Decision {
        let matched = self.rules.iter().find(|r| r.prefix.contains(dst));
        let action = match (matched, self.mode) {
            (Some(rule), _) => rule.action,
            (None, Mode::FullTunnel) => Action::Tunnel,
            (None, Mode::Split) => Action::Direct,
        };
        let verdict = match (action, path) {
            (Action::Tunnel, PathState::Up) => Verdict::Tunnel,
            (Action::Tunnel, PathState::Down) | (Action::Block, _) => Verdict::Drop,
            (Action::Direct, _) => Verdict::Direct,
        };
        Decision {
            verdict,
            matched: matched.map(|r| r.prefix),
        }
    }
}
