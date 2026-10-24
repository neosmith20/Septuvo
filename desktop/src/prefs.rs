//! Per-user app preferences (N4-D10): `$XDG_CONFIG_HOME/septuvo/desktop.conf`.
//!
//! Only the app's own presentation choices live here. Connections, the kill
//! switch, Lockdown Mode, Local Network Access and DNS are **service**
//! settings and never stored by the app. The file is small `key = value`
//! text, read with a size bound; unknown keys and bad values fall back to
//! defaults (a damaged file never stops the app). It is written atomically
//! (temporary file, then rename) with mode 0600.

use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

/// Largest preferences file read.
pub const MAX_BYTES: u64 = 16 * 1024;

/// Simple (default) or Advanced (D-037).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterfaceMode {
    /// `Default_VPN_UI.png`.
    #[default]
    Simple,
    /// `Advanced_VPN_UI.png` (N4-02).
    Advanced,
}

/// What the window's X does (D-041).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CloseButton {
    /// Deliberate disconnect, then exit (the default).
    #[default]
    ExitAndDisconnect,
    /// Hide in the system tray, keep the connection (N4-06).
    SendToSystemTray,
}

/// Saved window size and position, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bounds {
    /// Inner width.
    pub width: f32,
    /// Inner height.
    pub height: f32,
    /// Outer position, if the platform reports one.
    pub position: Option<(f32, f32)>,
    /// Whether the window was maximized.
    pub maximized: bool,
}

/// The app's preferences.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Prefs {
    /// Interface Mode.
    pub interface_mode: InterfaceMode,
    /// Close Button Behavior.
    pub close_button: CloseButton,
    /// Last window bounds in Simple mode.
    pub bounds: Option<Bounds>,
    /// Last window bounds in Advanced mode (its own size, ADVANCED-UI §4.2).
    pub advanced_bounds: Option<Bounds>,
    /// A selected draft (N4-05). The selection of a saved connection is the
    /// service's; a draft is the app's own, so the app remembers it.
    pub selected_draft: Option<u32>,
}

fn number(v: &str) -> Option<f32> {
    let n: f32 = v.parse().ok()?;
    (n.is_finite() && (-100_000.0..=100_000.0).contains(&n)).then_some(n)
}

/// One window's bounds while parsing.
#[derive(Default)]
struct Partial {
    width: Option<f32>,
    height: Option<f32>,
    x: Option<f32>,
    y: Option<f32>,
    maximized: bool,
}

impl Partial {
    fn set(&mut self, key: &str, v: &str) {
        match key {
            "width" => self.width = number(v).filter(|n| *n >= 1.0),
            "height" => self.height = number(v).filter(|n| *n >= 1.0),
            "x" => self.x = number(v),
            "y" => self.y = number(v),
            "maximized" => self.maximized = v == "true",
            _ => {}
        }
    }

    /// A size needs both dimensions; a position needs both coordinates.
    fn finish(self) -> Option<Bounds> {
        Some(Bounds {
            width: self.width?,
            height: self.height?,
            position: self.x.zip(self.y),
            maximized: self.maximized,
        })
    }
}

impl Prefs {
    /// Parses preferences text. Never fails: anything unusable is ignored.
    #[must_use]
    pub fn parse(text: &str) -> Prefs {
        let mut p = Prefs::default();
        let mut simple = Partial::default();
        let mut advanced = Partial::default();
        for line in text.lines().take(256) {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match (k.trim(), v.trim()) {
                ("interface_mode", "advanced") => p.interface_mode = InterfaceMode::Advanced,
                ("interface_mode", "simple") => p.interface_mode = InterfaceMode::Simple,
                ("close_button", "send-to-system-tray") => {
                    p.close_button = CloseButton::SendToSystemTray;
                }
                ("close_button", "exit-and-disconnect") => {
                    p.close_button = CloseButton::ExitAndDisconnect;
                }
                ("selected_draft", v) => {
                    p.selected_draft = v.parse().ok().filter(|n| (1..=9999).contains(n));
                }
                (k, v) => {
                    if let Some(rest) = k.strip_prefix("advanced_window_") {
                        advanced.set(rest, v);
                    } else if let Some(rest) = k.strip_prefix("window_") {
                        simple.set(rest, v);
                    }
                }
            }
        }
        p.bounds = simple.finish();
        p.advanced_bounds = advanced.finish();
        p
    }

