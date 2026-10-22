//! Service settings (`septuvod.toml`), read with the N2-01 parser.
//!
//! ```toml
//! version = 1
//! [daemon]
//! mode = "lab"                         # the only mode before the G1 review
//! lab_namespace = "sv-lab"             # must be the namespace septuvod runs in
//! runtime_dir = "/run/septuvo"
//! control_group = "septuvo"
//! device_config = "/etc/septuvo/septuvo.toml"
//! captive_helper_user = "septuvo-portal"   # optional: captive-portal mode (N3-02)
//! connections_dir = "/var/lib/septuvo/connections"   # optional (N4-03)
//! ```
//!
//! `device_config` is the configuration used when no saved connection is
//! selected; saved connections live in `connections_dir` (the default
//! shown), a root-owned private directory the service creates if needed.

use septuvo_config::toml::{self, Value};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Largest settings file accepted, in bytes.
pub const MAX_SETTINGS_BYTES: u64 = 16 * 1024;

/// Validated service settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Name of the network namespace the service must be running in
    /// (lab mode, the only mode before G1).
    pub lab_namespace: String,
    /// Directory for the status socket and the control directory.
    pub runtime_dir: PathBuf,
    /// Group whose members may use the control socket.
    pub control_group: String,
    /// The device configuration (N2-01) the service applies.
    pub device_config: PathBuf,
    /// The dedicated account captive-portal windows apply to (N3-02), or
    /// `None`: captive-portal mode is then unavailable.
    pub captive_helper_user: Option<String>,
    /// Where saved connections are stored (N4-03).
    pub connections_dir: PathBuf,
}

/// Default store of saved connections.
pub const DEFAULT_CONNECTIONS_DIR: &str = "/var/lib/septuvo/connections";

/// Why the settings were rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsError(pub String);

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SettingsError {}

fn err<T>(msg: impl Into<String>) -> Result<T, SettingsError> {
    Err(SettingsError(msg.into()))
}

/// A namespace name as `ip netns` accepts it, restricted further:
/// `A-Z a-z 0-9 _ - .`, 1 to 64 characters, not starting with `.` or `-`.
fn valid_namespace(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && !name.starts_with(['.', '-'])
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

/// A group name: lowercase letters, digits, `_` and `-`, 1 to 32 characters,
/// starting with a letter or `_`.
fn valid_group(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b == b'_')
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
}

/// An absolute path without `.`, `..`, `//`, control characters or a
/// trailing slash.
fn plain_absolute(text: &str) -> bool {
    text.len() <= 255
        && !text.chars().any(char::is_control)
        && text.strip_prefix('/').is_some_and(|rest| {
            rest.split('/')
                .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
        })
}

/// Parses and validates settings text.
///
/// # Errors
/// The first problem found.
pub fn parse(text: &str) -> Result<Settings, SettingsError> {
    let doc = toml::parse(text).map_err(|e| SettingsError(e.to_string()))?;
    if doc.root.entries.len() != 1
        || doc.root.get("version").map(|e| &e.value) != Some(&Value::Integer(1))
    {
        return err("the file must start with 'version = 1' and have no other top-level keys");
    }
    let [table] = doc.tables.as_slice() else {
        return err("exactly one [daemon] table is required");
    };
    if table.name != "daemon" {
        return err(format!("unknown table [{}]", table.name));
    }
    let mut mode = None;
    let mut lab_namespace = None;
    let mut runtime_dir = None;
    let mut control_group = None;
    let mut device_config = None;
    let mut captive_helper_user = None;
    let mut connections_dir = None;
    for e in &table.entries {
        let Value::String(v) = &e.value else {
            return err(format!(
                "{}: daemon.{}: expected a string",
                e.value_span, e.key
            ));
        };
        let slot = match e.key.as_str() {
            "mode" => &mut mode,
            "lab_namespace" => &mut lab_namespace,
            "runtime_dir" => &mut runtime_dir,
            "control_group" => &mut control_group,
            "device_config" => &mut device_config,
            "captive_helper_user" => &mut captive_helper_user,
            "connections_dir" => &mut connections_dir,
            _ => return err(format!("{}: daemon.{}: unknown key", e.key_span, e.key)),
        };
        *slot = Some(v.clone());
    }
    match mode.as_deref() {
        Some("lab") => {}
        Some(other) => {
            return err(format!(
                "daemon.mode = \"{other}\": only \"lab\" exists before the G1 security review"
            ));
        }
        None => return err("daemon.mode is required and must be \"lab\""),
    }
    let lab_namespace =
        lab_namespace.ok_or(SettingsError("daemon.lab_namespace is required".into()))?;
    if !valid_namespace(&lab_namespace) {
        return err("daemon.lab_namespace: invalid network namespace name");
    }
    let runtime_dir = runtime_dir.unwrap_or_else(|| septuvo_ipc::DEFAULT_RUNTIME_DIR.to_string());
    if !plain_absolute(&runtime_dir) {
        return err("daemon.runtime_dir must be a plain absolute path");
    }
    let control_group = control_group.unwrap_or_else(|| "septuvo".to_string());
    if !valid_group(&control_group) {
        return err("daemon.control_group: invalid group name");
    }
    let device_config =
        device_config.ok_or(SettingsError("daemon.device_config is required".into()))?;
    if !plain_absolute(&device_config) {
        return err("daemon.device_config must be a plain absolute path");
    }
    let connections_dir = connections_dir.unwrap_or_else(|| DEFAULT_CONNECTIONS_DIR.to_string());
    if !plain_absolute(&connections_dir) || connections_dir == "/" {
        return err("daemon.connections_dir must be a plain absolute path");
    }
    if let Some(user) = &captive_helper_user
        && (!valid_group(user) || user == "root")
    {
        return err("daemon.captive_helper_user: invalid account name (and never root)");
    }
    Ok(Settings {
        lab_namespace,
        runtime_dir: PathBuf::from(runtime_dir),
        control_group,
        device_config: PathBuf::from(device_config),
        captive_helper_user,
        connections_dir: PathBuf::from(connections_dir),
    })
}

