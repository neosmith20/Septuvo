//! The service-owned store of saved connections (N4-03, N4-D11).
//!
//! Only complete connections that `septuvo-config` accepts are stored;
//! drafts stay in the app. The store is a root-owned directory (mode 0700):
//!
//! ```text
//! /var/lib/septuvo/connections/
//!   index.toml      order, names, the selected connection, the next ID
//!   c0001.toml      a connection's configuration, without its name
//!   c0002.toml
//!   service.toml    the service settings: Lockdown Mode and Local Network
//!                   Access (N4-04)
//! ```
//!
//! Names live in the index, so renaming never rewrites a configuration;
//! export adds a `[connection]` table with the name back, so an exported
//! file imports again under the same name. Every change is validated first
//! and written atomically (temporary file, `fsync`, rename): a rejected or
//! interrupted change leaves the store as it was. Nothing here is secret:
//! connections hold no keys before G1, and none ever will in this format.

use septuvo_config::toml::{self, Value};
use septuvo_config::{
    DeviceConfig, connection_name, name_table, parse_device, quote, split_connection,
};
use septuvo_ipc::ConnectionId;
use std::fmt::Write as _;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

/// Most saved connections.
pub const MAX_CONNECTIONS: usize = 32;
/// Largest index file read, in bytes.
pub const MAX_INDEX_BYTES: u64 = 16 * 1024;
/// Largest connection file read, in bytes (the protocol's body limit).
pub const MAX_CONNECTION_BYTES: u64 = septuvo_ipc::MAX_BODY_BYTES as u64;

/// Why a store operation was refused, with the protocol error code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError {
    /// Machine-readable code for the response.
    pub code: &'static str,
    /// What went wrong, for people.
    pub message: String,
}

fn fail<T>(code: &'static str, message: impl Into<String>) -> Result<T, StoreError> {
    Err(StoreError {
        code,
        message: message.into(),
    })
}

fn io_err(what: &str, e: &io::Error) -> StoreError {
    StoreError {
        code: "storage",
        message: format!("{what}: {e}"),
    }
}

/// The saved connections.
#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    /// IDs and names, in display order.
    entries: Vec<(ConnectionId, String)>,
    selected: Option<ConnectionId>,
    next: u16,
    /// The service settings (N4-04).
    service: ServiceSettings,
}

/// Service settings the user changes through the control socket and the
/// service enforces (DEFAULT-UI §6; D-038, D-040).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ServiceSettings {
    /// Lockdown Mode (off by default, NP-K1).
    pub lockdown: bool,
    /// Local Network Access is Allowed (Not Allowed by default, NP-L1).
    pub local_network: bool,
}

impl ServiceSettings {
    fn text(self) -> String {
        format!(
            "# Septuvo service settings (written by septuvod; do not edit)\nversion = 1\nlockdown = {}\nlocal_network = \"{}\"\n",
            self.lockdown,
            if self.local_network {
                "allowed"
            } else {
                "not-allowed"
            }
        )
    }

    fn parse(text: &str) -> Result<ServiceSettings, String> {
        let doc = toml::parse(text).map_err(|e| e.to_string())?;
        if !doc.tables.is_empty()
            || doc.root.get("version").map(|e| &e.value) != Some(&Value::Integer(1))
        {
            return Err("unknown format".into());
        }
        let mut out = ServiceSettings::default();
        for e in &doc.root.entries {
            match (e.key.as_str(), &e.value) {
                ("version", _) => {}
                ("lockdown", Value::Boolean(b)) => out.lockdown = *b,
                ("local_network", Value::String(v)) if v == "allowed" => out.local_network = true,
                ("local_network", Value::String(v)) if v == "not-allowed" => {
                    out.local_network = false;
                }
                (k, _) => return Err(format!("unknown or invalid setting '{k}'")),
            }
        }
        Ok(out)
    }
}

fn key(name: &str) -> String {
    name.to_lowercase()
}

