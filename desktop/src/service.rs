//! Talking to `septuvod` without blocking the window.
//!
//! [`Worker`] runs one background thread: it asks for status over the
//! read-only status socket every [`REFRESH`], and performs a deliberate
//! disconnect or a Local Network Access change over the control socket when
//! asked. The window never waits on
//! the service. [`Link`] lets tests use a fake service instead.

use septuvo_cli::watch::{Snapshot, snapshot};
use septuvo_ipc::{self as ipc, ClientError, Command, Outcome};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

/// How often status is refreshed (DEFAULT-UI §9.2: every 1–2 seconds).
pub const REFRESH: Duration = Duration::from_millis(1500);

/// The service, as the window sees it.
pub trait Link {
    /// The latest status (refreshed in the background).
    fn latest(&mut self) -> Snapshot;
    /// Starts a deliberate disconnect. Its result arrives in [`Link::result`].
    fn disconnect(&mut self);
    /// The result of the last disconnect, once, when it has arrived.
    fn result(&mut self) -> Option<Result<(), String>>;
    /// Asks the service to set Local Network Access (N4-04, D-040). Its
    /// result arrives in [`Link::setting_result`]; the value shown is always
    /// the one the service then reports.
    fn set_local_network(&mut self, allowed: bool);
    /// The result of the last setting change, once, when it has arrived.
    fn setting_result(&mut self) -> Option<Result<(), String>>;
}

/// Plain-language reason a disconnect failed.
#[must_use]
pub fn explain(e: &ClientError) -> String {
    match e {
        ClientError::NotRunning => "The Septuvo service is not running.".into(),
        ClientError::PermissionDenied => {
            "You do not have permission to control Septuvo. Ask an administrator to add you to the septuvo group."
                .into()
        }
        other => crate::state::plain(&other.to_string()),
    }
}

/// One deliberate disconnect, synchronously, over the control socket.
///
/// # Errors
/// The service refused, could not be reached, or answered badly.
pub fn disconnect_now(runtime_dir: &Path) -> Result<(), String> {
    control(runtime_dir, Command::Disconnect)
}

/// Sets Local Network Access, synchronously, over the control socket.
///
/// # Errors
/// The service refused, could not be reached, or answered badly.
pub fn set_local_network_now(runtime_dir: &Path, allowed: bool) -> Result<(), String> {
    control(runtime_dir, Command::LocalNetwork(allowed))
}

fn control(runtime_dir: &Path, command: Command) -> Result<(), String> {
    match ipc::request(&ipc::control_socket(runtime_dir), command) {
        Ok(r) => match r.outcome {
            Outcome::Ok => Ok(()),
            Outcome::Err { code, message } => {
                Err(crate::state::plain(&format!("{message} ({code})")))
            }
        },
        Err(e) => Err(explain(&e)),
    }
}

enum Request {
    Disconnect,
    LocalNetwork(bool),
}

/// The real link: a background thread.
pub struct Worker {
    snapshots: Receiver<Snapshot>,
    requests: Sender<Request>,
    results: Receiver<Result<(), String>>,
    settings: Receiver<Result<(), String>>,
    last: Snapshot,
}

