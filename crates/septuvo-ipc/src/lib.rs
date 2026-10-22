//! Septuvo local control interface (N2-02, N4-03).
//!
//! The small, versioned text protocol between unprivileged clients (the
//! `septuvo` command-line tool, the desktop app) and the privileged service
//! `septuvod`, over Unix sockets on the same machine. It carries **no
//! secrets and no traffic**: only commands, status fields and, for saved
//! connections, their non-secret configuration text.
//!
//! ```text
//! request:   SEPTUVO/1 STATUS\n
//! response:  SEPTUVO/1 OK\n
//!            adapter=fail\n
//!            ...
//!            END\n
//! error:     SEPTUVO/1 ERR not_permitted control commands need the control socket\n
//!            END\n
//! ```
//!
//! **Bodies (N4-03).** Requests that carry a saved connection or a name
//! (`ADD`, `REPLACE`, `RENAME`) and responses that return one (`GET`,
//! `LIST`) add a length-prefixed body of at most [`MAX_BODY_BYTES`]: the
//! protocol's one exception to the 64-byte request limit. The length is
//! sent first, so neither side reads more than it was told to expect:
//!
//! ```text
//! request:   SEPTUVO/1 RENAME c0002 8\n          (header: at most 64 bytes)
//!            Home VPN                             (exactly 8 bytes of UTF-8)
//! response:  SEPTUVO/1 OK\n
//!            selected=c0002\n
//!            BODY 21\n
//!            c0002\tHome VPN\n ...              (exactly 21 bytes)
//!            END\n
//! ```
//!
//! Both sides treat the other as hostile (ADV-10): every message has a size
//! limit, header and fields are restricted to printable ASCII, a body is
//! UTF-8 text without control characters other than tab and line breaks,
//! and anything malformed is rejected rather than repaired.
//!
//! Licensed under Apache-2.0: it is the open interface the Apache-2.0
//! `septuvo` tool depends on (N2 work order, D-033).

use std::fmt;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Protocol name and version, the first word of every message.
pub const VERSION: &str = "SEPTUVO/1";
/// Largest request header (or body-less request), in bytes, including the
/// line feed.
pub const MAX_REQUEST_BYTES: usize = 64;
/// Largest response without its body, in bytes.
pub const MAX_RESPONSE_BYTES: usize = 4096;
/// Largest body of a request or response, in bytes (N4-03).
pub const MAX_BODY_BYTES: usize = 64 * 1024;
/// Most fields in one response.
pub const MAX_FIELDS: usize = 32;
/// Longest field key, error code or command, in bytes.
pub const MAX_KEY_BYTES: usize = 32;
/// Longest field value or error message, in bytes.
pub const MAX_VALUE_BYTES: usize = 200;
/// How long either side waits for the other before giving up.
pub const TIMEOUT: Duration = Duration::from_secs(5);

/// Default directory holding the service's sockets.
pub const DEFAULT_RUNTIME_DIR: &str = "/run/septuvo";
/// Status socket, inside the runtime directory: any local user may connect.
pub const STATUS_SOCKET: &str = "status.sock";
/// Control directory and socket, inside the runtime directory: only members
/// of the service's control group may enter the directory.
pub const CONTROL_DIR: &str = "control";
/// Control socket name, inside [`CONTROL_DIR`].
pub const CONTROL_SOCKET: &str = "control.sock";

/// Path of the status socket under `runtime_dir`.
#[must_use]
pub fn status_socket(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join(STATUS_SOCKET)
}

/// Path of the control socket under `runtime_dir`.
#[must_use]
pub fn control_socket(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join(CONTROL_DIR).join(CONTROL_SOCKET)
}

/// Longest captive-portal window a client may ask for, in minutes (Q1).
pub const MAX_CAPTIVE_MINUTES: u8 = 5;

/// A saved connection's ID, `c0001` to `c9999` on the wire. The service
/// assigns it; it never changes and is never reused while the connection
/// exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionId(u16);

impl ConnectionId {
    /// The largest ID.
    pub const MAX: u16 = 9999;

    /// An ID from its number, 1 to [`ConnectionId::MAX`].
    #[must_use]
    pub fn new(n: u16) -> Option<ConnectionId> {
        (1..=Self::MAX).contains(&n).then_some(ConnectionId(n))
    }

    /// Its number.
    #[must_use]
    pub fn number(self) -> u16 {
        self.0
    }

    /// Parses exactly `c` and four digits (`c0001`), nothing else.
    #[must_use]
    pub fn parse(text: &str) -> Option<ConnectionId> {
        let digits = text.strip_prefix('c')?;
        if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        ConnectionId::new(digits.parse().ok()?)
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "c{:04}", self.0)
    }
}

/// A command from a client to the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Report status. Allowed on both sockets.
    Status,
    /// Turn protection on (kill switch; no tunnel before G1).
    Connect,
    /// Deliberately disconnect: the only way protection is lifted (Q2).
    Disconnect,
    /// Open a captive-portal window for the dedicated helper account, 1 to
    /// [`MAX_CAPTIVE_MINUTES`] minutes (Q1, N3-02). On the wire: `CAPTIVE N`,
    /// or `CAPTIVE N NETWORK-DNS` to also authorize the network's own DNS for
    /// the helper during this one window.
    Captive {
        /// Length of the window, 1 to [`MAX_CAPTIVE_MINUTES`].
        minutes: u8,
        /// Explicitly authorized: the helper (only) may query the network's
        /// resolvers during this window. Never automatic.
        network_dns: bool,
    },
    /// List saved connections (body: `ID<TAB>name` lines). Read-only:
    /// allowed on both sockets, so every user can see the names.
    List,
    /// One saved connection's configuration text, with its name (body).
    Get(ConnectionId),
    /// Save a new complete connection (body: its configuration text).
    Add,
    /// Replace a saved connection's configuration (body: the new text).
    Replace(ConnectionId),
    /// Rename a saved connection (body: the new name, UTF-8).
    Rename(ConnectionId),
    /// Delete a saved connection.
    Delete(ConnectionId),
    /// Select a connection; while connected, switch to it (D-041).
    Select(ConnectionId),
}