impl Store {
    /// Opens the store in `dir`, creating the directory (mode 0700) if it
    /// does not exist. With `require_root`, the directory must belong to
    /// root and be private: anyone else able to write it could change
    /// which networks are protected.
    ///
    /// # Errors
    /// The directory is unusable or the index is damaged. The service then
    /// refuses to start rather than guess (a damaged index is never
    /// silently replaced).
    pub fn open(dir: &Path, require_root: bool) -> Result<Store, String> {
        match fs::symlink_metadata(dir) {
            Ok(meta) => {
                if !meta.file_type().is_dir() {
                    return Err(format!("{} is not a directory", dir.display()));
                }
                if require_root && (meta.uid() != 0 || meta.mode() & 0o077 != 0) {
                    return Err(format!(
                        "{} must belong to root and be private (mode 0700)",
                        dir.display()
                    ));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(dir)
                    .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
            }
            Err(e) => return Err(format!("cannot inspect {}: {e}", dir.display())),
        }
        let mut store = Store {
            dir: dir.to_path_buf(),
            entries: Vec::new(),
            selected: None,
            next: 1,
            service: ServiceSettings::default(),
        };
        let settings = dir.join("service.toml");
        match read_bounded(&settings, MAX_INDEX_BYTES) {
            Ok(text) => {
                store.service = ServiceSettings::parse(&text)
                    .map_err(|why| format!("{}: {why}", settings.display()))?;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("cannot read {}: {e}", settings.display())),
        }
        let index = dir.join("index.toml");
        match read_bounded(&index, MAX_INDEX_BYTES) {
            Ok(text) => store
                .load_index(&text)
                .map_err(|why| format!("{}: {why}", index.display()))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("cannot read {}: {e}", index.display())),
        }
        Ok(store)
    }

    fn load_index(&mut self, text: &str) -> Result<(), String> {
        let doc = toml::parse(text).map_err(|e| e.to_string())?;
        let root = &doc.root;
        if root.get("version").map(|e| &e.value) != Some(&Value::Integer(1)) {
            return Err("unknown index version".into());
        }
        let next = match root.get("next").map(|e| &e.value) {
            Some(Value::Integer(n)) => u16::try_from(*n)
                .ok()
                .filter(|n| (1..=ConnectionId::MAX + 1).contains(n))
                .ok_or("next is out of range")?,
            _ => return Err("next is missing".into()),
        };
        let order: Vec<ConnectionId> = match root.get("order").map(|e| &e.value) {
            Some(Value::Array(items)) => items
                .iter()
                .map(|(s, _)| ConnectionId::parse(s).ok_or(format!("bad ID '{s}'")))
                .collect::<Result<_, _>>()?,
            _ => return Err("order is missing".into()),
        };
        if order.len() > MAX_CONNECTIONS {
            return Err("too many connections".into());
        }
        let selected = match root.get("selected").map(|e| &e.value) {
            None => None,
            Some(Value::String(s)) => {
                Some(ConnectionId::parse(s).ok_or(format!("bad selected ID '{s}'"))?)
            }
            Some(_) => return Err("selected must be a string".into()),
        };
        if root
            .entries
            .iter()
            .any(|e| !matches!(e.key.as_str(), "version" | "next" | "order" | "selected"))
            || doc.tables.iter().any(|t| t.name != "names")
        {
            return Err("unknown key or table".into());
        }
        let names = doc.table("names").ok_or("the [names] table is missing")?;
        let mut entries = Vec::new();
        for id in order {
            if id.number() >= next || entries.iter().any(|(i, _)| *i == id) {
                return Err(format!("{id} is listed twice or beyond next"));
            }
            let name = match names.get(&id.to_string()).map(|e| &e.value) {
                Some(Value::String(n)) => {
                    connection_name(n).map_err(|why| format!("{id}: {why}"))?
                }
                _ => return Err(format!("{id} has no name")),
            };
            if entries
                .iter()
                .any(|(_, n): &(ConnectionId, String)| key(n) == key(&name))
            {
                return Err(format!("the name of {id} is used twice"));
            }
            entries.push((id, name));
        }
        if names.entries.len() != entries.len() {
            return Err("[names] lists a connection that is not in order".into());
        }
        if selected.is_some_and(|s| !entries.iter().any(|(i, _)| *i == s)) {
            return Err("the selected connection does not exist".into());
        }
        self.entries = entries;
        self.selected = selected;
        self.next = next;
        Ok(())
    }

