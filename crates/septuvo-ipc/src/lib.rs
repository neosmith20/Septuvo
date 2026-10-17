//! Septuvo local control interface (N2-02).
//!
//! The small, versioned text protocol between the unprivileged `septuvo`
//! command-line tool and the privileged service `septuvod`, over Unix
//! sockets on the same machine. It carries **no secrets and no traffic**:
//! only commands (status, connect, disconnect) and status fields.
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
//! Both sides treat the other as hostile (ADV-10): every message has a size
//! limit, fields are restricted to printable ASCII, and anything malformed
//! is rejected rather than repaired.
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
/// Largest request, in bytes, including the line feed.
pub const MAX_REQUEST_BYTES: usize = 64;
/// Largest response, in bytes.
pub const MAX_RESPONSE_BYTES: usize = 4096;
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

/// A command from the CLI to the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Report status. Allowed on both sockets.
    Status,
    /// Turn protection on (kill switch; no tunnel before G1).
    Connect,
    /// Deliberately disconnect: the only way protection is lifted (Q2).
    Disconnect,
}

impl Command {
    /// The command's word on the wire.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Command::Status => "STATUS",
            Command::Connect => "CONNECT",
            Command::Disconnect => "DISCONNECT",
        }
    }

    /// True if the command changes the network and so needs the control
    /// socket.
    #[must_use]
    pub fn needs_control(self) -> bool {
        !matches!(self, Command::Status)
    }

    /// The request line for this command.
    #[must_use]
    pub fn encode(self) -> String {
        format!("{VERSION} {}\n", self.as_str())
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

/// Parses one request.
///
/// # Errors
/// See [`ProtocolError`].
pub fn parse_request(bytes: &[u8]) -> Result<Command, ProtocolError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    let Some(line) = bytes.strip_suffix(b"\n") else {
        return Err(ProtocolError::Truncated);
    };
    let line = std::str::from_utf8(line).map_err(|_| ProtocolError::InvalidCharacters)?;
    if !printable(line) {
        return Err(ProtocolError::InvalidCharacters);
    }
    let Some((version, command)) = line.split_once(' ') else {
        return Err(ProtocolError::Malformed);
    };
    if version != VERSION {
        return Err(ProtocolError::WrongVersion);
    }
    match command {
        "STATUS" => Ok(Command::Status),
        "CONNECT" => Ok(Command::Connect),
        "DISCONNECT" => Ok(Command::Disconnect),
        _ => Err(ProtocolError::UnknownCommand),
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
        }
    }

    /// Adds a field. Invalid keys and fields beyond [`MAX_FIELDS`] are
    /// dropped; values are cleaned. Encoding therefore never fails.
    #[must_use]
    pub fn with(mut self, key: &str, value: &str) -> Self {
        if is_token(key) && self.fields.len() < MAX_FIELDS {
            self.fields.push((key.to_string(), clean(value)));
        }
        self
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
            Outcome::Err { code, message } => format!("{VERSION} ERR {code} {message}\n"),
        };
        for (k, v) in &self.fields {
            s.push_str(k);
            s.push('=');
            s.push_str(v);
            s.push('\n');
        }
        s.push_str("END\n");
        s
    }
}

/// Parses one response.
///
/// # Errors
/// See [`ProtocolError`].
pub fn parse_response(bytes: &[u8]) -> Result<Response, ProtocolError> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ProtocolError::InvalidCharacters)?;
    let Some(body) = text.strip_suffix("END\n") else {
        return Err(ProtocolError::Truncated);
    };
    let mut lines = body.split_terminator('\n');
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
    Ok(Response { outcome, fields })
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

/// Sends `command` to the socket at `path` and returns the response.
///
/// # Errors
/// See [`ClientError`].
pub fn request(path: &Path, command: Command) -> Result<Response, ClientError> {
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
        .write_all(command.encode().as_bytes())
        .map_err(ClientError::Io)?;
    let bytes = read_message(&mut stream, MAX_RESPONSE_BYTES, b"END\n").map_err(ClientError::Io)?;
    parse_response(&bytes).map_err(ClientError::Protocol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_round_trip() {
        for c in [Command::Status, Command::Connect, Command::Disconnect] {
            assert_eq!(parse_request(c.encode().as_bytes()), Ok(c));
        }
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