impl Command {
    /// The command's word on the wire.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Command::Status => "STATUS",
            Command::Connect => "CONNECT",
            Command::Disconnect => "DISCONNECT",
            Command::Captive { .. } => "CAPTIVE",
            Command::List => "LIST",
            Command::Get(_) => "GET",
            Command::Add => "ADD",
            Command::Replace(_) => "REPLACE",
            Command::Rename(_) => "RENAME",
            Command::Delete(_) => "DELETE",
            Command::Select(_) => "SELECT",
        }
    }

    /// True if the command changes anything or reveals a connection's
    /// configuration, and so needs the control socket. Status and the list
    /// of names are open to every local user.
    #[must_use]
    pub fn needs_control(self) -> bool {
        !matches!(self, Command::Status | Command::List)
    }

    /// True if the request carries a body.
    #[must_use]
    pub fn has_body(self) -> bool {
        matches!(
            self,
            Command::Add | Command::Replace(_) | Command::Rename(_)
        )
    }

    /// The request line for this command, for a body of `body_len` bytes
    /// when [`Command::has_body`].
    fn header(self, body_len: usize) -> String {
        match self {
            Command::Captive {
                minutes,
                network_dns: false,
            } => format!("{VERSION} CAPTIVE {minutes}\n"),
            Command::Captive {
                minutes,
                network_dns: true,
            } => format!("{VERSION} CAPTIVE {minutes} NETWORK-DNS\n"),
            Command::Get(id) | Command::Delete(id) | Command::Select(id) => {
                format!("{VERSION} {} {id}\n", self.as_str())
            }
            Command::Add => format!("{VERSION} ADD {body_len}\n"),
            Command::Replace(id) | Command::Rename(id) => {
                format!("{VERSION} {} {id} {body_len}\n", self.as_str())
            }
            other => format!("{VERSION} {}\n", other.as_str()),
        }
    }

    /// The request for a command without a body.
    ///
    /// # Panics
    /// The command needs a body (use [`Request::with_body`]).
    #[must_use]
    pub fn encode(self) -> String {
        assert!(!self.has_body(), "{} needs a body", self.as_str());
        self.header(0)
    }
}

/// A complete request: a command and, for `ADD`, `REPLACE` and `RENAME`,
/// its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The command.
    pub command: Command,
    /// The body, present exactly when [`Command::has_body`].
    pub body: Option<String>,
}

impl Request {
    /// A request without a body.
    #[must_use]
    pub fn new(command: Command) -> Request {
        Request {
            command,
            body: None,
        }
    }

    /// A request with a body.
    #[must_use]
    pub fn with_body(command: Command, body: impl Into<String>) -> Request {
        Request {
            command,
            body: Some(body.into()),
        }
    }

    /// The request as sent on the wire.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let body = self.body.as_deref().unwrap_or("");
        let mut out = self.command.header(body.len()).into_bytes();
        out.extend_from_slice(body.as_bytes());
        out
    }
}

/// Why a message was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    /// Larger than the limit for its kind.
    TooLarge,
    /// Not ended by a line feed (or by `END` for responses).
    Truncated,
    /// Bytes outside printable ASCII, or an empty or overlong field.
    InvalidCharacters,
    /// A different protocol or version.
    WrongVersion,
    /// A command this version does not define.
    UnknownCommand,
    /// Any other structural problem.
    Malformed,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ProtocolError::TooLarge => "message too large",
            ProtocolError::Truncated => "message incomplete",
            ProtocolError::InvalidCharacters => "message contains invalid characters",
            ProtocolError::WrongVersion => "different protocol version",
            ProtocolError::UnknownCommand => "unknown command",
            ProtocolError::Malformed => "malformed message",
        })
    }
}

impl std::error::Error for ProtocolError {}

/// Printable ASCII, space included.
fn printable(s: &str) -> bool {
    s.bytes().all(|b| (0x20..=0x7e).contains(&b))
}

/// A key, command or error code: `a-z`, `0-9`, `_`, 1 to 32 bytes.
fn is_token(s: &str) -> bool {
    (1..=MAX_KEY_BYTES).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Body text: no control characters except tab, line feed and carriage
/// return (configuration files may use CRLF).
fn body_char(c: char) -> bool {
    !c.is_control() || matches!(c, '\t' | '\n' | '\r')
}

/// A body length: decimal, no sign or leading zero, up to
/// [`MAX_BODY_BYTES`]. A response body may be empty (`LIST` with no saved
/// connections); a request body may not.
fn body_len(text: &str) -> Result<usize, ProtocolError> {
    if text == "0" {
        return Ok(0);
    }
    if text.is_empty()
        || text.len() > 5
        || text.starts_with('0')
        || !text.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(ProtocolError::Malformed);
    }
    match text.parse::<usize>() {
        Ok(n) if n <= MAX_BODY_BYTES => Ok(n),
        Ok(_) => Err(ProtocolError::TooLarge),
        Err(_) => Err(ProtocolError::Malformed),
    }
}

fn request_body_len(text: &str) -> Result<usize, ProtocolError> {
    match body_len(text)? {
        0 => Err(ProtocolError::Malformed),
        n => Ok(n),
    }
}

