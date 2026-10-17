//! The data directory: device records, a lock and the action log.
//!
//! ```text
//! DATA_DIR/                 0700, owned by the administrator running the tool
//! DATA_DIR/devices.db       0600  the records (written atomically)
//! DATA_DIR/devices.lock     0600  held while a command runs
//! DATA_DIR/actions.log      0600  who did what, when (no secrets, AD-2)
//! ```
//!
//! **Crash safety:** records are written to a temporary file, flushed to
//! disk, then renamed over the old file, and the directory is flushed. An
//! interruption at any point leaves either the old or the new file, never a
//! half-written one.
//!
//! **Fail closed (RV-2):** a missing records file means "no devices yet"; a
//! file that exists but cannot be read or parsed is an error, never an empty
//! list, so revoked devices can never come back by damaging the file.

use crate::records::{MAX_FILE_BYTES, Records};
use septuvo_policy::Prefix;
use std::fs::{self, DirBuilder, File, OpenOptions, Permissions, TryLockError};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// Records file name inside the data directory.
pub const RECORDS: &str = "devices.db";
/// Lock file name.
pub const LOCK: &str = "devices.lock";
/// Action log name.
pub const LOG: &str = "actions.log";
const TEMP: &str = "devices.db.tmp";

/// Why the data directory could not be used.
#[derive(Debug)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StoreError {}

fn err(what: &str, path: &Path, e: &io::Error) -> StoreError {
    StoreError(format!("{what} {}: {e}", path.display()))
}

/// An open data directory, locked for the lifetime of this value.
pub struct Store {
    dir: PathBuf,
    v4: Prefix,
    v6: Prefix,
    _lock: File,
}

/// Test hook: where a save is interrupted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interrupt {
    /// Not interrupted.
    Never,
    /// After the temporary file is written, before the rename.
    BeforeRename,
}

impl Store {
    /// Opens (creating if needed) `dir` and takes the lock. Fails at once if
    /// another `septuvo-server` command holds it.
    ///
    /// # Errors
    /// Unsafe paths, a held lock, or system errors.
    pub fn open(dir: &Path, v4: Prefix, v6: Prefix) -> Result<Store, StoreError> {
        match fs::symlink_metadata(dir) {
            Ok(m) if m.file_type().is_symlink() || !m.is_dir() => {
                return Err(StoreError(format!(
                    "{} is not a directory (symbolic links are refused)",
                    dir.display()
                )));
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(dir)
                    .map_err(|e| err("cannot create", dir, &e))?;
            }
            Err(e) => return Err(err("cannot inspect", dir, &e)),
        }
        fs::set_permissions(dir, Permissions::from_mode(0o700))
            .map_err(|e| err("cannot set permissions of", dir, &e))?;
        let lock_path = dir.join(LOCK);
        let lock = open_private(&lock_path, false)?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(StoreError(
                    "another septuvo-server command is running; try again".into(),
                ));
            }
            Err(TryLockError::Error(e)) => return Err(err("cannot lock", &lock_path, &e)),
        }
        Ok(Store {
            dir: dir.to_path_buf(),
            v4,
            v6,
            _lock: lock,
        })
    }

    /// Reads the records. Missing file: no devices. Damaged file: error.
    ///
    /// # Errors
    /// See the module documentation (fail closed).
    pub fn load(&self) -> Result<Records, StoreError> {
        let path = self.dir.join(RECORDS);
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Records::default()),
            Err(e) => return Err(err("cannot inspect", &path, &e)),
            Ok(m) if !m.is_file() => {
                return Err(StoreError(format!(
                    "{} is not a regular file",
                    path.display()
                )));
            }
            Ok(_) => {}
        }
        let mut text = String::new();
        File::open(&path)
            .and_then(|f| {
                f.take(u64::try_from(MAX_FILE_BYTES).unwrap_or(u64::MAX) + 1)
                    .read_to_string(&mut text)
            })
            .map_err(|e| err("cannot read", &path, &e))?;
        Records::decode(&text, self.v4, self.v6).map_err(|e| {
            StoreError(format!(
                "device records in {} are damaged ({e}); refusing every device until they are \
                 repaired or restored (RV-2)",
                path.display()
            ))
        })
    }

    /// Saves the records atomically.
    ///
    /// # Errors
    /// System errors; the previous records are then unchanged.
    pub fn save(&self, records: &Records) -> Result<(), StoreError> {
        self.save_with(records, Interrupt::Never)
    }

    /// [`Store::save`] with a simulated interruption, for tests.
    ///
    /// # Errors
    /// As [`Store::save`]; an interruption reports an error.
    pub fn save_with(&self, records: &Records, interrupt: Interrupt) -> Result<(), StoreError> {
        let temp = self.dir.join(TEMP);
        let path = self.dir.join(RECORDS);
        let _ = fs::remove_file(&temp);
        let mut f = open_private(&temp, true)?;
        f.write_all(records.encode().as_bytes())
            .and_then(|()| f.sync_all())
            .map_err(|e| err("cannot write", &temp, &e))?;
        drop(f);
        if interrupt == Interrupt::BeforeRename {
            return Err(StoreError("simulated interruption before rename".into()));
        }
        fs::rename(&temp, &path).map_err(|e| err("cannot replace", &path, &e))?;
        File::open(&self.dir)
            .and_then(|d| d.sync_all())
            .map_err(|e| err("cannot flush", &self.dir, &e))
    }

    /// Appends one line to the action log: time, action, device name.
    /// Inputs are already validated names; nothing secret is ever logged.
    ///
    /// # Errors
    /// System errors.
    pub fn log(&self, now: u64, action: &str, name: &str) -> Result<(), StoreError> {
        let path = self.dir.join(LOG);
        let mut f = OpenOptions::new()
            .append(true)
            .create(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| err("cannot open", &path, &e))?;
        writeln!(f, "{now} {action} {name}").map_err(|e| err("cannot write", &path, &e))
    }
}

