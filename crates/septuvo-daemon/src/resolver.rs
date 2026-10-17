//! The resolver configuration file (`resolv.conf`) while connected (N2-03).
//!
//! On connect with tunnel DNS, the service points the resolver file at the
//! tunnel resolvers, keeping the original as a backup. A deliberate
//! disconnect restores it. A crash leaves Septuvo's file in place, which,
//! together with the kill switch, keeps failing closed (Q2).
//!
//! In lab mode the file is the one `ip netns exec` gives programs in the lab
//! namespace: `/etc/netns/NAME/resolv.conf`. The host's `/etc/resolv.conf`
//! is never touched before the G1 review.

use std::fs::{self, File, Permissions};
use std::io::{self, Write};
use std::net::IpAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// First line of every file the service writes; how it recognises its own.
pub const MARKER: &str =
    "# Written by septuvod (Septuvo VPN, lab mode): DNS only through the tunnel.";

/// Manages the resolver file.
pub trait Resolver {
    /// Points the resolver file at `servers`, keeping the original once.
    ///
    /// # Errors
    /// Why the file could not be written.
    fn install(&mut self, servers: &[IpAddr]) -> Result<(), String>;
    /// Restores the original file, or removes Septuvo's if there was none.
    ///
    /// # Errors
    /// Why the file could not be restored.
    fn restore(&mut self) -> Result<(), String>;
    /// True if the current file is Septuvo's.
    fn installed(&self) -> bool;
    /// The network's own resolvers (captive-portal mode, N3-02): the
    /// `nameserver` lines of the original file, which is the backup while
    /// Septuvo's file is installed. At most
    /// [`septuvo_policy::nft::MAX_CAPTIVE_RESOLVERS`]; loopback, unspecified
    /// and multicast addresses and anything unparsable are skipped.
    fn network_resolvers(&self) -> Vec<IpAddr>;
}

/// Largest resolver file read when looking for the network's resolvers.
const MAX_ORIGINAL_BYTES: u64 = 64 * 1024;

/// The usable `nameserver` addresses in a resolver file's text.
#[must_use]
pub fn nameservers(text: &str) -> Vec<IpAddr> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            (words.next() == Some("nameserver"))
                .then(|| words.next())
                .flatten()
                .and_then(|w| w.parse::<IpAddr>().ok())
                .filter(|a| !a.is_loopback() && !a.is_unspecified() && !a.is_multicast())
        })
        .take(septuvo_policy::nft::MAX_CAPTIVE_RESOLVERS)
        .collect()
}

/// A `resolv.conf` at a fixed path, with its backup next to it.
#[derive(Debug, Clone)]
pub struct ResolvConf {
    path: PathBuf,
    backup: PathBuf,
    temp: PathBuf,
}

impl ResolvConf {
    /// The file at `path`, with its backup and temporary files alongside it.
    #[must_use]
    pub fn at(path: &Path) -> Self {
        let with = |suffix: &str| {
            let mut name = path.as_os_str().to_owned();
            name.push(suffix);
            PathBuf::from(name)
        };
        ResolvConf {
            path: path.to_path_buf(),
            backup: with(".septuvo-backup"),
            temp: with(".septuvo-tmp"),
        }
    }

    /// The file `ip netns exec NAME` shows as `/etc/resolv.conf`, that is
    /// `/etc/netns/NAME/resolv.conf`. `name` must already be validated (see
    /// `settings`).
    ///
    /// The backup and temporary files live in `/etc/netns` itself, **not**
    /// in `/etc/netns/NAME`: `ip netns exec` bind-mounts every file in that
    /// directory over `/etc`, so extra files there would leak into the lab
    /// namespace's view (found by the N2-03 lab). Same filesystem, so the
    /// final rename stays atomic.
    #[must_use]
    pub fn for_lab_namespace(name: &str) -> Self {
        let netns = Path::new("/etc/netns");
        ResolvConf {
            path: netns.join(name).join("resolv.conf"),
            backup: netns.join(format!(".septuvo-{name}.resolv.conf.backup")),
            temp: netns.join(format!(".septuvo-{name}.resolv.conf.tmp")),
        }
    }