    fn index_text(&self) -> String {
        let mut s = String::from(
            "# Septuvo saved connections (written by septuvod; do not edit)\nversion = 1\n",
        );
        let _ = writeln!(s, "next = {}", self.next);
        if let Some(sel) = self.selected {
            let _ = writeln!(s, "selected = \"{sel}\"");
        }
        let ids: Vec<String> = self
            .entries
            .iter()
            .map(|(i, _)| format!("\"{i}\""))
            .collect();
        let _ = writeln!(s, "order = [{}]", ids.join(", "));
        s.push_str("\n[names]\n");
        for (id, name) in &self.entries {
            let _ = writeln!(s, "{id} = {}", quote(name));
        }
        s
    }

    fn path(&self, id: ConnectionId) -> PathBuf {
        self.dir.join(format!("{id}.toml"))
    }

    fn write_index(&self) -> Result<(), StoreError> {
        write_atomic(&self.dir.join("index.toml"), &self.index_text())
            .map_err(|e| io_err("cannot save the connection list", &e))
    }

    /// The service settings.
    #[must_use]
    pub fn service(&self) -> ServiceSettings {
        self.service
    }

    /// Changes the service settings (saved before this returns).
    ///
    /// # Errors
    /// A write failure; nothing is changed then.
    pub fn set_service(&mut self, settings: ServiceSettings) -> Result<(), StoreError> {
        write_atomic(&self.dir.join("service.toml"), &settings.text())
            .map_err(|e| io_err("cannot save the service settings", &e))?;
        self.service = settings;
        Ok(())
    }

    /// Every saved connection's valid configuration (one that is no longer
    /// valid is skipped).
    #[must_use]
    pub fn configs(&self) -> Vec<DeviceConfig> {
        self.entries
            .iter()
            .filter_map(|(id, _)| self.config(*id).ok())
            .collect()
    }

    /// The connections, in display order.
    #[must_use]
    pub fn list(&self) -> &[(ConnectionId, String)] {
        &self.entries
    }

    /// The selected connection.
    #[must_use]
    pub fn selected(&self) -> Option<ConnectionId> {
        self.selected
    }