/// Opens a file readable only by its owner, refusing symbolic links.
fn open_private(path: &Path, truncate: bool) -> Result<File, StoreError> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(StoreError(format!(
            "{} is a symbolic link (refused)",
            path.display()
        )));
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(truncate)
        .mode(0o600)
        .open(path)
        .map_err(|e| err("cannot open", path, &e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::Scope;
    use std::os::unix::fs::MetadataExt;

    fn ranges() -> (Prefix, Prefix) {
        (
            "10.77.0.0/24".parse().unwrap(),
            "fd77:77:77::/64".parse().unwrap(),
        )
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("septuvo-store-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn save_and_load_with_private_permissions() {
        let dir = temp_dir("save");
        let (v4, v6) = ranges();
        let store = Store::open(&dir, v4, v6).unwrap();
        assert_eq!(
            store.load().unwrap(),
            Records::default(),
            "missing file: no devices yet"
        );
        let mut r = Records::default();
        r.add("laptop", Scope::Both, 1, v4, v6).unwrap();
        store.save(&r).unwrap();
        assert_eq!(store.load().unwrap(), r);
        assert_eq!(fs::metadata(&dir).unwrap().mode() & 0o777, 0o700);
        assert_eq!(
            fs::metadata(dir.join(RECORDS)).unwrap().mode() & 0o777,
            0o600
        );
        store.log(1, "add", "laptop").unwrap();
        assert_eq!(fs::read_to_string(dir.join(LOG)).unwrap(), "1 add laptop\n");
        assert_eq!(fs::metadata(dir.join(LOG)).unwrap().mode() & 0o777, 0o600);
        drop(store);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_interrupted_save_leaves_the_old_records_intact() {
        let dir = temp_dir("interrupt");
        let (v4, v6) = ranges();
        let store = Store::open(&dir, v4, v6).unwrap();
        let mut r = Records::default();
        r.add("first", Scope::Home, 1, v4, v6).unwrap();
        store.save(&r).unwrap();
        let mut newer = r.clone();
        newer.add("second", Scope::Home, 2, v4, v6).unwrap();
        assert!(store.save_with(&newer, Interrupt::BeforeRename).is_err());
        assert_eq!(store.load().unwrap(), r, "old records, untouched");
        store.save(&newer).unwrap();
        assert_eq!(store.load().unwrap(), newer, "a later save still works");
        drop(store);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn damaged_records_are_refused() {
        let dir = temp_dir("damaged");
        let (v4, v6) = ranges();
        let store = Store::open(&dir, v4, v6).unwrap();
        fs::write(dir.join(RECORDS), "septuvo-devices 1\ndevice x\n").unwrap();
        let e = store.load().unwrap_err();
        assert!(e.0.contains("refusing every device"), "{e}");
        fs::remove_file(dir.join(RECORDS)).unwrap();
        fs::create_dir(dir.join(RECORDS)).unwrap();
        assert!(store.load().is_err(), "a directory is not a records file");
        drop(store);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_second_command_cannot_run_at_the_same_time() {
        let dir = temp_dir("lock");
        let (v4, v6) = ranges();
        let first = Store::open(&dir, v4, v6).unwrap();
        let second = Store::open(&dir, v4, v6);
        assert!(matches!(second, Err(ref e) if e.0.contains("another septuvo-server command")));
        drop(first);
        assert!(
            Store::open(&dir, v4, v6).is_ok(),
            "free again once the first ends"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn symbolic_links_are_refused() {
        let dir = temp_dir("symlink");
        fs::create_dir_all(&dir).unwrap();
        let link = dir.join("data");
        std::os::unix::fs::symlink("/tmp", &link).unwrap();
        let (v4, v6) = ranges();
        assert!(Store::open(&link, v4, v6).is_err());
        let real = dir.join("real");
        let store = Store::open(&real, v4, v6).unwrap();
        std::os::unix::fs::symlink("/etc/passwd", real.join(TEMP)).unwrap();
        assert!(
            store.save(&Records::default()).is_ok(),
            "stale temp entries are replaced, not followed"
        );
        assert!(fs::read_to_string("/etc/passwd").is_ok());
        drop(store);
        fs::remove_dir_all(dir).unwrap();
    }
}
