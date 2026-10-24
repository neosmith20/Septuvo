//! Native **Open** (import) and **Save As** (export) dialogs (DEFAULT-UI
//! §4.3, §4.4; N4-D3).
//!
//! They are the desktop's own file chooser, reached through the
//! xdg-desktop-portal over D-Bus (`ashpd`, pure Rust): the user picks any
//! file or folder they can reach; the app never asks for a path or for
//! file text. The portal asks before replacing an existing file. Each
//! dialog runs on its own thread, so the window never waits; its result
//! arrives in [`Files::events`]. [`FakeFiles`] stands in for tests.
//!
//! Reading is bounded to the protocol's 64 KiB, and the text is only
//! returned, never interpreted here: the import preview validates it.
//! Writing is atomic (a temporary file in the same folder, then a rename),
//! so a failure never leaves half a file.

use std::fs;
use std::io::{Read as _, Write as _};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use crate::form::MAX_TEXT_BYTES;

/// A file the user chose to import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    /// The file's name, without its folder (for the suggested connection
    /// name), plain text.
    pub file_name: String,
    /// Its contents (UTF-8, at most 64 KiB).
    pub text: String,
}

/// The result of a file dialog. `Ok(None)`: the user cancelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    /// From [`Files::open`].
    Opened(Result<Option<Opened>, String>),
    /// From [`Files::save`]: the name of the file written.
    Saved(Result<Option<String>, String>),
}

/// The desktop's file dialogs.
pub trait Files {
    /// Shows the native Open dialog for a configuration file.
    fn open(&mut self);
    /// Shows the native Save As dialog, suggesting `file_name`, and writes
    /// `contents` to the file the user chooses.
    fn save(&mut self, file_name: String, contents: String);
    /// Results that have arrived.
    fn events(&mut self) -> Vec<FileEvent>;
}

/// The message when the file is not a usable text file.
pub const NOT_SUPPORTED: &str = "This file is not a supported Septuvo configuration.";
/// The message when the file is too large.
pub const TOO_LARGE: &str = "The file is too large.";

fn no_portal(e: &ashpd::Error) -> String {
    format!(
        "The file chooser is not available: the desktop's file portal did not answer ({}).",
        crate::state::plain(&e.to_string())
    )
}

/// A local path from a `file://` URI, percent-decoded. Anything else
/// (another scheme, a remote host) is refused.
#[must_use]
pub fn path_from_uri(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    if out.contains(&0) {
        return None;
    }
    Some(PathBuf::from(std::ffi::OsString::from_vec(out)))
}

/// Reads a chosen file: a regular file, at most 64 KiB, UTF-8 text.
///
/// # Errors
/// Not readable, not a regular file, too large or not text.
pub fn read_bounded(path: &Path) -> Result<Opened, String> {
    let meta = fs::metadata(path).map_err(|e| format!("The file could not be read: {e}."))?;
    if !meta.is_file() {
        return Err(NOT_SUPPORTED.into());
    }
    if meta.len() > MAX_TEXT_BYTES as u64 {
        return Err(TOO_LARGE.into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|f| f.take(MAX_TEXT_BYTES as u64 + 1).read_to_end(&mut bytes))
        .map_err(|e| format!("The file could not be read: {e}."))?;
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(TOO_LARGE.into());
    }
    let text = String::from_utf8(bytes).map_err(|_| NOT_SUPPORTED.to_string())?;
    let file_name = path
        .file_name()
        .map(|n| crate::state::plain(&n.to_string_lossy()))
        .unwrap_or_default();
    Ok(Opened { file_name, text })
}

/// Writes `contents` to `path` atomically (mode 0600: a backup is the
/// user's own file).
///
/// # Errors
/// The folder is not writable, or the disk is full.
pub fn write_atomic(path: &Path, contents: &str) -> Result<(), String> {
    let fail = |e: std::io::Error| format!("The file could not be saved: {e}.");
    let name = path.file_name().ok_or("The file could not be saved.")?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(".septuvo-tmp");
    let tmp = path.with_file_name(tmp_name);
    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)
            .map_err(fail)?;
        if let Err(e) = f.write_all(contents.as_bytes()).and_then(|()| f.sync_all()) {
            let _ = fs::remove_file(&tmp);
            return Err(fail(e));
        }
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        fail(e)
    })
}

async fn pick_open() -> Result<Option<PathBuf>, String> {
    use ashpd::desktop::ResponseError;
    use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
    let request = SelectedFiles::open_file()
        .title("Import Configuration")
        .accept_label("Open")
        .modal(true)
        .multiple(false)
        .filter(FileFilter::new("Septuvo Configuration (*.toml)").glob("*.toml"))
        .filter(FileFilter::new("All Files").glob("*"))
        .send()
        .await
        .map_err(|e| no_portal(&e))?;
    match request.response() {
        Ok(files) => Ok(files.uris().first().and_then(|u| path_from_uri(u.as_str()))),
        Err(ashpd::Error::Response(ResponseError::Cancelled)) => Ok(None),
        Err(e) => Err(no_portal(&e)),
    }
}

