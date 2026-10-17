//! The `septuvo-server` commands, separated from `main` so they can be tested.

use crate::records::{Device, Records, Scope, valid_name};
use crate::store::Store;
use septuvo_config::ServerConfig;
use std::fmt::Write as _;
use std::path::Path;

/// Shown whenever a device is added or shown.
pub const CANNOT_CONNECT: &str = "This device cannot connect yet: invitations, enrollment and keys \
are not available before the G1 security review.";

/// Usage text.
pub const USAGE: &str = "\
septuvo-server - Septuvo Server administration (development preview)

No device can connect yet: there are no invitations, enrollment or keys
before the G1 security review, and no network port is opened.

Usage:
  septuvo-server --config FILE config check
  septuvo-server --config FILE device add NAME --scope home|exit|both
  septuvo-server --config FILE device list
  septuvo-server --config FILE device show NAME
  septuvo-server --config FILE device revoke NAME
  septuvo-server --config FILE device remove NAME   (revoked devices only)

Names: 1-32 characters from a-z, 0-9, '-', '_' and '.'.
Exit codes: 0 success, 1 error, 2 usage error.
";

/// The outcome of a command: exit code and text for standard output or
/// standard error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Process exit code.
    pub code: u8,
    /// Text for standard output.
    pub out: String,
    /// Text for standard error.
    pub err: String,
}

fn ok(out: String) -> Outcome {
    Outcome {
        code: 0,
        out,
        err: String::new(),
    }
}

fn fail(code: u8, err: impl Into<String>) -> Outcome {
    Outcome {
        code,
        out: String::new(),
        err: err.into(),
    }
}

fn describe(d: &Device) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "name:      {}", d.name);
    let _ = writeln!(s, "scope:     {}", d.scope.as_str());
    let _ = writeln!(s, "ipv4:      {}", d.ipv4);
    let _ = writeln!(s, "ipv6:      {}", d.ipv6);
    let _ = writeln!(s, "created:   {} (seconds since 1970, UTC)", d.created);
    match d.revoked {
        Some(t) => {
            let _ = writeln!(s, "status:    revoked at {t}");
        }
        None => {
            let _ = writeln!(s, "status:    enrolled record (cannot connect before G1)");
        }
    }
    let _ = writeln!(s, "key:       {}", d.key);
    s
}

/// Runs a device command with the data directory locked.
fn with_store(
    config: &ServerConfig,
    f: impl FnOnce(&Store, Records) -> Result<String, String>,
) -> Outcome {
    let store = match Store::open(&config.data_dir, config.ipv4_range, config.ipv6_range) {
        Ok(s) => s,
        Err(e) => return fail(1, e.to_string()),
    };
    let records = match store.load() {
        Ok(r) => r,
        Err(e) => return fail(1, e.to_string()),
    };
    match f(&store, records) {
        Ok(out) => ok(out),
        Err(e) => fail(1, e),
    }
}

