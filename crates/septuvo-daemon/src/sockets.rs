//! The service's two Unix sockets and the code that serves them.
//!
//! ```text
//! RUNTIME_DIR/                 root, 0755
//! RUNTIME_DIR/status.sock      0666   any local user: STATUS only
//! RUNTIME_DIR/control/         root:GROUP, 0750   only GROUP members may enter
//! RUNTIME_DIR/control/control.sock   0660   STATUS, CONNECT, DISCONNECT
//! ```
//!
//! Access control is by file permissions, enforced by the kernel. The
//! control socket sits inside a directory that only the control group can
//! enter, so it is never reachable by others, not even for an instant
//! between creating the socket and setting its mode.

use septuvo_ipc::{self as ipc, Request, Response};
use std::fs::{self, DirBuilder, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

/// Why the runtime directory could not be prepared.
#[derive(Debug)]
pub struct SetupError(pub String);

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SetupError {}

fn setup<T>(what: &str, path: &Path, e: &io::Error) -> Result<T, SetupError> {
    Err(SetupError(format!("{what} {}: {e}", path.display())))
}

/// Creates `path` with `mode` if missing; otherwise checks that it is a real
/// directory (not a symbolic link) owned by root and not writable by others.
fn ensure_dir(path: &Path, mode: u32, gid: u32) -> Result<(), SetupError> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.file_type().is_dir() {
                return Err(SetupError(format!(
                    "{} exists and is not a directory (refusing to use it)",
                    path.display()
                )));
            }
            if meta.uid() != 0 || meta.mode() & 0o002 != 0 {
                return Err(SetupError(format!(
                    "{} must be owned by root and not writable by other users",
                    path.display()
                )));
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            if let Err(e) = DirBuilder::new().mode(mode).create(path) {
                return setup("cannot create", path, &e);
            }
        }
        Err(e) => return setup("cannot inspect", path, &e),
    }
    if let Err(e) = std::os::unix::fs::chown(path, Some(0), Some(gid)) {
        return setup("cannot set owner of", path, &e);
    }
    if let Err(e) = fs::set_permissions(path, Permissions::from_mode(mode)) {
        return setup("cannot set permissions of", path, &e);
    }
    Ok(())
}

/// Removes a stale socket left by an earlier run. Anything that is not a
/// socket is never removed: the service refuses to start instead.
fn remove_stale_socket(path: &Path) -> Result<(), SetupError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_socket() => {
            fs::remove_file(path).or_else(|e| setup("cannot remove old socket", path, &e))
        }
        Ok(_) => Err(SetupError(format!(
            "{} exists and is not a socket (refusing to touch it)",
            path.display()
        ))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => setup("cannot inspect", path, &e),
    }
}

fn bind(path: &Path, mode: u32, gid: u32) -> Result<UnixListener, SetupError> {
    remove_stale_socket(path)?;
    let listener = UnixListener::bind(path).or_else(|e| setup("cannot listen on", path, &e))?;
    if let Err(e) = std::os::unix::fs::chown(path, Some(0), Some(gid)) {
        return setup("cannot set owner of", path, &e);
    }
    fs::set_permissions(path, Permissions::from_mode(mode))
        .or_else(|e| setup("cannot set permissions of", path, &e))?;
    Ok(listener)
}

/// The two listening sockets.
pub struct Listeners {
    /// Status socket (any local user).
    pub status: UnixListener,
    /// Control socket (control group only).
    pub control: UnixListener,
}

/// Prepares `runtime_dir` and binds both sockets. `control_gid` is the
/// numeric ID of the control group.
///
/// # Errors
/// Any unsafe existing file, or any system error.
pub fn listen(runtime_dir: &Path, control_gid: u32) -> Result<Listeners, SetupError> {
    ensure_dir(runtime_dir, 0o755, 0)?;
    let control_dir = runtime_dir.join(ipc::CONTROL_DIR);
    ensure_dir(&control_dir, 0o750, control_gid)?;
    let status = bind(&ipc::status_socket(runtime_dir), 0o666, 0)?;
    let control = bind(&ipc::control_socket(runtime_dir), 0o660, control_gid)?;
    Ok(Listeners { status, control })
}

