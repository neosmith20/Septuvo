//! Incomplete configurations: **drafts** (DEFAULT-UI §4.2, N4-D11).
//!
//! A draft is application-owned data, never sent to the privileged service:
//! only complete, validated configurations are saved there. Drafts live
//! next to the app preferences, in `$XDG_CONFIG_HOME/septuvo/drafts/`, one
//! file per draft (`d0001.toml`), the directory mode 0700 and each file
//! mode 0600, written atomically. A draft file is the editor's text, which
//! may be invalid; reading it is lenient ([`Form::from_text`]) and bounded,
//! and anything unreadable is skipped.

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use crate::form::{Form, MAX_TEXT_BYTES};

/// Most drafts kept.
pub const MAX_DRAFTS: usize = 32;

/// One draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// Its number (`d0001` is 1). Never reused while the app runs.
    pub id: u32,
    /// What the user typed.
    pub form: Form,
}

/// The drafts, in creation order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Drafts {
    dir: Option<PathBuf>,
    items: Vec<Draft>,
}

fn file_id(name: &str) -> Option<u32> {
    let n = name.strip_prefix('d')?.strip_suffix(".toml")?;
    (n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()))
        .then(|| n.parse().ok())
        .flatten()
        .filter(|n| *n > 0)
}

impl Drafts {
    /// Drafts kept only in memory (tests, or no home directory).
    #[must_use]
    pub fn in_memory() -> Drafts {
        Drafts::default()
    }

    /// Loads the drafts in `dir` (missing: none yet).
    #[must_use]
    pub fn load(dir: PathBuf) -> Drafts {
        let mut items = Vec::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for e in entries.flatten().take(4 * MAX_DRAFTS) {
                let name = e.file_name();
                let Some(id) = name.to_str().and_then(file_id) else {
                    continue;
                };
                let path = e.path();
                let ok = fs::symlink_metadata(&path)
                    .is_ok_and(|m| m.is_file() && m.len() <= MAX_TEXT_BYTES as u64);
                if !ok {
                    continue;
                }
                if let Ok(text) = fs::read_to_string(&path) {
                    items.push(Draft {
                        id,
                        form: Form::from_text(&text),
                    });
                }
            }
        }
        items.sort_by_key(|d| d.id);
        items.truncate(MAX_DRAFTS);
        Drafts {
            dir: Some(dir),
            items,
        }
    }

    /// Every draft, in creation order.
    #[must_use]
    pub fn list(&self) -> &[Draft] {
        &self.items
    }

    /// One draft.
    #[must_use]
    pub fn get(&self, id: u32) -> Option<&Draft> {
        self.items.iter().find(|d| d.id == id)
    }

    fn path(&self, id: u32) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(format!("d{id:04}.toml")))
    }

    fn write(dir: &Path, path: &Path, text: &str) -> std::io::Result<()> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        let tmp = path.with_extension("toml.tmp");
        {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)?;
            f.write_all(text.as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, path)
    }

    /// Saves `form` as draft `id`, or as a new draft (`None`). Returns its
    /// number.
    ///
    /// # Errors
    /// Too many drafts, too large, or it cannot be written; nothing changes
    /// then.
    pub fn save(&mut self, id: Option<u32>, form: &Form) -> Result<u32, String> {
        let text = form.to_text();
        if text.len() > MAX_TEXT_BYTES {
            return Err("The configuration is too large.".into());
        }
        let id = if let Some(i) = id.filter(|i| self.get(*i).is_some()) {
            i
        } else {
            if self.items.len() >= MAX_DRAFTS {
                return Err(format!(
                    "At most {MAX_DRAFTS} drafts can be kept. Delete one first."
                ));
            }
            let next = self.items.iter().map(|d| d.id).max().unwrap_or(0) + 1;
            if next > 9999 {
                return Err("No draft numbers are left.".into());
            }
            next
        };
        if let (Some(dir), Some(path)) = (self.dir.clone(), self.path(id)) {
            Self::write(&dir, &path, &text)
                .map_err(|e| format!("The draft could not be saved: {e}."))?;
        }
        let draft = Draft {
            id,
            form: form.clone(),
        };
        match self.items.iter_mut().find(|d| d.id == id) {
            Some(d) => *d = draft,
            None => self.items.push(draft),
        }
        Ok(id)
    }

    /// Deletes a draft.
    ///
    /// # Errors
    /// The file cannot be removed; the draft stays then.
    pub fn delete(&mut self, id: u32) -> Result<(), String> {
        if let Some(path) = self.path(id) {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("The draft could not be deleted: {e}.")),
            }
        }
        self.items.retain(|d| d.id != id);
        Ok(())
    }
}

/// The drafts directory next to the preferences file.
#[must_use]
pub fn dir_for(prefs_path: &Path) -> Option<PathBuf> {
    prefs_path.parent().map(|p| p.join("drafts"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn form(name: &str) -> Form {
        Form {
            name: name.into(),
            server: "203.0.113.7:51820".into(),
            mtu: "abc".into(),
            ..Form::default()
        }
    }

    #[test]
    fn drafts_persist_privately_and_keep_what_was_typed() {
        let dir = std::env::temp_dir().join(format!("septuvo-drafts-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut d = Drafts::load(dir.clone());
        assert_eq!(d.list(), &[] as &[Draft]);
        let a = d.save(None, &form("Office")).unwrap();
        let b = d.save(None, &form("Lab")).unwrap();
        assert_eq!((a, b), (1, 2));
        d.save(Some(a), &form("Office 2")).unwrap();
        let again = Drafts::load(dir.clone());
        assert_eq!(again.list().len(), 2);
        assert_eq!(again.get(1).unwrap().form.name, "Office 2");
        assert_eq!(again.get(1).unwrap().form.mtu, "abc");
        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&dir.join("d0001.toml")), 0o600);
        d.delete(a).unwrap();
        assert!(!dir.join("d0001.toml").exists());
        // Junk in the directory is skipped.
        fs::write(dir.join("d0003.toml"), "x".repeat(MAX_TEXT_BYTES + 1)).unwrap();
        fs::write(dir.join("evil.toml"), "version = 1").unwrap();
        assert_eq!(Drafts::load(dir.clone()).list().len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_number_of_drafts_is_bounded() {
        let mut d = Drafts::in_memory();
        for i in 0..MAX_DRAFTS {
            d.save(None, &form(&format!("n{i}"))).unwrap();
        }
        assert!(d.save(None, &form("one more")).is_err());
    }
}
