//! Talking to `septuvod` without blocking the window.
//!
//! [`Worker`] runs one background thread: it asks for status over the
//! read-only status socket every [`REFRESH`], and performs a deliberate
//! disconnect, a Local Network Access change or a saved-connection
//! operation ([`Op`], N4-05) when asked. The window never waits on the
//! service. [`Link`] lets tests use a fake service instead ([`Fake`], which
//! keeps its own small connection store with the service's rules).

use septuvo_cli::watch::{Snapshot, snapshot};
use septuvo_ipc::{self as ipc, ClientError, Command, ConnectionId, Outcome, Request, Response};
use std::fmt::Write as _;
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
    /// Starts a saved-connection operation, a connect or a selection
    /// (N4-05). Its answer arrives in [`Link::replies`].
    fn send(&mut self, op: Op);
    /// The answers that have arrived since the last call, in order.
    fn replies(&mut self) -> Vec<Reply>;
}

/// A request about saved connections, or a connect (N4-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// The names and the selection (`LIST`, any user).
    List,
    /// One connection's configuration with its name (`GET`).
    Get(ConnectionId),
    /// Save a complete configuration (`ADD`; the text carries the name).
    Add(String),
    /// Replace a configuration (`REPLACE`).
    Replace(ConnectionId, String),
    /// Rename (`RENAME`): no network change.
    Rename(ConnectionId, String),
    /// Delete (`DELETE`); refused for the connection in use.
    Delete(ConnectionId),
    /// Select (`SELECT`); while connected this switches without a gap.
    Select(ConnectionId),
    /// Connect the selected connection (`CONNECT`; before G1 the kill
    /// switch only, N4-D12).
    Connect,
}

impl Op {
    /// The request on the wire.
    #[must_use]
    pub fn request(&self) -> Request {
        match self {
            Op::List => Request::new(Command::List),
            Op::Get(id) => Request::new(Command::Get(*id)),
            Op::Add(text) => Request::with_body(Command::Add, text.as_str()),
            Op::Replace(id, text) => Request::with_body(Command::Replace(*id), text.as_str()),
            Op::Rename(id, name) => Request::with_body(Command::Rename(*id), name.as_str()),
            Op::Delete(id) => Request::new(Command::Delete(*id)),
            Op::Select(id) => Request::new(Command::Select(*id)),
            Op::Connect => Request::new(Command::Connect),
        }
    }

    /// A short name, for logs and tests.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Op::List => "list",
            Op::Get(_) => "get",
            Op::Add(_) => "add",
            Op::Replace(..) => "replace",
            Op::Rename(..) => "rename",
            Op::Delete(_) => "delete",
            Op::Select(_) => "select",
            Op::Connect => "connect",
        }
    }
}

/// The service's answer to an [`Op`]: its response, or why it failed in
/// plain language (refused, not running, no permission, bad answer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// What was asked.
    pub op: Op,
    /// The answer.
    pub result: Result<Response, String>,
}

/// Performs one [`Op`], synchronously, on the right socket.
#[must_use]
pub fn perform(runtime_dir: &Path, op: &Op) -> Reply {
    let result = match septuvo_cli::service::call(runtime_dir, &op.request()) {
        Ok(r) => match &r.outcome {
            Outcome::Ok => Ok(r),
            Outcome::Err { message, .. } => Err(crate::state::plain(&sentence(message))),
        },
        Err(e) => Err(explain(&e)),
    };
    Reply {
        op: op.clone(),
        result,
    }
}

/// A service message as a sentence: capitalised, with a full stop.
#[must_use]
pub fn sentence(message: &str) -> String {
    let mut c = message.trim().chars();
    let mut out: String = match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => return "The Septuvo service refused the request.".into(),
    };
    if !out.ends_with(['.', '!', '?']) {
        out.push('.');
    }
    out
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

enum Job {
    Disconnect,
    LocalNetwork(bool),
    Op(Op),
}

