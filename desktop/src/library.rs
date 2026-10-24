//! The saved connections and drafts, as the app shows them (N4-05).
//!
//! **The service is authoritative** for saved connections: their names,
//! order and the selection come from its `LIST` answer, their settings from
//! `GET`. Nothing is assumed: until the service answers, the list is
//! unknown, and a user outside the `septuvo` group sees names only.
//! **Drafts** are the app's own ([`Drafts`]); selecting one is remembered
//! by the app, and it can never be connected.

use septuvo_ipc::{ConnectionId, Response};

use crate::drafts::Drafts;
use crate::form::Form;

/// A connection or a draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A saved connection (service).
    Saved(ConnectionId),
    /// A draft (app).
    Draft(u32),
}

/// A saved connection as reported by the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    /// Its ID.
    pub id: ConnectionId,
    /// Its name.
    pub name: String,
    /// Its configuration text with the name (from `GET`), once known.
    pub text: Option<String>,
}

impl Saved {
    /// The server address, if the configuration is known.
    #[must_use]
    pub fn server(&self) -> Option<String> {
        let f = Form::from_text(self.text.as_deref()?);
        (!f.server.is_empty()).then_some(f.server)
    }
}

/// One row of the profile selector or the Connections manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Which connection.
    pub key: Key,
    /// Its friendly name (plain text).
    pub name: String,
    /// The second line: the server address, `Setup Required` for a draft,
    /// or nothing when the settings cannot be read.
    pub detail: String,
    /// The selected connection.
    pub selected: bool,
    /// The connection the service is using while connected.
    pub in_use: bool,
}

/// Saved connections, drafts and the selection.
#[derive(Debug, Clone, Default)]
pub struct Library {
    /// Saved connections, in the service's order.
    pub saved: Vec<Saved>,
    /// The service's selection.
    pub service_selected: Option<ConnectionId>,
    /// The service has answered `LIST` at least once.
    pub loaded: bool,
    /// The last `LIST` failed: why.
    pub error: Option<String>,
    /// The app's drafts.
    pub drafts: Drafts,
    /// A draft the user selected (app-only; it can never be connected).
    pub draft_selected: Option<u32>,
}

/// Text from the service: printable characters only, bounded.
fn clean(s: &str) -> String {
    crate::state::plain(s)
}

impl Library {
    /// A library with these drafts and nothing known from the service yet.
    #[must_use]
    pub fn new(drafts: Drafts, draft_selected: Option<u32>) -> Library {
        let draft_selected = draft_selected.filter(|d| drafts.get(*d).is_some());
        Library {
            drafts,
            draft_selected,
            ..Library::default()
        }
    }

    /// Takes a `LIST` answer. Returns the connections whose settings are
    /// not known yet (to `GET`).
    pub fn apply_list(&mut self, r: &Response) -> Vec<ConnectionId> {
        let mut saved = Vec::new();
        for line in r.body.as_deref().unwrap_or("").lines().take(64) {
            let Some((id, name)) = line.split_once('\t') else {
                continue;
            };
            let Some(id) = ConnectionId::parse(id) else {
                continue;
            };
            let text = self
                .saved
                .iter()
                .find(|s| s.id == id)
                .and_then(|s| s.text.clone());
            saved.push(Saved {
                id,
                name: clean(name),
                text,
            });
        }
        self.saved = saved;
        self.service_selected = r.field("selected").and_then(ConnectionId::parse);
        self.loaded = true;
        self.error = None;
        self.saved
            .iter()
            .filter(|s| s.text.is_none())
            .map(|s| s.id)
            .collect()
    }

    /// Takes a `GET` answer.
    pub fn apply_get(&mut self, id: ConnectionId, r: &Response) {
        if let Some(s) = self.saved.iter_mut().find(|s| s.id == id) {
            s.text.clone_from(&r.body);
        }
    }

    /// Forgets a connection's settings (changed: `GET` it again).
    pub fn forget(&mut self, id: ConnectionId) {
        if let Some(s) = self.saved.iter_mut().find(|s| s.id == id) {
            s.text = None;
        }
    }

    /// The selection: a selected draft first, else the service's.
    #[must_use]
    pub fn selected(&self) -> Option<Key> {
        match self.draft_selected {
            Some(d) if self.drafts.get(d).is_some() => Some(Key::Draft(d)),
            _ => self
                .service_selected
                .filter(|id| self.saved.iter().any(|s| s.id == *id))
                .map(Key::Saved),
        }
    }