impl Worker {
    /// Starts the thread. The first status is fetched before returning, so
    /// the window never shows a guess. `wake` is called after every change
    /// (to repaint the window).
    ///
    /// # Panics
    /// The operating system cannot start a thread (out of resources).
    #[must_use]
    pub fn start(runtime_dir: PathBuf, wake: impl Fn() + Send + 'static) -> Worker {
        let first = snapshot(&runtime_dir);
        let (snap_tx, snapshots) = mpsc::channel();
        let (requests, req_rx) = mpsc::channel();
        let (res_tx, results) = mpsc::channel();
        let (set_tx, settings) = mpsc::channel();
        std::thread::Builder::new()
            .name("septuvo-service".into())
            .spawn(move || {
                loop {
                    match req_rx.recv_timeout(REFRESH) {
                        Ok(Request::Disconnect) => {
                            let r = disconnect_now(&runtime_dir);
                            if res_tx.send(r).is_err() {
                                return;
                            }
                        }
                        Ok(Request::LocalNetwork(allowed)) => {
                            let r = set_local_network_now(&runtime_dir, allowed);
                            if set_tx.send(r).is_err() {
                                return;
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                    if snap_tx.send(snapshot(&runtime_dir)).is_err() {
                        return;
                    }
                    wake();
                }
            })
            .expect("cannot start the service thread");
        Worker {
            snapshots,
            requests,
            results,
            settings,
            last: first,
        }
    }
}

impl Link for Worker {
    fn latest(&mut self) -> Snapshot {
        while let Ok(s) = self.snapshots.try_recv() {
            self.last = s;
        }
        self.last.clone()
    }

    fn disconnect(&mut self) {
        let _ = self.requests.send(Request::Disconnect);
    }

    fn result(&mut self) -> Option<Result<(), String>> {
        self.results.try_recv().ok()
    }

    fn set_local_network(&mut self, allowed: bool) {
        let _ = self.requests.send(Request::LocalNetwork(allowed));
    }

    fn setting_result(&mut self) -> Option<Result<(), String>> {
        self.settings.try_recv().ok()
    }
}

/// A fake service for tests and the design preview.
#[derive(Debug, Clone)]
pub struct Fake {
    /// What status returns.
    pub snapshot: Snapshot,
    /// What a disconnect returns; on success the snapshot becomes `after`.
    pub disconnect_result: Result<(), String>,
    /// The status after a successful disconnect.
    pub after: Snapshot,
    /// Disconnects requested so far.
    pub disconnects: u32,
    /// What a Local Network Access change returns; on success the
    /// snapshot's `local_network` becomes the new value.
    pub setting_outcome: Result<(), String>,
    /// Every request made to the fake service, in order (`status`,
    /// `disconnect`, `local-network allowed`, `local-network not-allowed`),
    /// shared with the test that created it.
    pub log: std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>,
    pending: Option<Result<(), String>>,
    pending_setting: Option<Result<(), String>>,
}

impl Fake {
    /// A fake that always reports `snapshot`; disconnect succeeds and turns
    /// it into `after`.
    #[must_use]
    pub fn new(snapshot: Snapshot, after: Snapshot) -> Fake {
        Fake {
            snapshot,
            disconnect_result: Ok(()),
            after,
            disconnects: 0,
            setting_outcome: Ok(()),
            log: std::sync::Arc::default(),
            pending: None,
            pending_setting: None,
        }
    }
}

impl Fake {
    fn record(&self, request: &'static str) {
        if let Ok(mut log) = self.log.lock() {
            log.push(request);
        }
    }
}

impl Link for Fake {
    fn latest(&mut self) -> Snapshot {
        self.record("status");
        self.snapshot.clone()
    }

    fn disconnect(&mut self) {
        self.record("disconnect");
        self.disconnects += 1;
        if self.disconnect_result.is_ok() {
            self.snapshot = self.after.clone();
        }
        self.pending = Some(self.disconnect_result.clone());
    }

    fn result(&mut self) -> Option<Result<(), String>> {
        self.pending.take()
    }

    fn set_local_network(&mut self, allowed: bool) {
        let value = if allowed { "allowed" } else { "not-allowed" };
        self.record(if allowed {
            "local-network allowed"
        } else {
            "local-network not-allowed"
        });
        if self.setting_outcome.is_ok()
            && let Snapshot::Service(r) = &mut self.snapshot
        {
            match r.fields.iter_mut().find(|(k, _)| k == "local_network") {
                Some((_, v)) => *v = value.to_string(),
                None => r.fields.push(("local_network".into(), value.into())),
            }
        }
        self.pending_setting = Some(self.setting_outcome.clone());
    }

    fn setting_result(&mut self) -> Option<Result<(), String>> {
        self.pending_setting.take()
    }
}