/// The real link: a background thread.
pub struct Worker {
    snapshots: Receiver<Snapshot>,
    requests: Sender<Job>,
    results: Receiver<Result<(), String>>,
    settings: Receiver<Result<(), String>>,
    replies: Receiver<Reply>,
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
        let (reply_tx, replies) = mpsc::channel();
        std::thread::Builder::new()
            .name("septuvo-service".into())
            .spawn(move || {
                loop {
                    match req_rx.recv_timeout(REFRESH) {
                        Ok(Job::Disconnect) => {
                            let r = disconnect_now(&runtime_dir);
                            if res_tx.send(r).is_err() {
                                return;
                            }
                        }
                        Ok(Job::LocalNetwork(allowed)) => {
                            let r = set_local_network_now(&runtime_dir, allowed);
                            if set_tx.send(r).is_err() {
                                return;
                            }
                        }
                        Ok(Job::Op(op)) => {
                            if reply_tx.send(perform(&runtime_dir, &op)).is_err() {
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
            replies,
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
        let _ = self.requests.send(Job::Disconnect);
    }

    fn result(&mut self) -> Option<Result<(), String>> {
        self.results.try_recv().ok()
    }

    fn set_local_network(&mut self, allowed: bool) {
        let _ = self.requests.send(Job::LocalNetwork(allowed));
    }

    fn setting_result(&mut self) -> Option<Result<(), String>> {
        self.settings.try_recv().ok()
    }

    fn send(&mut self, op: Op) {
        let _ = self.requests.send(Job::Op(op));
    }

    fn replies(&mut self) -> Vec<Reply> {
        self.replies.try_iter().collect()
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
    /// The fake's saved connections: ID, name and stored text (without the
    /// name), with the service's rules (validated, unique names).
    pub store: Vec<(ConnectionId, String, String)>,
    /// The selected connection.
    pub selected: Option<ConnectionId>,
    /// Operations to refuse, by [`Op::name`], with the service's message.
    pub refuse: Vec<(&'static str, String)>,
    next: u16,
    pending: Option<Result<(), String>>,
    pending_setting: Option<Result<(), String>>,
    queued: Vec<Reply>,
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
            store: Vec::new(),
            selected: None,
            refuse: Vec::new(),
            next: 1,
            pending: None,
            pending_setting: None,
            queued: Vec::new(),
        }
    }

    /// The same fake with saved connections (complete configuration texts
    /// with a `[connection]` name), the first one selected.
    ///
    /// # Panics
    /// A text is not a valid named configuration (a mistake in a test).
    #[must_use]
    pub fn with_connections(mut self, texts: &[&str]) -> Fake {
        for t in texts {
            let r = self.op(&Op::Add((*t).to_string()));
            assert!(r.is_ok(), "fixture: {r:?}");
        }
        if let Some((id, _, _)) = self.store.first() {
            let id = *id;
            self.selected = Some(id);
            self.set_field("connection", &id.to_string());
        }
        self
    }

    fn set_field(&mut self, key: &str, value: &str) {
        for snap in [&mut self.snapshot, &mut self.after] {
            if let Snapshot::Service(r) = snap {
                match r.fields.iter_mut().find(|(k, _)| k == key) {
                    Some((_, v)) => *v = value.to_string(),
                    None => r.fields.push((key.into(), value.into())),
                }
            }
        }
    }

    fn connected(&self) -> bool {
        matches!(&self.snapshot, Snapshot::Service(r) if r.field("kill_switch") == Some("on"))
    }

    fn named(&self, text: &str, keep: Option<ConnectionId>) -> Result<(String, String), String> {
        let (config, stored) = septuvo_config::split_connection(text)
            .map_err(|e| sentence(&format!("invalid configuration: {e}")))?;
        let name = match (config.name, keep) {
            (Some(n), _) => n,
            (None, Some(id)) => self
                .store
                .iter()
                .find(|(i, _, _)| *i == id)
                .map(|(_, n, _)| n.clone())
                .ok_or("No such connection.")?,
            (None, None) => return Err("A saved connection needs a name.".into()),
        };
        let lower = name.to_lowercase();
        if self
            .store
            .iter()
            .any(|(i, n, _)| Some(*i) != keep && n.to_lowercase() == lower)
        {
            return Err("A connection with this name already exists.".into());
        }
        Ok((name, stored))
    }

    fn index(&self, id: ConnectionId) -> Result<usize, String> {
        self.store
            .iter()
            .position(|(i, _, _)| *i == id)
            .ok_or_else(|| "No such connection.".into())
    }

    /// What the fake service answers.
    fn op(&mut self, op: &Op) -> Result<Response, String> {
        if let Some((_, why)) = self.refuse.iter().find(|(n, _)| *n == op.name()) {
            return Err(why.clone());
        }
        match op {
            Op::List => {
                let mut body = String::new();
                for (i, n, _) in &self.store {
                    let _ = writeln!(body, "{i}\t{n}");
                }
                Ok(Response::ok()
                    .with(
                        "selected",
                        &self.selected.map_or("none".into(), |i| i.to_string()),
                    )
                    .with_body(&body))
            }
            Op::Get(id) => {
                let (_, n, t) = &self.store[self.index(*id)?];
                Ok(Response::ok().with_body(&format!("{t}{}", septuvo_config::name_table(n))))
            }
            Op::Add(text) => {
                let (name, stored) = self.named(text, None)?;
                let id = ConnectionId::new(self.next).ok_or("No IDs left.")?;
                self.next += 1;
                self.store.push((id, name, stored));
                Ok(Response::ok().with("id", &id.to_string()))
            }
            Op::Replace(id, text) => {
                let i = self.index(*id)?;
                let (name, stored) = self.named(text, Some(*id))?;
                self.store[i] = (*id, name, stored);
                Ok(Response::ok())
            }
            Op::Rename(id, name) => {
                let i = self.index(*id)?;
                let name = septuvo_config::connection_name(name).map_err(|e| sentence(&e))?;
                let lower = name.to_lowercase();
                if self
                    .store
                    .iter()
                    .any(|(o, n, _)| o != id && n.to_lowercase() == lower)
                {
                    return Err("A connection with this name already exists.".into());
                }
                self.store[i].1 = name;
                Ok(Response::ok())
            }
            Op::Delete(id) => {
                let i = self.index(*id)?;
                if self.selected == Some(*id) && self.connected() {
                    return Err("This connection is in use: disconnect before deleting it.".into());
                }
                self.store.remove(i);
                if self.selected == Some(*id) {
                    self.selected = self
                        .store
                        .get(i.min(self.store.len().saturating_sub(1)))
                        .map(|c| c.0);
                    let v = self.selected.map_or("default".into(), |i| i.to_string());
                    self.set_field("connection", &v);
                }
                Ok(Response::ok())
            }
            Op::Select(id) => {
                self.index(*id)?;
                self.selected = Some(*id);
                self.set_field("connection", &id.to_string());
                Ok(Response::ok())
            }
            Op::Connect => {
                for (k, v) in [("state", "blocking"), ("kill_switch", "on")] {
                    if let Snapshot::Service(r) = &mut self.snapshot {
                        match r.fields.iter_mut().find(|(key, _)| key == k) {
                            Some((_, val)) => *val = v.to_string(),
                            None => r.fields.push((k.into(), v.into())),
                        }
                    }
                }
                Ok(Response::ok())
            }
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

    fn send(&mut self, op: Op) {
        self.record(op.name());
        let result = self.op(&op);
        self.queued.push(Reply { op, result });
    }

    fn replies(&mut self) -> Vec<Reply> {
        std::mem::take(&mut self.queued)
    }
}