fn id(text: &str) -> Result<ConnectionId, ProtocolError> {
    ConnectionId::parse(text).ok_or(ProtocolError::Malformed)
}

/// Parses a request line (without its line feed): the command and, for a
/// command with a body, the body's length.
fn parse_header(line: &str) -> Result<(Command, usize), ProtocolError> {
    if !printable(line) {
        return Err(ProtocolError::InvalidCharacters);
    }
    let Some((version, command)) = line.split_once(' ') else {
        return Err(ProtocolError::Malformed);
    };
    if version != VERSION {
        return Err(ProtocolError::WrongVersion);
    }
    let words: Vec<&str> = command.split(' ').collect();
    match words.as_slice() {
        ["STATUS"] => Ok((Command::Status, 0)),
        ["CONNECT"] => Ok((Command::Connect, 0)),
        ["DISCONNECT"] => Ok((Command::Disconnect, 0)),
        ["LIST"] => Ok((Command::List, 0)),
        ["GET", c] => Ok((Command::Get(id(c)?), 0)),
        ["DELETE", c] => Ok((Command::Delete(id(c)?), 0)),
        ["SELECT", c] => Ok((Command::Select(id(c)?), 0)),
        ["ADD", n] => Ok((Command::Add, request_body_len(n)?)),
        ["REPLACE", c, n] => Ok((Command::Replace(id(c)?), request_body_len(n)?)),
        ["RENAME", c, n] => Ok((Command::Rename(id(c)?), request_body_len(n)?)),
        ["CAPTIVE", n] => captive_minutes(n).map(|minutes| {
            (
                Command::Captive {
                    minutes,
                    network_dns: false,
                },
                0,
            )
        }),
        ["CAPTIVE", n, "NETWORK-DNS"] => captive_minutes(n).map(|minutes| {
            (
                Command::Captive {
                    minutes,
                    network_dns: true,
                },
                0,
            )
        }),
        [
            "CAPTIVE" | "GET" | "DELETE" | "SELECT" | "ADD" | "REPLACE" | "RENAME",
            _,
            ..,
        ] => Err(ProtocolError::Malformed),
        _ => Err(ProtocolError::UnknownCommand),
    }
}

/// Validates body bytes as text.
fn body_text(bytes: &[u8]) -> Result<String, ProtocolError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ProtocolError::InvalidCharacters)?;
    if text.chars().all(body_char) {
        Ok(text.to_string())
    } else {
        Err(ProtocolError::InvalidCharacters)
    }
}

/// Parses one complete request: the header line and, if the command has
/// one, a body of exactly the announced length.
///
/// # Errors
/// See [`ProtocolError`].
pub fn parse_framed(bytes: &[u8]) -> Result<Request, ProtocolError> {
    let window = &bytes[..bytes.len().min(MAX_REQUEST_BYTES)];
    let Some(end) = window.iter().position(|&b| b == b'\n') else {
        return Err(if bytes.len() > MAX_REQUEST_BYTES {
            ProtocolError::TooLarge
        } else {
            ProtocolError::Truncated
        });
    };
    let line = std::str::from_utf8(&bytes[..end]).map_err(|_| ProtocolError::InvalidCharacters)?;
    let (command, len) = parse_header(line)?;
    let rest = &bytes[end + 1..];
    if !command.has_body() {
        // Nothing may follow a request without a body.
        return if rest.is_empty() {
            Ok(Request::new(command))
        } else {
            Err(ProtocolError::Malformed)
        };
    }
    match rest.len().cmp(&len) {
        std::cmp::Ordering::Less => Err(ProtocolError::Truncated),
        std::cmp::Ordering::Greater => Err(ProtocolError::Malformed),
        std::cmp::Ordering::Equal => Ok(Request {
            command,
            body: Some(body_text(rest)?),
        }),
    }
}

/// Parses one request without a body.
///
/// # Errors
/// See [`ProtocolError`]; a command that needs a body is
/// [`ProtocolError::Truncated`] or [`ProtocolError::Malformed`] here.
pub fn parse_request(bytes: &[u8]) -> Result<Command, ProtocolError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    let request = parse_framed(bytes)?;
    if request.body.is_some() {
        return Err(ProtocolError::Malformed);
    }
    Ok(request.command)
}

/// Exactly one digit, 1 to [`MAX_CAPTIVE_MINUTES`]: no sign, no leading
/// zero, no spaces.
fn captive_minutes(n: &str) -> Result<u8, ProtocolError> {
    match n.as_bytes() {
        [d @ b'1'..=b'9'] if d - b'0' <= MAX_CAPTIVE_MINUTES => Ok(d - b'0'),
        _ => Err(ProtocolError::Malformed),
    }
}

/// The result line of a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The command succeeded.
    Ok,
    /// The command failed.
    Err {
        /// Machine-readable code, for example `not_permitted`.
        code: String,
        /// Human-readable message.
        message: String,
    },
}

/// A response from the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// Success or failure.
    pub outcome: Outcome,
    /// `key=value` fields, in order.
    pub fields: Vec<(String, String)>,
    /// Text returned by `LIST` and `GET` (N4-03).
    pub body: Option<String>,
}

/// Makes `text` safe to send: printable ASCII only (others become `?`),
/// at most [`MAX_VALUE_BYTES`] bytes.
fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if (' '..='~').contains(&c) { c } else { '?' })
        .take(MAX_VALUE_BYTES)
        .collect()
}

impl Response {
    /// A successful response with no fields.
    #[must_use]
    pub fn ok() -> Self {
        Response {
            outcome: Outcome::Ok,
            fields: Vec::new(),
            body: None,
        }
    }