    /// A connection's name.
    #[must_use]
    pub fn name(&self, key: Key) -> Option<String> {
        match key {
            Key::Saved(id) => self
                .saved
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.name.clone()),
            Key::Draft(d) => self.drafts.get(d).map(|d| d.form.name.trim().to_string()),
        }
    }

    /// A saved connection.
    #[must_use]
    pub fn saved(&self, id: ConnectionId) -> Option<&Saved> {
        self.saved.iter().find(|s| s.id == id)
    }

    /// True if another connection or draft (not `except`) already uses
    /// `name`, regardless of case (DEFAULT-UI §3.2).
    #[must_use]
    pub fn name_taken(&self, name: &str, except: Option<Key>) -> bool {
        let lower = name.trim().to_lowercase();
        let saved = self
            .saved
            .iter()
            .any(|s| except != Some(Key::Saved(s.id)) && s.name.to_lowercase() == lower);
        let drafts = self.drafts.list().iter().any(|d| {
            except != Some(Key::Draft(d.id)) && d.form.name.trim().to_lowercase() == lower
        });
        saved || drafts
    }

    /// Every connection, then every draft, for the selector and the
    /// Connections manager. `in_use` is the connection the service reports
    /// it is using while connected.
    #[must_use]
    pub fn entries(&self, in_use: Option<ConnectionId>) -> Vec<Entry> {
        let selected = self.selected();
        let mut out: Vec<Entry> = self
            .saved
            .iter()
            .map(|s| Entry {
                key: Key::Saved(s.id),
                name: s.name.clone(),
                detail: s.server().unwrap_or_default(),
                selected: selected == Some(Key::Saved(s.id)),
                in_use: in_use == Some(s.id),
            })
            .collect();
        out.extend(self.drafts.list().iter().map(|d| Entry {
            key: Key::Draft(d.id),
            name: clean(d.form.name.trim()),
            detail: "Draft · Setup Required".into(),
            selected: selected == Some(Key::Draft(d.id)),
            in_use: false,
        }));
        out
    }
}

/// A file name for an export: the friendly name with characters that are
/// unsafe in file names replaced, plus `.toml` (DEFAULT-UI §4.4).
#[must_use]
pub fn export_file_name(name: &str) -> String {
    let mut s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    s = s.trim_start_matches('.').trim().to_string();
    if s.is_empty() {
        s = "Septuvo connection".into();
    }
    format!("{s}.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u16) -> ConnectionId {
        ConnectionId::new(n).unwrap()
    }

    fn list(body: &str, selected: &str) -> Response {
        Response::ok().with("selected", selected).with_body(body)
    }

    #[test]
    fn the_service_list_is_authoritative() {
        let mut lib = Library::default();
        assert_eq!(lib.entries(None), vec![]);
        assert!(!lib.loaded);
        let want = lib.apply_list(&list(
            "c0001\tHome VPN\nc0002\tBüro\u{1b}[2J\nbad\n",
            "c0002",
        ));
        assert_eq!(want, vec![id(1), id(2)]);
        assert!(
            !lib.saved[1].name.contains('\u{1b}'),
            "service text is plain"
        );
        assert_eq!(lib.selected(), Some(Key::Saved(id(2))));
        lib.apply_get(
            id(1),
            &Response::ok().with_body(
                "version = 1\n[server]\nendpoints = [\"203.0.113.7:51820\"]\n[routing]\nmode = \"full-tunnel\"\n",
            ),
        );
        let e = lib.entries(Some(id(1)));
        assert_eq!(e[0].detail, "203.0.113.7:51820");
        assert!(e[0].in_use && !e[0].selected && e[1].selected);
        // Known settings survive a refresh; removed connections go.
        assert_eq!(lib.apply_list(&list("c0001\tHome VPN\n", "none")), vec![]);
        assert_eq!(lib.selected(), None);
    }

    #[test]
    fn drafts_follow_the_saved_connections_and_can_be_selected() {
        let mut drafts = Drafts::in_memory();
        let d = drafts
            .save(
                None,
                &Form {
                    name: "Lab".into(),
                    ..Form::default()
                },
            )
            .unwrap();
        let mut lib = Library::new(drafts, Some(d));
        lib.apply_list(&list("c0001\tHome\n", "c0001"));
        assert_eq!(lib.selected(), Some(Key::Draft(d)));
        let e = lib.entries(None);
        assert_eq!(e.len(), 2);
        assert_eq!(e[1].detail, "Draft · Setup Required");
        assert!(lib.name_taken("LAB", None));
        assert!(lib.name_taken(" home ", None));
        assert!(!lib.name_taken("home", Some(Key::Saved(id(1)))));
        assert!(
            Library::new(Drafts::in_memory(), Some(9))
                .draft_selected
                .is_none()
        );
    }

    #[test]
    fn export_names_are_safe() {
        assert_eq!(export_file_name("Home VPN"), "Home VPN.toml");
        assert_eq!(export_file_name("../a/b:c\n"), "_a_b_c.toml");
        assert_eq!(export_file_name("..."), "Septuvo connection.toml");
    }
}