    /// A connection's name.
    #[must_use]
    pub fn name(&self, id: ConnectionId) -> Option<&str> {
        self.entries
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, n)| n.as_str())
    }

    fn require(&self, id: ConnectionId) -> Result<&str, StoreError> {
        match self.name(id) {
            Some(n) => Ok(n),
            None => fail("not_found", format!("no saved connection {id}")),
        }
    }

    fn check_unique(&self, name: &str, except: Option<ConnectionId>) -> Result<(), StoreError> {
        match self
            .entries
            .iter()
            .find(|(i, n)| Some(*i) != except && key(n) == key(name))
        {
            Some((other, _)) => fail(
                "duplicate_name",
                format!("another saved connection ({other}) already has this name"),
            ),
            None => Ok(()),
        }
    }

    /// A connection's stored text.
    fn text(&self, id: ConnectionId) -> Result<String, StoreError> {
        self.require(id)?;
        read_bounded(&self.path(id), MAX_CONNECTION_BYTES)
            .map_err(|e| io_err(&format!("cannot read {id}"), &e))
    }

    /// A connection's validated configuration (named).
    ///
    /// # Errors
    /// Not found, unreadable, or no longer valid (for example a file from an
    /// older format): it is then never used.
    pub fn config(&self, id: ConnectionId) -> Result<DeviceConfig, StoreError> {
        let text = self.text(id)?;
        let mut config = parse_device(&text).map_err(|e| StoreError {
            code: "invalid_config",
            message: format!("{id} is not valid: {e}"),
        })?;
        config.name = self.name(id).map(str::to_string);
        Ok(config)
    }

    /// Export text: the stored configuration and a `[connection]` table
    /// with the name.
    ///
    /// # Errors
    /// Not found or unreadable.
    pub fn export(&self, id: ConnectionId) -> Result<String, StoreError> {
        let text = self.text(id)?;
        Ok(format!("{text}{}", name_table(self.require(id)?)))
    }

    fn validate(text: &str) -> Result<(DeviceConfig, String), StoreError> {
        split_connection(text).map_err(|e| StoreError {
            code: "invalid_config",
            message: e.to_string(),
        })
    }

    /// Saves a new connection. `text` must be a complete configuration with
    /// a `[connection]` name. It is not selected.
    ///
    /// # Errors
    /// Invalid, unnamed, duplicate name, store full, or a write failure;
    /// nothing is changed then.
    pub fn add(&mut self, text: &str) -> Result<ConnectionId, StoreError> {
        let (config, stored) = Self::validate(text)?;
        let Some(name) = config.name else {
            return fail(
                "invalid_config",
                "a saved connection needs a name: add [connection] name = \"...\"",
            );
        };
        self.check_unique(&name, None)?;
        if self.entries.len() >= MAX_CONNECTIONS {
            return fail(
                "limit",
                format!("at most {MAX_CONNECTIONS} saved connections"),
            );
        }
        let Some(id) = ConnectionId::new(self.next) else {
            return fail("limit", "no connection IDs are left");
        };
        write_atomic(&self.path(id), &stored)
            .map_err(|e| io_err("cannot save the connection", &e))?;
        self.next += 1;
        self.entries.push((id, name));
        if let Err(e) = self.write_index() {
            self.entries.pop();
            self.next -= 1;
            let _ = fs::remove_file(self.path(id));
            return Err(e);
        }
        Ok(id)
    }

    /// Replaces a connection's configuration. A `[connection]` name in the
    /// text renames it too; without one the name is kept.
    ///
    /// # Errors
    /// Not found, invalid, duplicate name or a write failure; nothing is
    /// changed then.
    pub fn replace(&mut self, id: ConnectionId, text: &str) -> Result<DeviceConfig, StoreError> {
        self.require(id)?;
        let (mut config, stored) = Self::validate(text)?;
        if let Some(name) = &config.name {
            self.check_unique(name, Some(id))?;
        }
        let old_name = self.require(id)?.to_string();
        write_atomic(&self.path(id), &stored)
            .map_err(|e| io_err("cannot save the connection", &e))?;
        if let Some(name) = config.name.clone().filter(|n| *n != old_name) {
            self.set_name(id, name)?;
        }
        config.name = Some(self.require(id)?.to_string());
        Ok(config)
    }

    fn set_name(&mut self, id: ConnectionId, name: String) -> Result<(), StoreError> {
        let Some(slot) = self.entries.iter_mut().find(|(i, _)| *i == id) else {
            return fail("not_found", format!("no saved connection {id}"));
        };
        let old = std::mem::replace(&mut slot.1, name);
        if let Err(e) = self.write_index() {
            if let Some(slot) = self.entries.iter_mut().find(|(i, _)| *i == id) {
                slot.1 = old;
            }
            return Err(e);
        }
        Ok(())
    }

    /// Renames a connection: the name only, no network effect.
    ///
    /// # Errors
    /// Not found, invalid or duplicate name, or a write failure.
    pub fn rename(&mut self, id: ConnectionId, name: &str) -> Result<String, StoreError> {
        self.require(id)?;
        let name = connection_name(name).map_err(|why| StoreError {
            code: "invalid_name",
            message: why,
        })?;
        self.check_unique(&name, Some(id))?;
        self.set_name(id, name.clone())?;
        Ok(name)
    }

    /// Selects a connection (remembered across restarts).
    ///
    /// # Errors
    /// Not found or a write failure.
    pub fn select(&mut self, id: ConnectionId) -> Result<(), StoreError> {
        self.require(id)?;
        let old = self.selected.replace(id);
        if let Err(e) = self.write_index() {
            self.selected = old;
            return Err(e);
        }
        Ok(())
    }

    /// Deletes a connection. If it was selected, the next remaining one
    /// (in display order) is selected, or none (DEFAULT-UI §4.1). Returns
    /// the new selection.
    ///
    /// # Errors
    /// Not found or a write failure; nothing is changed then.
    pub fn delete(&mut self, id: ConnectionId) -> Result<Option<ConnectionId>, StoreError> {
        self.require(id)?;
        let old_entries = self.entries.clone();
        let old_selected = self.selected;
        let pos = self.entries.iter().position(|(i, _)| *i == id).unwrap_or(0);
        self.entries.remove(pos);
        if self.selected == Some(id) {
            self.selected = self
                .entries
                .get(pos)
                .or_else(|| self.entries.last())
                .map(|(i, _)| *i);
        }
        if let Err(e) = self.write_index() {
            self.entries = old_entries;
            self.selected = old_selected;
            return Err(e);
        }
        // The index no longer lists it; a file left behind is unused.
        let _ = fs::remove_file(self.path(id));
        Ok(self.selected)
    }
}