    /// A failure. An invalid `code` becomes `failed`; the message is cleaned.
    #[must_use]
    pub fn error(code: &str, message: &str) -> Self {
        Response {
            outcome: Outcome::Err {
                code: if is_token(code) { code } else { "failed" }.to_string(),
                message: clean(message),
            },
            fields: Vec::new(),
            body: None,
        }
    }

    /// Adds a field. Invalid keys, fields beyond [`MAX_FIELDS`] and fields
    /// that would make the encoded response larger than
    /// [`MAX_RESPONSE_BYTES`] are dropped; values are cleaned. The result
    /// therefore always encodes to a response the client accepts. A dropped
    /// field reads as missing, which the client shows as unknown, never as
    /// passed (found by the N3-03 fuzzing).
    #[must_use]
    pub fn with(mut self, key: &str, value: &str) -> Self {
        if is_token(key) && self.fields.len() < MAX_FIELDS {
            let value = clean(value);
            let added = key.len() + 1 + value.len() + 1;
            // Room for a `BODY n` line is always kept.
            if self.encoded_len() + added + BODY_LINE <= MAX_RESPONSE_BYTES {
                self.fields.push((key.to_string(), value));
            }
        }
        self
    }

    /// Sets the body. Characters a body may not contain become `?`, and
    /// text beyond [`MAX_BODY_BYTES`] is cut at a character boundary, so the
    /// result always encodes to a response the client accepts.
    #[must_use]
    pub fn with_body(mut self, text: &str) -> Self {
        let mut body = String::new();
        for c in text.chars().map(|c| if body_char(c) { c } else { '?' }) {
            if body.len() + c.len_utf8() > MAX_BODY_BYTES {
                break;
            }
            body.push(c);
        }
        self.body = Some(body);
        self
    }

    /// Length of [`Response::encode`] without the body, without building it.
    fn encoded_len(&self) -> usize {
        let first = match &self.outcome {
            Outcome::Ok => VERSION.len() + 4,
            Outcome::Err { code, message } if message.is_empty() => {
                VERSION.len() + 5 + code.len() + 1
            }
            Outcome::Err { code, message } => {
                VERSION.len() + 5 + code.len() + 1 + message.len() + 1
            }
        };
        let fields: usize = self.fields.iter().map(|(k, v)| k.len() + v.len() + 2).sum();
        first + fields + "END\n".len()
    }

    /// [`Response::with`], only if `condition` holds.
    #[must_use]
    pub fn with_if(self, condition: bool, key: &str, value: &str) -> Self {
        if condition {
            self.with(key, value)
        } else {
            self
        }
    }

    /// The value of the first field called `key`.
    #[must_use]
    pub fn field(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The response as sent on the wire.
    #[must_use]
    pub fn encode(&self) -> String {
        let mut s = match &self.outcome {
            Outcome::Ok => format!("{VERSION} OK\n"),
            // An empty message adds no trailing space, so encoding what was
            // parsed never makes it longer (found by the N3-03 fuzzing).
            Outcome::Err { code, message } if message.is_empty() => {
                format!("{VERSION} ERR {code}\n")
            }
            Outcome::Err { code, message } => format!("{VERSION} ERR {code} {message}\n"),
        };
        for (k, v) in &self.fields {
            s.push_str(k);
            s.push('=');
            s.push_str(v);
            s.push('\n');
        }
        if let Some(body) = &self.body {
            s.push_str("BODY ");
            s.push_str(&body.len().to_string());
            s.push('\n');
            s.push_str(body);
        }
        s.push_str("END\n");
        s
    }
}

/// The longest `BODY n` line.
const BODY_LINE: usize = "BODY 65536\n".len();

/// Where a response's header part ends and how long its body is: scans the
/// lines after the first for `END` or `BODY n`. `None`: not complete yet.
/// Errors on anything that can never become a valid response.
fn frame(bytes: &[u8]) -> Result<Option<(usize, Option<usize>)>, ProtocolError> {
    let mut start = 0;
    let mut first = true;
    while let Some(nl) = bytes[start..].iter().position(|&b| b == b'\n') {
        let line = &bytes[start..start + nl];
        let next = start + nl + 1;
        if first && line == b"END" {
            return Err(ProtocolError::Malformed);
        }
        if !first {
            if line == b"END" {
                return Ok(Some((start, None)));
            }
            if let Some(n) = line.strip_prefix(b"BODY ") {
                let n = std::str::from_utf8(n).map_err(|_| ProtocolError::Malformed)?;
                return Ok(Some((start, Some(body_len(n)?))));
            }
        }
        first = false;
        start = next;
        if start > MAX_RESPONSE_BYTES {
            return Err(ProtocolError::TooLarge);
        }
    }
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    Ok(None)
}

/// Total length of a complete response in `bytes`, or `None` if more is
/// needed.
fn response_len(bytes: &[u8]) -> Result<Option<usize>, ProtocolError> {
    Ok(frame(bytes)?.map(|(at, body)| match body {
        None => at + "END\n".len(),
        Some(n) => {
            let header = bytes[at..]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(0, |p| p + 1);
            at + header + n + "END\n".len()
        }
    }))
}

/// Parses one response.
///
/// # Errors
/// See [`ProtocolError`].
pub fn parse_response(bytes: &[u8]) -> Result<Response, ProtocolError> {
    if bytes.len() > MAX_RESPONSE_BYTES + MAX_BODY_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    let Some((at, body_len)) = frame(bytes)? else {
        return Err(ProtocolError::Truncated);
    };
    let (head, body) = match body_len {
        None => {
            if &bytes[at..] != b"END\n" {
                return Err(ProtocolError::Malformed);
            }
            (&bytes[..at], None)
        }
        Some(n) => {
            let line_end = at + bytes[at..].iter().position(|&b| b == b'\n').unwrap_or(0) + 1;
            let rest = &bytes[line_end..];
            if rest.len() < n + 4 {
                return Err(ProtocolError::Truncated);
            }
            if &rest[n..] != b"END\n" {
                return Err(ProtocolError::Malformed);
            }
            (&bytes[..at], Some(body_text(&rest[..n])?))
        }
    };
    let text = std::str::from_utf8(head).map_err(|_| ProtocolError::InvalidCharacters)?;
    let mut lines = text.split_terminator('\n');
    let first = lines.next().ok_or(ProtocolError::Malformed)?;
    if !printable(first) {
        return Err(ProtocolError::InvalidCharacters);
    }
    let rest = first
        .strip_prefix(VERSION)
        .and_then(|r| r.strip_prefix(' '))
        .ok_or(ProtocolError::WrongVersion)?;
    let outcome = if rest == "OK" {
        Outcome::Ok
    } else if let Some(err) = rest.strip_prefix("ERR ") {
        let (code, message) = err.split_once(' ').unwrap_or((err, ""));
        if !is_token(code) || message.len() > MAX_VALUE_BYTES {
            return Err(ProtocolError::Malformed);
        }
        Outcome::Err {
            code: code.to_string(),
            message: message.to_string(),
        }
    } else {
        return Err(ProtocolError::Malformed);
    };
    let mut fields = Vec::new();
    for line in lines {
        let (k, v) = line.split_once('=').ok_or(ProtocolError::Malformed)?;
        if !is_token(k) || v.len() > MAX_VALUE_BYTES || !printable(v) {
            return Err(ProtocolError::InvalidCharacters);
        }
        if fields.len() == MAX_FIELDS {
            return Err(ProtocolError::TooLarge);
        }
        fields.push((k.to_string(), v.to_string()));
    }
    Ok(Response {
        outcome,
        fields,
        body,
    })
}

/// Reads until `end` is seen, the peer closes, or more than `max` bytes
/// arrive (then returns the `max + 1` bytes, so the caller can reject them).
///
/// # Errors
/// Read errors, including time-outs.
pub fn read_message(reader: &mut impl Read, max: usize, end: &[u8]) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 512];
    while buf.len() <= max && !buf.ends_with(end) {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    buf.truncate(max + 1);
    Ok(buf)
}