/// Reads and validates a settings file (regular file, bounded size).
///
/// # Errors
/// A read error or the first problem found.
pub fn load(path: &Path) -> Result<Settings, SettingsError> {
    let meta = fs::metadata(path).map_err(|e| SettingsError(format!("{}: {e}", path.display())))?;
    if !meta.is_file() || meta.len() > MAX_SETTINGS_BYTES {
        return err(format!(
            "{}: must be a regular file of at most {MAX_SETTINGS_BYTES} bytes",
            path.display()
        ));
    }
    let text =
        fs::read_to_string(path).map_err(|e| SettingsError(format!("{}: {e}", path.display())))?;
    parse(&text).map_err(|e| SettingsError(format!("{}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "version = 1\n[daemon]\nmode = \"lab\"\nlab_namespace = \"sv-c-1\"\n\
                        device_config = \"/etc/septuvo/septuvo.toml\"\n";

    #[test]
    fn minimal_settings_use_defaults() {
        let s = parse(GOOD).expect("valid");
        assert_eq!(s.lab_namespace, "sv-c-1");
        assert_eq!(s.runtime_dir, PathBuf::from("/run/septuvo"));
        assert_eq!(s.control_group, "septuvo");
        assert_eq!(s.device_config, PathBuf::from("/etc/septuvo/septuvo.toml"));
        assert_eq!(s.captive_helper_user, None, "captive-portal mode is opt-in");
        assert_eq!(s.connections_dir, PathBuf::from(DEFAULT_CONNECTIONS_DIR));
        let custom = parse(&format!("{GOOD}connections_dir = \"/tmp/lab/c\"\n")).expect("valid");
        assert_eq!(custom.connections_dir, PathBuf::from("/tmp/lab/c"));
        for bad in ["relative", "/", "/a/../b", "/a//b"] {
            assert!(
                parse(&format!("{GOOD}connections_dir = \"{bad}\"\n")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn captive_helper_user_is_validated() {
        let with = |u: &str| parse(&format!("{GOOD}captive_helper_user = \"{u}\"\n"));
        assert_eq!(
            with("septuvo-portal")
                .unwrap()
                .captive_helper_user
                .as_deref(),
            Some("septuvo-portal")
        );
        for bad in ["root", "", "Portal", "a b", "../x", "x:y", "-x"] {
            assert!(with(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn only_lab_mode_exists_before_g1() {
        let e = parse(&GOOD.replace("\"lab\"", "\"production\"")).expect_err("refused");
        assert!(e.0.contains("only \"lab\""));
        assert!(parse(&GOOD.replace("mode = \"lab\"\n", "")).is_err());
    }

    #[test]
    fn rejects_bad_values_and_unknown_keys() {
        for (from, to) in [
            ("sv-c-1", "-bad"),
            ("sv-c-1", "a/b"),
            ("/etc/septuvo/septuvo.toml", "relative.toml"),
            ("/etc/septuvo/septuvo.toml", "/etc/../shadow"),
            ("mode = \"lab\"", "mode = \"lab\"\nextra = \"x\""),
            (
                "mode = \"lab\"",
                "mode = \"lab\"\ncontrol_group = \"Bad Group\"",
            ),
            (
                "mode = \"lab\"",
                "mode = \"lab\"\nruntime_dir = \"/run//x\"",
            ),
            ("mode = \"lab\"", "mode = 1"),
            ("version = 1", "version = 2"),
            ("[daemon]", "[other]"),
        ] {
            assert!(GOOD.contains(from), "{from}");
            assert!(parse(&GOOD.replacen(from, to, 1)).is_err(), "{to}");
        }
        assert!(parse(&format!("{GOOD}[more]\n")).is_err());
        assert!(parse("version = 1\n").is_err());
    }
}