/// Reads one request from `stream`, asks `handle` for the response and sends
/// it. Malformed, oversized or slow requests get an error response; nothing
/// a client sends can make the service stop.
///
/// One request per connection: the header and exactly the body it
/// announces are read (at most [`ipc::MAX_BODY_BYTES`]); anything sent after
/// that is never read or acted on.
pub fn serve_one(stream: &mut UnixStream, handle: &mut impl FnMut(&Request) -> Response) {
    let _ = stream.set_read_timeout(Some(ipc::TIMEOUT));
    let _ = stream.set_write_timeout(Some(ipc::TIMEOUT));
    let response = match ipc::read_request(stream) {
        Err(_) => Response::error("timeout", "no complete request received in time"),
        Ok(bytes) => match ipc::parse_framed(&bytes) {
            Ok(request) => handle(&request),
            Err(e) => Response::error("bad_request", &e.to_string()),
        },
    };
    let _ = stream.write_all(response.encode().as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn pair() -> (UnixStream, UnixStream) {
        UnixStream::pair().expect("socket pair")
    }

    /// Sends `request`, serves it, and returns (handler called?, response
    /// text). The text is `None` if the connection was reset, which the
    /// kernel does when the service closes with unread oversized input.
    fn exchange(request: &[u8]) -> (bool, Option<String>) {
        let (mut client, mut server) = pair();
        client.write_all(request).expect("write");
        client
            .shutdown(std::net::Shutdown::Write)
            .expect("shutdown");
        let mut called = false;
        serve_one(&mut server, &mut |r| {
            called = true;
            Response::ok().with("seen", r.command.as_str())
        });
        drop(server);
        let mut out = String::new();
        let text = client.read_to_string(&mut out).ok().map(|_| out);
        (called, text)
    }

    #[test]
    fn serves_a_valid_request() {
        let (called, out) = exchange(b"SEPTUVO/1 STATUS\n");
        assert!(called);
        let out = out.expect("response");
        let r = ipc::parse_response(out.as_bytes()).expect("valid response");
        assert_eq!(r.field("seen"), Some("STATUS"));
    }

    #[test]
    fn malformed_and_oversized_requests_are_never_acted_on() {
        for bad in [
            &b"hello\n"[..],
            b"SEPTUVO/1 REBOOT\n",
            b"SEPTUVO/1 STATUS",
            b"SEPTUVO/1 ADD 10\nshort",
            b"SEPTUVO/1 RENAME c0001 3\na\x00b",
            &[b'A'; 10_000][..],
        ] {
            let (called, out) = exchange(bad);
            assert!(
                !called,
                "handler must not run for {:?}",
                &bad[..bad.len().min(20)]
            );
            if let Some(out) = out {
                let r = ipc::parse_response(out.as_bytes()).expect("valid response");
                assert!(
                    matches!(r.outcome, ipc::Outcome::Err { ref code, .. } if code == "bad_request"),
                    "{out}"
                );
            }
        }
    }

    /// Only one request is read per connection: a second one sent after it
    /// is never acted on. (The client may not even get the answer: the
    /// connection is closed with its extra data unread.)
    #[test]
    fn only_the_first_request_is_read() {
        let (mut client, mut server) = pair();
        client
            .write_all(b"SEPTUVO/1 STATUS\nSEPTUVO/1 CONNECT\n")
            .expect("write");
        let mut seen = Vec::new();
        serve_one(&mut server, &mut |r| {
            seen.push(r.command);
            Response::ok()
        });
        assert_eq!(seen, [ipc::Command::Status]);
    }

    #[test]
    fn serves_a_request_with_a_body() {
        let wire = ipc::Request::with_body(ipc::Command::Add, "x".repeat(5000)).encode();
        let (called, out) = exchange(&wire);
        assert!(called);
        let r = ipc::parse_response(out.expect("response").as_bytes()).expect("valid");
        assert_eq!(r.field("seen"), Some("ADD"));
    }

    #[test]
    fn refuses_unsafe_existing_paths() {
        let dir = std::env::temp_dir().join(format!("septuvo-sock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("not-a-socket");
        fs::write(&file, "x").expect("file");
        assert!(
            remove_stale_socket(&file).is_err(),
            "regular files are never removed"
        );
        assert!(file.exists());
        assert!(remove_stale_socket(&dir.join("missing")).is_ok());
        let sock = dir.join("old.sock");
        let _listener = UnixListener::bind(&sock).expect("bind");
        assert!(
            remove_stale_socket(&sock).is_ok(),
            "stale sockets are removed"
        );
        assert!(!sock.exists());
        assert!(
            ensure_dir(&file, 0o755, 0).is_err(),
            "a file is not a directory"
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }
}