/// Reads one request: the header line (at most [`MAX_REQUEST_BYTES`]) and
/// then exactly the body it announces, never more. Returns the bytes for
/// [`parse_framed`], which reports anything wrong with them.
///
/// # Errors
/// Read errors, including time-outs.
pub fn read_request(reader: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    // The header, one byte at a time, so nothing after it is consumed.
    while buf.len() <= MAX_REQUEST_BYTES && !buf.ends_with(b"\n") {
        if reader.read(&mut byte)? == 0 {
            return Ok(buf);
        }
        buf.push(byte[0]);
    }
    let header = std::str::from_utf8(&buf)
        .ok()
        .and_then(|l| l.strip_suffix('\n'));
    let Some(Ok((command, want))) = header.map(parse_header) else {
        // parse_framed reports what is wrong with it.
        return Ok(buf);
    };
    if !command.has_body() {
        return Ok(buf);
    }
    let mut body = vec![0u8; want];
    let mut got = 0;
    while got < want {
        let n = reader.read(&mut body[got..])?;
        if n == 0 {
            break;
        }
        got += n;
    }
    buf.extend_from_slice(&body[..got]);
    Ok(buf)
}

/// Reads one response, body included, never more than the limits allow.
///
/// # Errors
/// Read errors, including time-outs.
pub fn read_response(reader: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match response_len(&buf) {
            Ok(Some(total)) if buf.len() >= total => return Ok(buf),
            Err(_) => return Ok(buf),
            _ => {}
        }
        if buf.len() > MAX_RESPONSE_BYTES + MAX_BODY_BYTES + 16 {
            return Ok(buf);
        }
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            return Ok(buf);
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

/// Why a client request failed.
#[derive(Debug)]
pub enum ClientError {
    /// No service is listening (socket missing or connection refused).
    NotRunning,
    /// The socket exists but this user may not use it.
    PermissionDenied,
    /// Any other input/output problem, such as a time-out.
    Io(io::Error),
    /// The service sent something that is not a valid response.
    Protocol(ProtocolError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClientError::NotRunning => f.write_str("the Septuvo service is not running"),
            ClientError::PermissionDenied => f.write_str(
                "permission denied: changing the connection needs membership of the service's control group",
            ),
            ClientError::Io(e) => write!(f, "cannot talk to the Septuvo service: {e}"),
            ClientError::Protocol(e) => write!(f, "invalid answer from the Septuvo service: {e}"),
        }
    }
}

impl std::error::Error for ClientError {}

/// Sends `request` to the socket at `path` and returns the response.
///
/// # Errors
/// See [`ClientError`].
pub fn send(path: &Path, request: &Request) -> Result<Response, ClientError> {
    let mut stream = UnixStream::connect(path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => ClientError::NotRunning,
        io::ErrorKind::PermissionDenied => ClientError::PermissionDenied,
        _ => ClientError::Io(e),
    })?;
    stream
        .set_read_timeout(Some(TIMEOUT))
        .map_err(ClientError::Io)?;
    stream
        .set_write_timeout(Some(TIMEOUT))
        .map_err(ClientError::Io)?;
    stream
        .write_all(&request.encode())
        .map_err(ClientError::Io)?;
    let bytes = read_response(&mut stream).map_err(ClientError::Io)?;
    parse_response(&bytes).map_err(ClientError::Protocol)
}

