//! Septuvo route and LAN-exception policy model (N1-02).
//!
//! Decides, for each destination address, whether traffic must use the VPN
//! path, may go directly (an explicitly approved exception), or is dropped —
//! failing closed when the VPN path is down.
//!
//! Pure logic only: no operating-system calls, no networking and no
//! cryptography (track N1, D-029). The routing and firewall rules that enforce
//! these decisions on Linux are generated and tested in N1-03.

pub mod parse;
pub mod policy;
pub mod prefix;

pub use parse::{ParseError, ParseErrorKind, parse_policy};
pub use policy::{Action, Decision, Mode, PathState, Policy, PolicyError, Rule, Verdict};
pub use prefix::{Family, Prefix, PrefixError};