    /// Where the original is kept while Septuvo's file is installed.
    #[must_use]
    pub fn backup_path(&self) -> &Path {
        &self.backup
    }

    /// The managed path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn ours(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|t| t.lines().next() == Some(MARKER))
}

fn io_err(what: &str, path: &Path, e: &io::Error) -> String {
    format!("{what} {}: {e}", path.display())
}

/// The file's text for `servers`.
#[must_use]
pub fn contents(servers: &[IpAddr]) -> String {
    let mut s =
        format!("{MARKER}\n# The original is restored by 'septuvo disconnect'. Do not edit.\n");
    for server in servers {
        s.push_str("nameserver ");
        s.push_str(&server.to_string());
        s.push('\n');
    }
    s
}

impl Resolver for ResolvConf {
    fn install(&mut self, servers: &[IpAddr]) -> Result<(), String> {
        if servers.is_empty() {
            return Err("no tunnel resolvers configured".into());
        }
        for p in [&self.path, &self.backup, &self.temp] {
            if is_symlink(p) {
                return Err(format!(
                    "{} is a symbolic link (refusing to follow it)",
                    p.display()
                ));
            }
        }
        if let Some(dir) = self.path.parent() {
            if is_symlink(dir) {
                return Err(format!(
                    "{} is a symbolic link (refusing to follow it)",
                    dir.display()
                ));
            }
            fs::create_dir_all(dir).map_err(|e| io_err("cannot create", dir, &e))?;
        }
        // Keep the original exactly once: a second connect, or a connect
        // after a crash, must not overwrite the backup with Septuvo's file.
        if exists(&self.path) && !ours(&self.path) && !exists(&self.backup) {
            fs::rename(&self.path, &self.backup)
                .map_err(|e| io_err("cannot back up", &self.path, &e))?;
        }
        let mut f = File::create(&self.temp).map_err(|e| io_err("cannot write", &self.temp, &e))?;
        f.write_all(contents(servers).as_bytes())
            .and_then(|()| f.sync_all())
            .map_err(|e| io_err("cannot write", &self.temp, &e))?;
        fs::set_permissions(&self.temp, Permissions::from_mode(0o644))
            .map_err(|e| io_err("cannot set permissions of", &self.temp, &e))?;
        fs::rename(&self.temp, &self.path).map_err(|e| io_err("cannot replace", &self.path, &e))
    }

    fn restore(&mut self) -> Result<(), String> {
        if exists(&self.backup) {
            if is_symlink(&self.backup) {
                return Err(format!("{} is a symbolic link", self.backup.display()));
            }
            return fs::rename(&self.backup, &self.path)
                .map_err(|e| io_err("cannot restore", &self.path, &e));
        }
        if ours(&self.path) {
            return fs::remove_file(&self.path)
                .map_err(|e| io_err("cannot remove", &self.path, &e));
        }
        Ok(())
    }

    fn installed(&self) -> bool {
        ours(&self.path)
    }

