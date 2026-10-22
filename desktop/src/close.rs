//! The window's X (DEFAULT-UI §3.7, D-041), as a small state machine.
//!
//! - **Exit and Disconnect** (the default): if the service has anything
//!   active, ask it for a deliberate disconnect and exit **only after it
//!   confirms**. Nothing active: exit at once.
//! - **Failure** (refused, not permitted, service not answering): the
//!   window stays, with `Could not disconnect safely.` and the reason.
//!   Protection is left exactly as it is. Closing again exits the app
//!   only; the service keeps its state (the GUI ending never changes
//!   network state, D-041).
//! - **Send to System Tray** needs the tray (N4-06); until then the window
//!   always uses Exit and Disconnect.
//!
//! Minimize and maximize are the operating system's and never reach this.

use septuvo_cli::watch::Snapshot;

use crate::state::needs_disconnect;

/// What the window should do now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Let the window close and the app exit.
    Exit,
    /// Keep the window open (cancel the close).
    Stay,
    /// Keep the window open and ask the service for a deliberate disconnect.
    StayAndDisconnect,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Idle,
    Disconnecting,
    Failed,
}

/// The close controller.
#[derive(Debug, Clone, Default)]
pub struct Close {
    phase: Phase,
    /// Shown in the window while set.
    pub notice: Option<String>,
}

/// The notice prefix required by DEFAULT-UI §3.7.
pub const COULD_NOT: &str = "Could not disconnect safely.";

impl Close {
    /// True while waiting for the service to confirm the disconnect.
    #[must_use]
    pub fn disconnecting(&self) -> bool {
        self.phase == Phase::Disconnecting
    }

    /// The user clicked X (or pressed Alt+F4).
    pub fn requested(&mut self, service: &Snapshot) -> Step {
        match self.phase {
            Phase::Disconnecting => Step::Stay,
            // A second close after a failure: the user chose to leave the
            // service as it is. Only the GUI exits.
            Phase::Failed => Step::Exit,
            Phase::Idle => match service {
                Snapshot::Lost(_) => {
                    self.phase = Phase::Failed;
                    self.notice = Some(format!(
                        "{COULD_NOT} The Septuvo service is not responding. Open Help & Diagnostics, \
                         or close again to exit; protection stays exactly as it is."
                    ));
                    Step::Stay
                }
                Snapshot::Service(r) if needs_disconnect(r) => {
                    self.phase = Phase::Disconnecting;
                    self.notice = None;
                    Step::StayAndDisconnect
                }
                Snapshot::Service(_) => Step::Exit,
            },
        }
    }

    /// The service answered the disconnect.
    pub fn result(&mut self, r: Result<(), String>) -> Step {
        if self.phase != Phase::Disconnecting {
            return Step::Stay;
        }
        match r {
            Ok(()) => {
                self.phase = Phase::Idle;
                Step::Exit
            }
            Err(why) => {
                self.phase = Phase::Failed;
                self.notice = Some(format!(
                    "{COULD_NOT} {why} Open Help & Diagnostics, or close again to exit; protection stays exactly as it is."
                ));
                Step::Stay
            }
        }
    }

    /// Any other action clears a failed close (for example the user retried
    /// Disconnect with the button).
    pub fn reset(&mut self) {
        if self.phase == Phase::Failed {
            self.phase = Phase::Idle;
            self.notice = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::samples;

    #[test]
    fn nothing_active_exits_at_once() {
        let mut c = Close::default();
        assert_eq!(
            c.requested(&Snapshot::Service(samples::disconnected())),
            Step::Exit
        );
    }

    #[test]
    fn active_session_disconnects_then_exits_only_on_confirmation() {
        let mut c = Close::default();
        let s = Snapshot::Service(samples::blocking());
        assert_eq!(c.requested(&s), Step::StayAndDisconnect);
        assert!(c.disconnecting());
        assert_eq!(
            c.requested(&s),
            Step::Stay,
            "a second X while waiting does nothing"
        );
        assert_eq!(c.result(Ok(())), Step::Exit);
    }

    #[test]
    fn a_failed_disconnect_keeps_the_window_and_says_why() {
        let mut c = Close::default();
        c.requested(&Snapshot::Service(samples::blocking()));
        assert_eq!(c.result(Err("Permission denied.".into())), Step::Stay);
        let n = c.notice.clone().unwrap();
        assert!(
            n.starts_with(COULD_NOT) && n.contains("Permission denied."),
            "{n}"
        );
        assert_eq!(
            c.requested(&Snapshot::Service(samples::blocking())),
            Step::Exit
        );
    }

    #[test]
    fn a_lost_service_never_exits_silently() {
        let mut c = Close::default();
        assert_eq!(c.requested(&Snapshot::Lost("gone".into())), Step::Stay);
        assert!(c.notice.as_deref().unwrap().starts_with(COULD_NOT));
        assert_eq!(c.requested(&Snapshot::Lost("gone".into())), Step::Exit);
        let mut c = Close::default();
        c.requested(&Snapshot::Lost("gone".into()));
        c.reset();
        assert!(c.notice.is_none());
        assert_eq!(
            c.requested(&Snapshot::Service(samples::disconnected())),
            Step::Exit
        );
    }
}