async fn pick_save(file_name: &str) -> Result<Option<PathBuf>, String> {
    use ashpd::desktop::ResponseError;
    use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
    let request = SelectedFiles::save_file()
        .title("Export Configuration")
        .accept_label("Save")
        .modal(true)
        .current_name(file_name)
        .filter(FileFilter::new("Septuvo Configuration (*.toml)").glob("*.toml"))
        .send()
        .await
        .map_err(|e| no_portal(&e))?;
    match request.response() {
        Ok(files) => Ok(files.uris().first().and_then(|u| path_from_uri(u.as_str()))),
        Err(ashpd::Error::Response(ResponseError::Cancelled)) => Ok(None),
        Err(e) => Err(no_portal(&e)),
    }
}

/// The real dialogs, through the desktop portal.
pub struct Portal {
    tx: Sender<FileEvent>,
    rx: Receiver<FileEvent>,
    wake: std::sync::Arc<dyn Fn() + Send + Sync>,
}

impl Portal {
    /// Dialogs that call `wake` when a result arrives (to repaint).
    #[must_use]
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Portal {
        let (tx, rx) = mpsc::channel();
        Portal {
            tx,
            rx,
            wake: std::sync::Arc::new(wake),
        }
    }

    fn spawn(&self, job: impl FnOnce() -> FileEvent + Send + 'static) {
        let tx = self.tx.clone();
        let wake = self.wake.clone();
        let started = std::thread::Builder::new()
            .name("septuvo-file-dialog".into())
            .spawn(move || {
                let _ = tx.send(job());
                wake();
            });
        if started.is_err() {
            let _ = self.tx.send(FileEvent::Opened(Err(
                "The file chooser could not be started.".into(),
            )));
        }
    }
}

impl Files for Portal {
    fn open(&mut self) {
        self.spawn(|| {
            FileEvent::Opened(
                ashpd::zbus::block_on(pick_open())
                    .and_then(|p| p.map(|p| read_bounded(&p)).transpose()),
            )
        });
    }

    fn save(&mut self, file_name: String, contents: String) {
        self.spawn(move || {
            FileEvent::Saved(ashpd::zbus::block_on(pick_save(&file_name)).and_then(|p| {
                p.map(|p| {
                    write_atomic(&p, &contents).map(|()| {
                        p.file_name()
                            .map(|n| crate::state::plain(&n.to_string_lossy()))
                            .unwrap_or_default()
                    })
                })
                .transpose()
            }))
        });
    }

    fn events(&mut self) -> Vec<FileEvent> {
        self.rx.try_iter().collect()
    }
}

/// Dialogs for tests: each answers at once with the prepared result.
#[derive(Debug, Clone, Default)]
pub struct FakeFiles {
    /// What the next Open returns (default: cancelled).
    pub open_result: Option<Result<Option<Opened>, String>>,
    /// What the next Save As returns: the chosen file name, or why it
    /// failed (default: cancelled).
    pub save_result: Option<Result<Option<String>, String>>,
    /// Every file "written": name and contents, shared with the test.
    pub written: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
    /// The file names suggested to Save As, shared with the test.
    pub suggested: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    queued: Vec<FileEvent>,
}

impl Files for FakeFiles {
    fn open(&mut self) {
        let r = self.open_result.clone().unwrap_or(Ok(None));
        self.queued.push(FileEvent::Opened(r));
    }

    fn save(&mut self, file_name: String, contents: String) {
        if let Ok(mut s) = self.suggested.lock() {
            s.push(file_name);
        }
        let r = self.save_result.clone().unwrap_or(Ok(None));
        if let (Ok(Some(name)), Ok(mut w)) = (&r, self.written.lock()) {
            w.push((name.clone(), contents));
        }
        self.queued.push(FileEvent::Saved(r));
    }

    fn events(&mut self) -> Vec<FileEvent> {
        std::mem::take(&mut self.queued)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_local_file_uris_become_paths() {
        assert_eq!(
            path_from_uri("file:///home/a/My%20VPN.toml"),
            Some(PathBuf::from("/home/a/My VPN.toml"))
        );
        assert_eq!(
            path_from_uri("file://localhost/tmp/x"),
            Some(PathBuf::from("/tmp/x"))
        );
        for bad in [
            "http://x/y",
            "file://host/x",
            "file:///a%2",
            "file:///a%zz",
            "file:///a%00b",
            "x",
        ] {
            assert_eq!(path_from_uri(bad), None, "{bad}");
        }
    }

    #[test]
    fn reading_is_bounded_and_text_only() {
        let dir = std::env::temp_dir().join(format!("septuvo-files-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let ok = dir.join("Home VPN.toml");
        fs::write(&ok, "version = 1\n").unwrap();
        let o = read_bounded(&ok).unwrap();
        assert_eq!(o.file_name, "Home VPN.toml");
        let big = dir.join("big.toml");
        fs::write(&big, vec![b'#'; MAX_TEXT_BYTES + 1]).unwrap();
        assert_eq!(read_bounded(&big).unwrap_err(), TOO_LARGE);
        let bin = dir.join("bin.toml");
        fs::write(&bin, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(read_bounded(&bin).unwrap_err(), NOT_SUPPORTED);
        assert_eq!(read_bounded(&dir).unwrap_err(), NOT_SUPPORTED);
        // Atomic write, replacing an existing file the user confirmed.
        write_atomic(&ok, "new").unwrap();
        assert_eq!(fs::read_to_string(&ok).unwrap(), "new");
        assert!(write_atomic(&dir.join("missing").join("x.toml"), "x").is_err());
        assert_eq!(
            fs::read_dir(&dir).unwrap().count(),
            3,
            "no temporary files left"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