/// Sends a command without a body.
///
/// # Errors
/// See [`ClientError`].
pub fn request(path: &Path, command: Command) -> Result<Response, ClientError> {
    send(path, &Request::new(command))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    #[test]
    fn requests_round_trip() {
        for c in [Command::Status, Command::Connect, Command::Disconnect] {
            assert_eq!(parse_request(c.encode().as_bytes()), Ok(c));
        }
        for minutes in 1..=MAX_CAPTIVE_MINUTES {
            for network_dns in [false, true] {
                let c = Command::Captive {
                    minutes,
                    network_dns,
                };
                assert_eq!(parse_request(c.encode().as_bytes()), Ok(c));
                assert!(c.needs_control());
            }
        }
        assert_eq!(
            Command::Captive {
                minutes: 5,
                network_dns: false
            }
            .encode(),
            "SEPTUVO/1 CAPTIVE 5\n"
        );
        assert_eq!(
            Command::Captive {
                minutes: 1,
                network_dns: true
            }
            .encode(),
            "SEPTUVO/1 CAPTIVE 1 NETWORK-DNS\n"
        );
        assert!(!Command::Status.needs_control());
        assert!(Command::Connect.needs_control());
        assert!(Command::Disconnect.needs_control());
    }

    #[test]
    fn bad_requests_are_rejected() {
        let long = format!("{VERSION} {}\n", "X".repeat(MAX_REQUEST_BYTES));
        let cases: [(&[u8], ProtocolError); 9] = [
            (b"SEPTUVO/1 STATUS", ProtocolError::Truncated),
            (b"", ProtocolError::Truncated),
            (b"SEPTUVO/2 STATUS\n", ProtocolError::WrongVersion),
            (b"SEPTUVO/1 status\n", ProtocolError::UnknownCommand),
            (b"SEPTUVO/1 STATUS extra\n", ProtocolError::UnknownCommand),
            (b"SEPTUVO/1\n", ProtocolError::Malformed),
            (b"SEPTUVO/1 STA\x00TUS\n", ProtocolError::InvalidCharacters),
            (b"SEPTUVO/1 \xffSTATUS\n", ProtocolError::InvalidCharacters),
            (long.as_bytes(), ProtocolError::TooLarge),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_request(input), Err(expected), "{input:?}");
        }
    }

    #[test]
    fn captive_requests_are_strictly_bounded() {
        for bad in [
            "CAPTIVE 0",
            "CAPTIVE 6",
            "CAPTIVE 9",
            "CAPTIVE 10",
            "CAPTIVE 05",
            "CAPTIVE +5",
            "CAPTIVE -1",
            "CAPTIVE  5",
            "CAPTIVE 5 ",
            "CAPTIVE x",
            "CAPTIVE ",
            "CAPTIVE 5 network-dns",
            "CAPTIVE 5 NETWORK-DNS ",
            "CAPTIVE 5  NETWORK-DNS",
            "CAPTIVE NETWORK-DNS",
            "CAPTIVE 6 NETWORK-DNS",
            "CAPTIVE 5 NETWORK-DNS NETWORK-DNS",
        ] {
            let line = format!("{VERSION} {bad}\n");
            assert_eq!(
                parse_request(line.as_bytes()),
                Err(ProtocolError::Malformed),
                "{bad:?}"
            );
        }
        assert_eq!(
            parse_request(b"SEPTUVO/1 CAPTIVE\n"),
            Err(ProtocolError::UnknownCommand)
        );
    }

    #[test]
    fn responses_round_trip() {
        let r = Response::ok()
            .with("adapter", "fail")
            .with("note", "not before G1");
        assert_eq!(parse_response(r.encode().as_bytes()), Ok(r.clone()));
        assert_eq!(r.field("adapter"), Some("fail"));
        assert_eq!(r.field("missing"), None);
        let e = Response::error("not_permitted", "use the control socket");
        assert_eq!(parse_response(e.encode().as_bytes()), Ok(e));
    }

    #[test]
    fn building_a_response_never_produces_invalid_output() {
        let r = Response::error("Bad Code!", "line\nbreak \u{1b}[31m")
            .with("BAD", "x")
            .with("ok", &"v".repeat(1000))
            .with("tab", "a\tb");
        let encoded = r.encode();
        let parsed = parse_response(encoded.as_bytes()).expect("always valid");
        assert_eq!(
            parsed.outcome,
            Outcome::Err {
                code: "failed".into(),
                message: "line?break ?[31m".into()
            }
        );
        assert_eq!(parsed.fields.len(), 2);
        assert_eq!(parsed.field("ok").map(str::len), Some(MAX_VALUE_BYTES));
        assert_eq!(parsed.field("tab"), Some("a?b"));
        let mut many = Response::ok();
        for i in 0..100 {
            many = many.with(&format!("k{i}"), "v");
        }
        assert_eq!(many.fields.len(), MAX_FIELDS);
        assert!(parse_response(many.encode().as_bytes()).is_ok());
    }

    /// Regression (N3-03 fuzzing, case seed 0x35f6d570c8fcedeb): many long
    /// fields used to encode to more than the client accepts.
    #[test]
    fn built_responses_never_exceed_the_size_limit() {
        let mut r = Response::ok();
        for i in 0..MAX_FIELDS {
            r = r.with(&format!("field_{i}"), &"v".repeat(MAX_VALUE_BYTES));
        }
        let encoded = r.encode();
        assert!(
            encoded.len() <= MAX_RESPONSE_BYTES,
            "{} bytes",
            encoded.len()
        );
        assert_eq!(r.encoded_len(), encoded.len());
        assert!(
            r.fields.len() < MAX_FIELDS,
            "the fields that do not fit are dropped"
        );
        assert_eq!(parse_response(encoded.as_bytes()), Ok(r.clone()));
        let first = r.fields.first().map(|(k, _)| k.as_str());
        assert_eq!(first, Some("field_0"), "earlier fields are kept");
    }

    /// Regression (N3-03 fuzzing): an error without a message re-encodes
    /// without a trailing space, so a response at the limit stays valid.
    #[test]
    fn error_without_message_round_trips_at_the_size_limit() {
        let e = Response::error("limit", "");
        assert_eq!(e.encode(), "SEPTUVO/1 ERR limit\nEND\n");
        let mut body = String::from("SEPTUVO/1 ERR limit\n");
        while body.len() + 4 + 205 <= MAX_RESPONSE_BYTES && body.matches('=').count() < MAX_FIELDS {
            let _ = writeln!(body, "k{}={}", body.len() % 10, "v".repeat(200));
        }
        let pad = MAX_RESPONSE_BYTES - body.len() - 4 - 3;
        let _ = writeln!(body, "p={}\nEND", "v".repeat(pad.min(MAX_VALUE_BYTES)));
        let parsed = parse_response(body.as_bytes()).expect("valid response");
        assert_eq!(parse_response(parsed.encode().as_bytes()), Ok(parsed));
    }

    #[test]
    fn bad_responses_are_rejected() {
        let cases: [(&[u8], ProtocolError); 8] = [
            (b"SEPTUVO/1 OK\n", ProtocolError::Truncated),
            (b"SEPTUVO/2 OK\nEND\n", ProtocolError::WrongVersion),
            (b"SEPTUVO/1 FINE\nEND\n", ProtocolError::Malformed),
            (b"SEPTUVO/1 ERR Bad x\nEND\n", ProtocolError::Malformed),
            (b"SEPTUVO/1 OK\nnovalue\nEND\n", ProtocolError::Malformed),
            (
                b"SEPTUVO/1 OK\nKey=v\nEND\n",
                ProtocolError::InvalidCharacters,
            ),
            (
                b"SEPTUVO/1 OK\nk=\x1b[2J\nEND\n",
                ProtocolError::InvalidCharacters,
            ),
            (b"END\n", ProtocolError::Malformed),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_response(input), Err(expected), "{input:?}");
        }
        let big = vec![b'a'; MAX_RESPONSE_BYTES + 1];
        assert_eq!(parse_response(&big), Err(ProtocolError::TooLarge));
    }

    #[test]
    fn read_message_is_bounded() {
        let mut endless = std::io::repeat(b'a');
        let got = read_message(&mut endless, 100, b"\n").expect("reads");
        assert_eq!(got.len(), 101);
        let mut short: &[u8] = b"SEPTUVO/1 STATUS\nignored";
        let got = read_message(&mut short, MAX_REQUEST_BYTES, b"\n").expect("reads");
        assert!(got.starts_with(b"SEPTUVO/1 STATUS\n"));
    }

    #[test]
    fn connection_ids_are_strict() {
        let id = ConnectionId::parse("c0042").expect("valid");
        assert_eq!((id.number(), id.to_string()), (42, "c0042".to_string()));
        assert_eq!(
            ConnectionId::new(9999).map(|i| i.to_string()).as_deref(),
            Some("c9999")
        );
        for bad in [
            "c0000", "c42", "c00042", "C0042", "c-001", "c+001", "x0001", "c 001", "", "c",
        ] {
            assert_eq!(ConnectionId::parse(bad), None, "{bad:?}");
        }
        assert_eq!(ConnectionId::new(0), None);
        assert_eq!(ConnectionId::new(10_000), None);
    }

    #[test]
    fn connection_requests_round_trip() {
        let id = ConnectionId::new(7).expect("id");
        for c in [
            Command::List,
            Command::Get(id),
            Command::Delete(id),
            Command::Select(id),
        ] {
            assert_eq!(parse_request(c.encode().as_bytes()), Ok(c));
            assert_eq!(parse_framed(c.encode().as_bytes()), Ok(Request::new(c)));
        }
        assert!(
            !Command::List.needs_control(),
            "names are visible to every user"
        );
        assert!(
            Command::Get(id).needs_control(),
            "configurations need the control socket"
        );
        for (c, body) in [
            (Command::Add, "version = 1\n[server]\n"),
            (Command::Replace(id), "x = \"é\"\r\n\ty"),
            (Command::Rename(id), "\u{1f3e0} Home VPN"),
        ] {
            let r = Request::with_body(c, body);
            let wire = r.encode();
            assert_eq!(parse_framed(&wire), Ok(r.clone()));
            assert!(wire.iter().position(|&b| b == b'\n').expect("header") < MAX_REQUEST_BYTES);
            assert!(c.needs_control() && c.has_body());
        }
        assert_eq!(
            Request::with_body(Command::Rename(id), "Home").encode(),
            b"SEPTUVO/1 RENAME c0007 4\nHome".to_vec()
        );
    }

    #[test]
    fn bodies_are_exact_and_bounded() {
        let ok = |s: &[u8]| parse_framed(s);
        assert_eq!(ok(b"SEPTUVO/1 ADD 5\nabcd"), Err(ProtocolError::Truncated));
        assert_eq!(
            ok(b"SEPTUVO/1 ADD 5\nabcdef"),
            Err(ProtocolError::Malformed)
        );
        assert_eq!(ok(b"SEPTUVO/1 ADD 0\n"), Err(ProtocolError::Malformed));
        assert_eq!(
            ok(b"SEPTUVO/1 ADD 05\nabcde"),
            Err(ProtocolError::Malformed)
        );
        assert_eq!(
            ok(b"SEPTUVO/1 ADD +5\nabcde"),
            Err(ProtocolError::Malformed)
        );
        assert_eq!(ok(b"SEPTUVO/1 ADD 65537\n"), Err(ProtocolError::TooLarge));
        assert_eq!(ok(b"SEPTUVO/1 ADD 999999\n"), Err(ProtocolError::Malformed));
        assert_eq!(ok(b"SEPTUVO/1 ADD\n"), Err(ProtocolError::UnknownCommand));
        assert_eq!(
            ok(b"SEPTUVO/1 ADD 3\na\x00b"),
            Err(ProtocolError::InvalidCharacters)
        );
        assert_eq!(
            ok(b"SEPTUVO/1 ADD 3\na\x1bb"),
            Err(ProtocolError::InvalidCharacters)
        );
        assert_eq!(
            ok(b"SEPTUVO/1 ADD 2\n\xff\xfe"),
            Err(ProtocolError::InvalidCharacters)
        );
        assert_eq!(
            ok(b"SEPTUVO/1 RENAME c0001 1 2\nab"),
            Err(ProtocolError::Malformed)
        );
        assert_eq!(
            ok(b"SEPTUVO/1 GET c0001 4\nabcd"),
            Err(ProtocolError::Malformed)
        );
        assert_eq!(
            ok(b"SEPTUVO/1 SELECT c0001\nextra"),
            Err(ProtocolError::Malformed)
        );
        assert_eq!(
            ok(b"SEPTUVO/1 SELECT c0000\n"),
            Err(ProtocolError::Malformed)
        );
        let max = vec![b'a'; MAX_BODY_BYTES];
        let mut wire = format!("SEPTUVO/1 ADD {MAX_BODY_BYTES}\n").into_bytes();
        wire.extend_from_slice(&max);
        assert!(ok(&wire).is_ok());
        // parse_request never accepts a body.
        assert!(parse_request(b"SEPTUVO/1 ADD 1\na").is_err());
    }

    #[test]
    fn read_request_takes_exactly_the_announced_body() {
        let wire = Request::with_body(Command::Add, "body!").encode();
        let mut input = wire.clone();
        input.extend_from_slice(b"TRAILING DATA");
        let mut reader: &[u8] = &input;
        assert_eq!(read_request(&mut reader).expect("reads"), wire);
        assert_eq!(
            reader, b"TRAILING DATA",
            "nothing after the body is consumed"
        );
        let mut endless = std::io::repeat(b'a');
        assert_eq!(
            read_request(&mut endless).expect("reads").len(),
            MAX_REQUEST_BYTES + 1
        );
        // A huge announced length is refused before anything is allocated.
        let mut huge: &[u8] = b"SEPTUVO/1 ADD 99999\nxx";
        let got = read_request(&mut huge).expect("reads");
        assert_eq!(got, b"SEPTUVO/1 ADD 99999\n");
        assert_eq!(parse_framed(&got), Err(ProtocolError::TooLarge));
    }

    #[test]
    fn responses_with_bodies_round_trip() {
        let r = Response::ok()
            .with("selected", "c0001")
            .with_body("c0001\tHome VPN\nc0002\tEND\nEND\n");
        let wire = r.encode();
        assert_eq!(parse_response(wire.as_bytes()), Ok(r.clone()));
        let mut reader = wire.as_bytes().chain(&b"more"[..]);
        let got = read_response(&mut reader).expect("reads");
        assert_eq!(got, wire.as_bytes());
        let big = Response::ok().with_body(&"é".repeat(MAX_BODY_BYTES));
        let body = big.body.as_deref().expect("body");
        assert!(body.len() <= MAX_BODY_BYTES && body.len() > MAX_BODY_BYTES - 2);
        assert_eq!(parse_response(big.encode().as_bytes()), Ok(big.clone()));
        let empty = Response::ok().with_body("");
        assert_eq!(parse_response(empty.encode().as_bytes()), Ok(empty));
        let cleaned = Response::ok().with_body("a\u{0}b\u{1b}c\td");
        assert_eq!(cleaned.body.as_deref(), Some("a?b?c\td"));
        for bad in [
            &b"SEPTUVO/1 OK\nBODY 3\nabEND\n"[..],
            b"SEPTUVO/1 OK\nBODY 3\nabcdEND\n",
            b"SEPTUVO/1 OK\nBODY 3\nabc",
            b"SEPTUVO/1 OK\nBODY 00\nEND\n",
            b"SEPTUVO/1 OK\nBODY 99999\nEND\n",
            b"SEPTUVO/1 OK\nBODY 1\n\x07END\n",
            b"SEPTUVO/1 OK\nEND\nextra",
        ] {
            assert!(parse_response(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn socket_paths() {
        let dir = Path::new("/run/septuvo");
        assert_eq!(status_socket(dir), Path::new("/run/septuvo/status.sock"));
        assert_eq!(
            control_socket(dir),
            Path::new("/run/septuvo/control/control.sock")
        );
    }

    #[test]
    fn missing_service_is_reported_as_not_running() {
        let missing = Path::new("/nonexistent-septuvo-test/status.sock");
        assert!(matches!(
            request(missing, Command::Status),
            Err(ClientError::NotRunning)
        ));
    }
}