    /// The text written to disk.
    #[must_use]
    pub fn render(&self) -> String {
        let mut s =
            String::from("# Septuvo VPN app preferences (written by the app)\nversion = 1\n");
        s.push_str(match self.interface_mode {
            InterfaceMode::Simple => "interface_mode = simple\n",
            InterfaceMode::Advanced => "interface_mode = advanced\n",
        });
        s.push_str(match self.close_button {
            CloseButton::ExitAndDisconnect => "close_button = exit-and-disconnect\n",
            CloseButton::SendToSystemTray => "close_button = send-to-system-tray\n",
        });
        if let Some(d) = self.selected_draft {
            let _ = writeln!(s, "selected_draft = {d}");
        }
        for (prefix, bounds) in [
            ("window_", self.bounds),
            ("advanced_window_", self.advanced_bounds),
        ] {
            if let Some(b) = bounds {
                let _ = writeln!(s, "{prefix}width = {}", b.width.round());
                let _ = writeln!(s, "{prefix}height = {}", b.height.round());
                if let Some((x, y)) = b.position {
                    let _ = writeln!(s, "{prefix}x = {}\n{prefix}y = {}", x.round(), y.round());
                }
                let _ = writeln!(s, "{prefix}maximized = {}", b.maximized);
            }
        }
        s
    }

    /// The saved bounds for a mode.
    #[must_use]
    pub fn bounds_for(&self, mode: InterfaceMode) -> Option<Bounds> {
        match mode {
            InterfaceMode::Simple => self.bounds,
            InterfaceMode::Advanced => self.advanced_bounds,
        }
    }

    /// Sets the saved bounds for a mode.
    pub fn set_bounds_for(&mut self, mode: InterfaceMode, b: Bounds) {
        match mode {
            InterfaceMode::Simple => self.bounds = Some(b),
            InterfaceMode::Advanced => self.advanced_bounds = Some(b),
        }
    }

    /// Loads preferences; a missing, unreadable or oversized file gives
    /// the defaults.
    #[must_use]
    pub fn load(path: &Path) -> Prefs {
        match fs::metadata(path) {
            Ok(m) if m.is_file() && m.len() <= MAX_BYTES => fs::read_to_string(path)
                .map(|t| Prefs::parse(&t))
                .unwrap_or_default(),
            _ => Prefs::default(),
        }
    }

    /// Saves preferences atomically, mode 0600, creating the directory.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("conf.tmp");
        {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)?;
            f.write_all(self.render().as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, path)
    }
}

/// `$XDG_CONFIG_HOME/septuvo/desktop.conf`, else `$HOME/.config/septuvo/desktop.conf`.
#[must_use]
pub fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("septuvo").join("desktop.conf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let p = Prefs {
            interface_mode: InterfaceMode::Advanced,
            close_button: CloseButton::SendToSystemTray,
            bounds: Some(Bounds {
                width: 800.0,
                height: 700.0,
                position: Some((10.0, -20.0)),
                maximized: true,
            }),
            advanced_bounds: Some(Bounds {
                width: 980.0,
                height: 640.0,
                position: None,
                maximized: false,
            }),
            selected_draft: Some(3),
        };
        assert_eq!(Prefs::parse(&p.render()), p);
        assert_eq!(Prefs::parse(&Prefs::default().render()), Prefs::default());
    }

    #[test]
    fn damaged_files_give_defaults() {
        let p = Prefs::parse(
            "interface_mode = sideways\nwindow_width = NaN\nwindow_height = 9e99\nrandom\n= =\n",
        );
        assert_eq!(p, Prefs::default());
        let p = Prefs::parse("window_width = 900\n");
        assert_eq!(p.bounds, None, "a size needs both dimensions");
    }

    #[test]
    fn saved_atomically_with_private_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("septuvo-prefs-{}", std::process::id()));
        let path = dir.join("septuvo").join("desktop.conf");
        let p = Prefs {
            interface_mode: InterfaceMode::Advanced,
            ..Prefs::default()
        };
        p.save(&path).unwrap();
        assert_eq!(Prefs::load(&path), p);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::write(&path, "x".repeat(usize::try_from(MAX_BYTES).unwrap() + 1)).unwrap();
        assert_eq!(Prefs::load(&path), Prefs::default());
        let _ = fs::remove_dir_all(&dir);
    }
}