/// Runs `args` (without the program name) at time `now`.
#[must_use]
pub fn run(args: &[String], now: u64) -> Outcome {
    let strs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (config_path, rest) = match strs.as_slice() {
        ["--help" | "-h" | "help"] => return ok(USAGE.to_string()),
        ["--config", path, rest @ ..] => (Path::new(*path), rest),
        _ => return fail(2, USAGE),
    };
    let config = match septuvo_config::load_server(config_path) {
        Ok(c) => c,
        Err(e) => return fail(1, format!("{}: {e}", config_path.display())),
    };
    match rest {
        ["config", "check"] => ok(format!(
            "{}: valid server configuration\n{}\n",
            config_path.display(),
            config.summary()
        )),
        ["device", "add", name, "--scope", scope] => {
            let Some(scope) = Scope::parse(scope) else {
                return fail(2, "scope must be home, exit or both");
            };
            with_store(&config, |store, mut records| {
                let device = records
                    .add(name, scope, now, config.ipv4_range, config.ipv6_range)?
                    .clone();
                store.save(&records).map_err(|e| e.to_string())?;
                store.log(now, "add", name).map_err(|e| e.to_string())?;
                Ok(format!(
                    "added device\n{}{CANNOT_CONNECT}\n",
                    describe(&device)
                ))
            })
        }
        ["device", "list"] => with_store(&config, |_, records| {
            let mut s = String::new();
            if records.devices().is_empty() {
                s.push_str("no devices\n");
            }
            for d in records.devices() {
                let status = if d.revoked.is_some() {
                    "revoked"
                } else {
                    "enrolled"
                };
                let _ = writeln!(
                    s,
                    "{:<32} {:<5} {:<15} {:<39} {status}",
                    d.name,
                    d.scope.as_str(),
                    d.ipv4,
                    d.ipv6
                );
            }
            Ok(s)
        }),
        ["device", "show", name] => with_store(&config, |_, records| {
            records
                .get(name)
                .map(|d| format!("{}{CANNOT_CONNECT}\n", describe(d)))
                .ok_or_else(|| format!("no device called '{}'", shown(name)))
        }),
        ["device", "revoke", name] => with_store(&config, |store, mut records| {
            records
                .revoke(name, now)
                .map_err(|_| revoke_error(&records, name))?;
            store.save(&records).map_err(|e| e.to_string())?;
            store.log(now, "revoke", name).map_err(|e| e.to_string())?;
            Ok(format!(
                "revoked '{name}'; its addresses stay reserved until it is removed\n"
            ))
        }),
        ["device", "remove", name] => with_store(&config, |store, mut records| {
            records.remove(name).map_err(|e| {
                if valid_name(name) {
                    e
                } else {
                    format!("no device called '{}'", shown(name))
                }
            })?;
            store.save(&records).map_err(|e| e.to_string())?;
            store.log(now, "remove", name).map_err(|e| e.to_string())?;
            Ok(format!("removed '{name}'\n"))
        }),
        _ => fail(2, USAGE),
    }
}

fn revoke_error(records: &Records, name: &str) -> String {
    match records.get(name) {
        Some(_) => format!("'{name}' is already revoked"),
        None => format!("no device called '{}'", shown(name)),
    }
}

