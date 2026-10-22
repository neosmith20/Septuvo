//! Septuvo VPN desktop application for Linux (N4, development preview).
//!
//! Septuvo-authored code is Rust (D-005): egui/eframe with AccessKit. The
//! only non-Rust code it uses is the operating system's own (display server,
//! keyboard layouts, OpenGL driver, D-Bus), reviewed and listed in
//! `native-allowlist.txt`. The app is an unprivileged viewer and remote
//! control for the `septuvod` service: it never changes the network itself,
//! refuses to run as root, and shows only what the service reports.
//!
//! No cryptography, keys, enrollment or protocol code before the G1
//! security review.

pub mod app;
pub mod assets;
pub mod close;
pub mod icons;
pub mod main_window;
pub mod prefs;
pub mod service;
pub mod shells;
pub mod state;
pub mod theme;

use std::os::unix::fs::MetadataExt;

/// The app's ID for the window manager and desktop entry (reverse DNS of the
/// project's GitHub home: no Septuvo domain is registered or cleared, D-016).
pub const APP_ID: &str = "io.github.neosmith20.SeptuvoVPN";

/// Why the app will not start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused(pub String);

/// The app runs as an ordinary user only: a graphical program running as
/// root would put the whole UI stack in the most privileged account. The
/// service does the privileged work (least privilege, TB-1).
///
/// # Errors
/// `uid` is 0 (root).
pub fn check_user(uid: u32) -> Result<(), Refused> {
    if uid == 0 {
        Err(Refused(
            "Septuvo VPN does not run as root. Start it as your normal user; the Septuvo service \
             does the privileged work. To control it, your user needs to be in the `septuvo` group."
                .into(),
        ))
    } else {
        Ok(())
    }
}

/// The user this process runs as (the owner of `/proc/self`).
///
/// # Errors
/// `/proc` is not available.
pub fn current_uid() -> std::io::Result<u32> {
    Ok(std::fs::metadata("/proc/self")?.uid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_root_only() {
        assert!(check_user(0).is_err());
        assert!(check_user(1000).is_ok());
        assert!(check_user(65534).is_ok());
    }

    #[test]
    fn current_uid_is_readable() {
        assert!(current_uid().is_ok());
    }
}
