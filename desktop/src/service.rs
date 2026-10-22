//! Talking to `septuvod` without blocking the window.
//!
//! [`Worker`] runs one background thread: it asks for status over the
//! read-only status socket every [`REFRESH`], and performs a deliberate
//! disconnect over the control socket when asked. The window never waits on
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
    match ipc::request(&ipc::control_socket(runtime_dir), Command::Disconnect) {
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
}

/// The real link: a background thread.
pub struct Worker {
    snapshots: Receiver<Snapshot>,
    requests: Sender<Request>,
    results: Receiver<Result<(), String>>,
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
    pending: Option<Result<(), String>>,
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
            pending: None,
        }
    }
}

impl Link for Fake {
    fn latest(&mut self) -> Snapshot {
        self.snapshot.clone()
    }

    fn disconnect(&mut self) {
        self.disconnects += 1;
        if self.disconnect_result.is_ok() {
            self.snapshot = self.after.clone();
        }
        self.pending = Some(self.disconnect_result.clone());
    }

    fn result(&mut self) -> Option<Result<(), String>> {
        self.pending.take()
    }
}