/// Reads at most `limit` bytes of a regular file (never follows a symlink
/// to something else, never reads a device).
fn read_bounded(path: &Path, limit: u64) -> io::Result<String> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a regular file",
        ));
    }
    if meta.len() > limit {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }
    let mut text = String::new();
    File::open(path)?
        .take(limit + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > limit {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }
    Ok(text)
}

/// Writes `text` to `path` atomically: a temporary file (mode 0600) in the
/// same directory, flushed to disk, then renamed over the target.
fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    // A leftover from an interrupted write (or anything planted there) is
    // removed first; create_new then refuses to follow a symlink or reuse a
    // file (O_CREAT | O_EXCL).
    match fs::remove_file(&tmp) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    if let Some(dir) = path.parent() {
        File::open(dir)?.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONN: &str = "version = 1\n[server]\nendpoints = [\"10.200.0.1:5000\"]\n[routing]\nmode = \"full-tunnel\"\n[dns]\nresolvers = [\"10.77.0.1\"]\n";

    fn named(name: &str) -> String {
        format!("{CONN}{}", name_table(name))
    }

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("septuvo-store-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn add_list_get_rename_select_delete() {
        let dir = temp("cycle");
        let mut s = Store::open(&dir, false).expect("open");
        assert_eq!(s.list(), &[] as &[(ConnectionId, String)]);
        let home = s.add(&named("Home VPN")).expect("add");
        let work = s.add(&named("Büro \"Main\"")).expect("add unicode");
        assert_eq!(home.to_string(), "c0001");
        assert_eq!(s.list().len(), 2);
        // Names are unique, case-insensitively.
        assert_eq!(
            s.add(&named("home vpn")).map_err(|e| e.code),
            Err("duplicate_name")
        );
        assert_eq!(
            s.add(CONN).map_err(|e| e.code),
            Err("invalid_config"),
            "a name is required"
        );
        // Export carries the name and imports again.
        let exported = s.export(work).expect("export");
        assert!(exported.contains("[connection]"));
        let mut other = Store::open(&temp("import"), false).expect("open");
        let again = other.add(&exported).expect("re-import");
        assert_eq!(other.name(again), Some("Büro \"Main\""));
        assert_eq!(s.rename(home, "  Home  ").as_deref(), Ok("Home"));
        assert_eq!(
            s.rename(home, "BÜRO \"main\"").map_err(|e| e.code),
            Err("duplicate_name")
        );
        assert_eq!(s.rename(home, "").map_err(|e| e.code), Err("invalid_name"));
        s.select(work).expect("select");
        // Everything survives a restart.
        let s2 = Store::open(&dir, false).expect("reopen");
        assert_eq!(s2.list(), s.list());
        assert_eq!(s2.selected(), Some(work));
        assert_eq!(
            s2.config(home).expect("config").name.as_deref(),
            Some("Home")
        );
        // Deleting the selected one selects the next remaining.
        assert_eq!(s.delete(work), Ok(Some(home)));
        assert_eq!(s.delete(home), Ok(None));
        assert!(!s.path(home).exists());
        // IDs are never reused.
        assert_eq!(
            s.add(&named("New")).map(|i| i.to_string()).as_deref(),
            Ok("c0003")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bad_connection_changes_nothing() {
        let dir = temp("bad");
        let mut s = Store::open(&dir, false).expect("open");
        let id = s.add(&named("Home")).expect("add");
        let before = (
            fs::read_to_string(dir.join("index.toml")).unwrap(),
            s.export(id).unwrap(),
        );
        let bad = [
            "version = 1\n".to_string(),
            named("x").replace("10.77.0.1", "10.77.0.999"),
            format!("{CONN}[behavior]\nalways_on = true\n[connection]\nname = \"y\"\n"),
            named("y").replace("[dns]", "[dns]\nmode = \"system\""),
        ];
        for text in &bad {
            assert_eq!(
                s.add(text).map_err(|e| e.code),
                Err("invalid_config"),
                "{text}"
            );
            assert_eq!(
                s.replace(id, text).map_err(|e| e.code),
                Err("invalid_config"),
                "{text}"
            );
        }
        let missing = ConnectionId::new(99).unwrap();
        assert_eq!(s.select(missing).map_err(|e| e.code), Err("not_found"));
        assert_eq!(s.delete(missing).map_err(|e| e.code), Err("not_found"));
        let after = (
            fs::read_to_string(dir.join("index.toml")).unwrap(),
            s.export(id).unwrap(),
        );
        assert_eq!(before, after);
        assert_eq!(s.list().len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replace_keeps_or_changes_the_name() {
        let dir = temp("replace");
        let mut s = Store::open(&dir, false).expect("open");
        let id = s.add(&named("Home")).expect("add");
        let other = s.add(&named("Work")).expect("add");
        let c = s
            .replace(id, &CONN.replace("5000", "5001"))
            .expect("replace");
        assert_eq!(c.name.as_deref(), Some("Home"));
        assert!(s.export(id).unwrap().contains("5001"));
        assert_eq!(
            s.replace(id, &named("Work")).map_err(|e| e.code),
            Err("duplicate_name")
        );
        assert_eq!(
            s.replace(id, &named("Home 2")).unwrap().name.as_deref(),
            Some("Home 2")
        );
        assert_eq!(s.name(other), Some("Work"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn damaged_or_unsafe_stores_are_refused() {
        let dir = temp("damaged");
        fs::create_dir_all(&dir).unwrap();
        for bad in [
            "version = 2\nnext = 1\norder = []\n[names]\n",
            "version = 1\nnext = 2\norder = [\"c0001\", \"c0001\"]\n[names]\nc0001 = \"a\"\n",
            "version = 1\nnext = 2\norder = [\"c0001\"]\n[names]\n",
            "version = 1\nnext = 1\norder = [\"c0001\"]\n[names]\nc0001 = \"a\"\n",
            "version = 1\nnext = 3\norder = [\"c0001\", \"c0002\"]\n[names]\nc0001 = \"a\"\nc0002 = \"A\"\n",
            "version = 1\nnext = 2\nselected = \"c0005\"\norder = [\"c0001\"]\n[names]\nc0001 = \"a\"\n",
            "version = 1\nnext = 2\norder = [\"c0001\"]\nextra = 1\n[names]\nc0001 = \"a\"\n",
        ] {
            fs::write(dir.join("index.toml"), bad).unwrap();
            assert!(Store::open(&dir, false).is_err(), "{bad}");
        }
        let _ = fs::remove_dir_all(&dir);
        // A file where the directory should be.
        let file = temp("file");
        fs::write(&file, "x").unwrap();
        assert!(Store::open(&file, false).is_err());
        let _ = fs::remove_file(&file);
    }

    #[test]
    fn the_store_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp("mode");
        let mut s = Store::open(&dir, false).expect("open");
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        let id = s.add(&named("Home")).expect("add");
        assert_eq!(
            fs::metadata(s.path(id)).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(
            Store::open(&dir, true).is_err(),
            "a readable or non-root store is refused"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn service_settings_persist_and_damage_is_refused() {
        let dir = temp("service");
        let mut s = Store::open(&dir, false).expect("open");
        assert_eq!(
            s.service(),
            ServiceSettings::default(),
            "both off by default"
        );
        let on = ServiceSettings {
            lockdown: true,
            local_network: true,
        };
        s.set_service(on).expect("save");
        assert_eq!(Store::open(&dir, false).expect("reopen").service(), on);
        for bad in [
            "version = 1\nlockdown = \"yes\"\n",
            "version = 1\nlocal_network = \"maybe\"\n",
            "version = 2\n",
            "version = 1\n[x]\n",
        ] {
            fs::write(dir.join("service.toml"), bad).unwrap();
            assert!(Store::open(&dir, false).is_err(), "{bad}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_store_is_bounded() {
        let dir = temp("limit");
        let mut s = Store::open(&dir, false).expect("open");
        for i in 0..MAX_CONNECTIONS {
            s.add(&named(&format!("C{i}"))).expect("add");
        }
        assert_eq!(s.add(&named("one more")).map_err(|e| e.code), Err("limit"));
        let _ = fs::remove_dir_all(&dir);
    }
}