/// A name for messages: printable characters only, bounded.
fn shown(name: &str) -> String {
    name.chars().filter(|c| !c.is_control()).take(40).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct Lab {
        dir: PathBuf,
        config: String,
    }

    impl Lab {
        fn new(name: &str) -> Lab {
            let dir = std::env::temp_dir()
                .join(format!("septuvo-server-cli-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            let config = dir.join("server.toml");
            fs::write(
                &config,
                format!(
                    "version = 1\n[listen]\nport = 51820\n[network]\nipv4_range = \"10.77.0.0/29\"\n\
                     ipv6_range = \"fd77:77:77::/64\"\n[storage]\ndata_dir = \"{}\"\n",
                    dir.join("data").display()
                ),
            )
            .unwrap();
            Lab {
                dir,
                config: config.to_string_lossy().into_owned(),
            }
        }

        fn run(&self, args: &[&str], now: u64) -> Outcome {
            let mut all = vec!["--config".to_string(), self.config.clone()];
            all.extend(args.iter().map(ToString::to_string));
            run(&all, now)
        }
    }

    impl Drop for Lab {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn full_device_lifecycle() {
        let lab = Lab::new("lifecycle");
        let out = lab.run(&["device", "add", "laptop", "--scope", "both"], 100);
        assert_eq!(out.code, 0, "{}", out.err);
        assert!(out.out.contains("ipv4:      10.77.0.2"));
        assert!(out.out.contains("key:       not a key"));
        assert!(out.out.contains(CANNOT_CONNECT));
        assert_eq!(
            lab.run(&["device", "add", "phone", "--scope", "home"], 101)
                .code,
            0
        );
        let list = lab.run(&["device", "list"], 102).out;
        assert!(list.contains("laptop") && list.contains("phone") && list.contains("enrolled"));
        assert_eq!(
            lab.run(&["device", "remove", "laptop"], 103).code,
            1,
            "must revoke first"
        );
        assert_eq!(lab.run(&["device", "revoke", "laptop"], 104).code, 0);
        let shown = lab.run(&["device", "show", "laptop"], 105).out;
        assert!(shown.contains("revoked at 104"));
        assert_eq!(lab.run(&["device", "revoke", "laptop"], 106).code, 1);
        assert_eq!(lab.run(&["device", "remove", "laptop"], 107).code, 0);
        assert_eq!(lab.run(&["device", "show", "laptop"], 108).code, 1);
        let log = fs::read_to_string(lab.dir.join("data/actions.log")).unwrap();
        assert_eq!(
            log,
            "100 add laptop\n101 add phone\n104 revoke laptop\n107 remove laptop\n"
        );
    }

    #[test]
    fn address_range_exhaustion_is_reported() {
        let lab = Lab::new("exhaust");
        // 10.77.0.0/29: .2 to .6 are usable (5 devices).
        for i in 0..5 {
            assert_eq!(
                lab.run(&["device", "add", &format!("d{i}"), "--scope", "home"], 1)
                    .code,
                0
            );
        }
        let out = lab.run(&["device", "add", "d5", "--scope", "home"], 1);
        assert_eq!(out.code, 1);
        assert!(out.err.contains("no free IPv4"));
    }

    #[test]
    fn damaged_records_refuse_every_command() {
        let lab = Lab::new("damaged");
        assert_eq!(
            lab.run(&["device", "add", "a", "--scope", "home"], 1).code,
            0
        );
        fs::write(lab.dir.join("data/devices.db"), "corrupt\n").unwrap();
        for args in [
            &["device", "list"][..],
            &["device", "add", "b", "--scope", "home"],
            &["device", "show", "a"],
        ] {
            let out = lab.run(args, 2);
            assert_eq!(out.code, 1, "{args:?}");
            assert!(out.err.contains("refusing every device"), "{}", out.err);
        }
        assert_eq!(
            fs::read_to_string(lab.dir.join("data/devices.db")).unwrap(),
            "corrupt\n",
            "never overwritten"
        );
    }

    #[test]
    fn usage_and_hostile_input() {
        let lab = Lab::new("usage");
        assert_eq!(run(&[], 0).code, 2);
        assert_eq!(run(&["--help".into()], 0).code, 0);
        assert!(
            run(&["--help".into()], 0)
                .out
                .contains("No device can connect yet")
        );
        assert_eq!(
            lab.run(&["device", "add", "x", "--scope", "admin"], 0).code,
            2
        );
        assert_eq!(lab.run(&["device", "add", "x"], 0).code, 2);
        assert_eq!(
            lab.run(&["device", "invite", "x"], 0).code,
            2,
            "no invitations before G1"
        );
        let out = lab.run(
            &["device", "add", "Bad Name\u{1b}[2J", "--scope", "home"],
            0,
        );
        assert_eq!(out.code, 1);
        let out = lab.run(&["device", "show", "evil\u{1b}[2J"], 0);
        assert!(
            !out.err.contains('\u{1b}'),
            "control characters are never echoed"
        );
        let bad_config = run(
            &[
                "--config".into(),
                "/nonexistent/server.toml".into(),
                "device".into(),
                "list".into(),
            ],
            0,
        );
        assert_eq!(bad_config.code, 1);
    }

    #[test]
    fn config_check_summarizes() {
        let lab = Lab::new("check");
        let out = lab.run(&["config", "check"], 0);
        assert_eq!(out.code, 0, "{}", out.err);
        assert!(out.out.contains("no port is opened"));
    }
}