    fn network_resolvers(&self) -> Vec<IpAddr> {
        use std::io::Read;
        let original = if exists(&self.backup) {
            &self.backup
        } else if !ours(&self.path) {
            &self.path
        } else {
            return Vec::new();
        };
        if is_symlink(original) {
            return Vec::new();
        }
        let mut text = String::new();
        let read =
            File::open(original).and_then(|f| f.take(MAX_ORIGINAL_BYTES).read_to_string(&mut text));
        if read.is_ok() {
            nameservers(&text)
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> (PathBuf, ResolvConf) {
        let dir =
            std::env::temp_dir().join(format!("septuvo-resolv-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        let r = ResolvConf::at(&dir.join("resolv.conf"));
        (dir, r)
    }

    fn servers() -> Vec<IpAddr> {
        vec!["10.77.0.1".parse().unwrap(), "fd77::1".parse().unwrap()]
    }

    #[test]
    fn install_backs_up_once_and_restore_brings_back_the_original() {
        let (dir, mut r) = temp("backup");
        fs::write(r.path(), "nameserver 192.168.1.1\n").unwrap();
        r.install(&servers()).unwrap();
        let text = fs::read_to_string(r.path()).unwrap();
        assert!(text.starts_with(MARKER));
        assert!(text.contains("nameserver 10.77.0.1\nnameserver fd77::1\n"));
        assert!(r.installed());
        // Connecting again (or after a crash) keeps the true original.
        r.install(&servers()).unwrap();
        r.restore().unwrap();
        assert_eq!(
            fs::read_to_string(r.path()).unwrap(),
            "nameserver 192.168.1.1\n"
        );
        assert!(!r.installed());
        assert!(!dir.join("resolv.conf.septuvo-backup").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn without_an_original_restore_removes_septuvos_file() {
        let (dir, mut r) = temp("none");
        r.install(&servers()).unwrap();
        assert!(r.installed());
        r.restore().unwrap();
        assert!(!r.path().exists());
        // Restoring when nothing is ours changes nothing.
        fs::write(r.path(), "nameserver 1.1.1.1\n").unwrap();
        r.restore().unwrap();
        assert_eq!(
            fs::read_to_string(r.path()).unwrap(),
            "nameserver 1.1.1.1\n"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn refuses_symbolic_links_and_empty_server_lists() {
        let (dir, mut r) = temp("symlink");
        std::os::unix::fs::symlink("/etc/passwd", r.path()).unwrap();
        assert!(r.install(&servers()).is_err());
        assert!(
            fs::symlink_metadata(r.path())
                .unwrap()
                .file_type()
                .is_symlink()
        );
        fs::remove_file(r.path()).unwrap();
        assert!(r.install(&[]).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn network_resolvers_come_from_the_original_file() {
        let (dir, mut r) = temp("network");
        fs::write(
            r.path(),
            "# comment\nnameserver 192.168.1.1\nnameserver 127.0.0.53\nnameserver   fe80::1\n\
             nameserver bogus\nnameserver 0.0.0.0\nnameserver 224.0.0.251\noptions edns0\n\
             nameserver 192.168.1.2\nnameserver 192.168.1.3\nnameserver 192.168.1.4\n",
        )
        .unwrap();
        let expect: Vec<IpAddr> = ["192.168.1.1", "fe80::1", "192.168.1.2", "192.168.1.3"]
            .iter()
            .map(|a| a.parse().unwrap())
            .collect();
        assert_eq!(
            r.network_resolvers(),
            expect,
            "skips bad entries, at most four"
        );
        r.install(&servers()).unwrap();
        assert_eq!(
            r.network_resolvers(),
            expect,
            "while Septuvo's file is installed, the backup is read"
        );
        r.restore().unwrap();
        fs::remove_file(r.path()).unwrap();
        r.install(&servers()).unwrap();
        assert_eq!(
            r.network_resolvers(),
            Vec::<IpAddr>::new(),
            "no original: none (never the tunnel resolvers)"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn lab_path_is_under_etc_netns_and_side_files_are_outside_the_namespace_dir() {
        let r = ResolvConf::for_lab_namespace("sv-lab");
        assert_eq!(r.path(), Path::new("/etc/netns/sv-lab/resolv.conf"));
        assert_eq!(
            r.backup_path(),
            Path::new("/etc/netns/.septuvo-sv-lab.resolv.conf.backup")
        );
        assert!(!r.backup_path().starts_with("/etc/netns/sv-lab"));
        assert!(!r.temp.starts_with("/etc/netns/sv-lab"));
    }
}
