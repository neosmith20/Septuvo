//! Four-stage connection status model (D-020, N1-04).
//!
//! A VPN can look "up" while protecting nothing: an adapter can exist before
//! any peer is authenticated, a peer can be authenticated while no traffic
//! gets through, and traffic can pass while DNS is broken or leaking. Each
//! stage is therefore checked and shown on its own, and the overall state is
//! **Connected** only when every stage has passed, in order. Anything else,
//! including a later stage passing while an earlier one has not, is shown as
//! not connected. Data only: nothing here touches the network.

use std::fmt::{self, Write as _};

/// The four independent stages, in the order they must pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// The tunnel interface exists and is up.
    Adapter,
    /// The Septuvo Server proved the identity pinned at enrollment.
    PeerAuthenticated,
    /// A probe through the tunnel reached the server side.
    Reachability,
    /// A DNS lookup through the tunnel resolver succeeded.
    Dns,
}

impl Stage {
    /// All stages in order.
    pub const ALL: [Stage; 4] = [
        Stage::Adapter,
        Stage::PeerAuthenticated,
        Stage::Reachability,
        Stage::Dns,
    ];

    /// Label shown to people.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Stage::Adapter => "Tunnel adapter",
            Stage::PeerAuthenticated => "Server identity verified",
            Stage::Reachability => "Traffic through tunnel",
            Stage::Dns => "DNS through tunnel",
        }
    }
}

/// Result of checking one stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// Not checked, or the result is not available.
    Unknown,
    /// Checked and failed.
    Fail,
    /// Checked and passed.
    Pass,
}

impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Check::Unknown => "unknown",
            Check::Fail => "failed",
            Check::Pass => "ok",
        })
    }
}

/// Status of every stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionStatus {
    /// Tunnel adapter.
    pub adapter: Check,
    /// Server identity verified.
    pub peer: Check,
    /// Traffic through the tunnel.
    pub reachability: Check,
    /// DNS through the tunnel.
    pub dns: Check,
}

/// Overall state derived from the stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overall {
    /// Every stage passed, in order.
    Connected,
    /// The first stage that has not passed.
    NotConnected(Stage),
    /// A later stage reports success while an earlier one has not passed.
    /// Never trusted: treated as not connected.
    Inconsistent(Stage),
}

impl ConnectionStatus {
    /// Nothing known (for example: no Septuvo service running).
    pub const UNKNOWN: ConnectionStatus = ConnectionStatus {
        adapter: Check::Unknown,
        peer: Check::Unknown,
        reachability: Check::Unknown,
        dns: Check::Unknown,
    };

    /// The check for one stage.
    #[must_use]
    pub fn get(&self, stage: Stage) -> Check {
        match stage {
            Stage::Adapter => self.adapter,
            Stage::PeerAuthenticated => self.peer,
            Stage::Reachability => self.reachability,
            Stage::Dns => self.dns,
        }
    }

    /// Overall state. Connected only if all four stages passed.
    #[must_use]
    pub fn overall(&self) -> Overall {
        let first_missing = Stage::ALL.into_iter().find(|&s| self.get(s) != Check::Pass);
        match first_missing {
            None => Overall::Connected,
            Some(missing) => {
                let later_pass = Stage::ALL
                    .into_iter()
                    .skip_while(|&s| s != missing)
                    .any(|s| self.get(s) == Check::Pass);
                if later_pass {
                    Overall::Inconsistent(missing)
                } else {
                    Overall::NotConnected(missing)
                }
            }
        }
    }

    /// True only when every stage passed.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.overall() == Overall::Connected
    }

    /// Human-readable report, one line per stage plus a summary.
    #[must_use]
    pub fn report(&self, note: Option<&str>) -> String {
        let mut s = String::new();
        for stage in Stage::ALL {
            let _ = writeln!(s, "{:<26} {}", stage.label(), self.get(stage));
        }
        let summary = match self.overall() {
            Overall::Connected => "Connected: all four checks passed".to_string(),
            Overall::NotConnected(stage) => {
                format!("Not connected: \"{}\" has not passed", stage.label())
            }
            Overall::Inconsistent(stage) => format!(
                "Not connected: \"{}\" has not passed, so later results are not trusted",
                stage.label()
            ),
        };
        let _ = write!(s, "\nStatus: {summary}\n");
        if let Some(note) = note {
            let _ = writeln!(s, "{note}");
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATES: [Check; 3] = [Check::Unknown, Check::Fail, Check::Pass];

    fn all_combinations() -> Vec<ConnectionStatus> {
        let mut v = Vec::new();
        for adapter in STATES {
            for peer in STATES {
                for reachability in STATES {
                    for dns in STATES {
                        v.push(ConnectionStatus {
                            adapter,
                            peer,
                            reachability,
                            dns,
                        });
                    }
                }
            }
        }
        v
    }

    #[test]
    fn adapter_up_never_means_connected() {
        let s = ConnectionStatus {
            adapter: Check::Pass,
            ..ConnectionStatus::UNKNOWN
        };
        assert_eq!(s.overall(), Overall::NotConnected(Stage::PeerAuthenticated));
        assert!(!s.report(None).contains("Status: Connected"));
    }

    #[test]
    fn connected_only_when_all_four_pass_exhaustively() {
        let combos = all_combinations();
        assert_eq!(combos.len(), 81);
        for s in combos {
            let all_pass = Stage::ALL.iter().all(|&st| s.get(st) == Check::Pass);
            assert_eq!(s.is_connected(), all_pass, "{s:?}");
            assert_eq!(
                s.report(None).contains("Status: Connected"),
                all_pass,
                "{s:?}"
            );
        }
    }

    #[test]
    fn later_success_without_earlier_success_is_inconsistent() {
        let s = ConnectionStatus {
            adapter: Check::Pass,
            peer: Check::Unknown,
            reachability: Check::Pass,
            dns: Check::Pass,
        };
        assert_eq!(s.overall(), Overall::Inconsistent(Stage::PeerAuthenticated));
        assert!(s.report(None).contains("not trusted"));
    }

    #[test]
    fn first_failing_stage_is_reported() {
        let s = ConnectionStatus {
            adapter: Check::Pass,
            peer: Check::Pass,
            reachability: Check::Pass,
            dns: Check::Fail,
        };
        assert_eq!(s.overall(), Overall::NotConnected(Stage::Dns));
        assert!(
            s.report(None)
                .contains("\"DNS through tunnel\" has not passed")
        );
    }

    #[test]
    fn unknown_is_never_connected() {
        assert_eq!(
            ConnectionStatus::UNKNOWN.overall(),
            Overall::NotConnected(Stage::Adapter)
        );
    }
}
