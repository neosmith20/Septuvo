//! Start-up guards: the service runs only as root, only in lab mode and only
//! inside the named lab network namespace (N2 work order). It never changes
//! the firewall of a real machine before the G1 review.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// Directory where `ip netns` keeps named network namespaces.
pub const NETNS_DIR: &str = "/run/netns";

/// The effective user ID from the text of `/proc/self/status`.
#[must_use]
pub fn effective_uid(proc_status: &str) -> Option<u32> {
    let line = proc_status.lines().find(|l| l.starts_with("Uid:"))?;
    // Uid: real effective saved filesystem
    line.split_whitespace().nth(2)?.parse().ok()
}

/// True if this process runs as root (effective UID 0). Unreadable means no.
#[must_use]
pub fn running_as_root() -> bool {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| effective_uid(&s))
        == Some(0)
}

/// True if this process's network namespace is the one `ip netns` created
/// under `name`: the same check as `ip netns identify`, comparing the
/// namespace's device and inode with the bind mount `/run/netns/NAME`.
///
/// Fails closed: any error, a missing namespace or a mismatch means no.
#[must_use]
pub fn in_named_namespace(name: &str) -> bool {
    in_named_namespace_at(Path::new(NETNS_DIR), Path::new("/proc/self/ns/net"), name)
}

/// [`in_named_namespace`] with explicit paths, for tests.
#[must_use]
pub fn in_named_namespace_at(netns_dir: &Path, own: &Path, name: &str) -> bool {
    if name.is_empty() || name.contains('/') || name.starts_with('.') {
        return false;
    }
    let (Ok(own), Ok(named)) = (fs::metadata(own), fs::metadata(netns_dir.join(name))) else {
        return false;
    };
    own.dev() == named.dev() && own.ino() == named.ino()
}

/// The numeric ID of group `name`, from the text of `/etc/group`.
#[must_use]
pub fn group_id(etc_group: &str, name: &str) -> Option<u32> {
    etc_group.lines().find_map(|line| {
        let mut parts = line.split(':');
        let group = parts.next()?;
        let _password = parts.next()?;
        let gid = parts.next()?;
        (group == name).then(|| gid.parse().ok()).flatten()
    })
}

/// The user ID of account `name` in `/etc/passwd` text.
#[must_use]
pub fn user_id(etc_passwd: &str, name: &str) -> Option<u32> {
    etc_passwd.lines().find_map(|line| {
        let mut parts = line.split(':');
        let user = parts.next()?;
        let _password = parts.next()?;
        let uid = parts.next()?;
        (user == name).then(|| uid.parse().ok()).flatten()
    })
}

#[cfg(test)]
mod tests {

    #[test]
    fn reads_user_ids() {
        let passwd = "root:x:0:0::/root:/bin/sh\nseptuvo-portal:x:995:995::/:/usr/sbin/nologin\nbad:x:abc:1::/:/bin/sh\n";
        assert_eq!(user_id(passwd, "septuvo-portal"), Some(995));
        assert_eq!(user_id(passwd, "root"), Some(0));
        assert_eq!(user_id(passwd, "bad"), None);
        assert_eq!(user_id(passwd, "septuvo"), None, "exact names only");
    }
    use super::*;

    #[test]
    fn reads_effective_uid() {
        let status = "Name:\tsepvod\nUid:\t1000\t0\t0\t0\nGid:\t0\t0\t0\t0\n";
        assert_eq!(effective_uid(status), Some(0));
        assert_eq!(effective_uid("Uid:\t1000\t1000\t1000\t1000\n"), Some(1000));
        assert_eq!(effective_uid("Name: x\n"), None);
        assert_eq!(effective_uid("Uid:\tabc\n"), None);
    }

    #[test]
    fn finds_group_ids() {
        let group = "root:x:0:\nseptuvo:x:998:alice,bob\nseptuvo-lab:x:1001:\nbroken\n";
        assert_eq!(group_id(group, "septuvo"), Some(998));
        assert_eq!(group_id(group, "septuvo-lab"), Some(1001));
        assert_eq!(group_id(group, "septu"), None);
        assert_eq!(group_id(group, "broken"), None);
        assert_eq!(group_id("x:y:notanumber:\n", "x"), None);
    }

    #[test]
    fn namespace_check_fails_closed() {
        let dir = std::env::temp_dir().join(format!("septuvo-guard-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let own = dir.join("own");
        fs::write(&own, "x").expect("file");
        // A different file: different inode, so not the same namespace.
        fs::write(dir.join("other"), "y").expect("file");
        // A hard link to the same file: same device and inode.
        let _ = fs::remove_file(dir.join("same"));
        fs::hard_link(&own, dir.join("same")).expect("hard link");
        assert!(in_named_namespace_at(&dir, &own, "same"));
        assert!(!in_named_namespace_at(&dir, &own, "other"));
        assert!(!in_named_namespace_at(&dir, &own, "missing"));
        assert!(!in_named_namespace_at(&dir, &own, ""));
        assert!(!in_named_namespace_at(&dir, &own, "../own"));
        assert!(!in_named_namespace_at(
            &dir,
            Path::new("/nonexistent"),
            "same"
        ));
        fs::remove_dir_all(&dir).expect("cleanup");
    }
}
