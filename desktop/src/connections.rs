//! What the app does with saved connections (N4-05): selecting and
//! switching, connecting, the Connections manager's five actions, Add /
//! Import and the configuration editor (DEFAULT-UI §3.1–§3.3, §4, §7).
//!
//! Plain Rust, no drawing: the windows call these methods, and the app
//! carries out the [`Effect`]s (service requests, file dialogs). Every
//! change goes to the **service**, which validates again and answers;
//! nothing is shown as done before it answers. Drafts are the app's own.

use std::time::{Duration, Instant};

use septuvo_cli::watch::Snapshot;
use septuvo_config::{Problem, connection_name, name_table, parse_device, split_connection};
use septuvo_ipc::ConnectionId;

use crate::files::{FileEvent, NOT_SUPPORTED, Opened};
use crate::form::{Checked, Field, Form};
use crate::library::{Key, Library, export_file_name};
use crate::service::{Op, Reply};

/// How often the list is asked for again (changes made elsewhere, for
/// example with the command line, appear within this time).
pub const LIST_REFRESH: Duration = Duration::from_secs(10);

/// The first lines of an exported file (DEFAULT-UI §4.4).
pub const EXPORT_HEADER: &str = "# Septuvo VPN connection backup: a non-secret configuration.\n\
# This backup does not include private credentials. You may need to set up access again.\n";

/// The note shown after an export (DEFAULT-UI §4.4).
pub const BACKUP_WARNING: &str =
    "This backup does not include private credentials. You may need to set up access again.";

pub use crate::state::COMPLETE_FIRST;

/// Something the app must do for the controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Send a request to the service.
    Op(Op),
    /// A deliberate disconnect, before deleting the connection in use.
    Disconnect,
    /// Show the native Open dialog.
    OpenFile,
    /// Show the native Save As dialog and write the file.
    SaveFile {
        /// Suggested file name.
        file_name: String,
        /// What to write.
        contents: String,
    },
    /// Remember the selected draft in the app preferences.
    RememberDraft(Option<u32>),
}

/// A short message in a secondary window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// The text (plain).
    pub text: String,
    /// An error (red) or a success (plain).
    pub error: bool,
}

/// The **Rename** dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDialog {
    /// Which connection.
    pub key: Key,
    /// The new name, as typed.
    pub text: String,
    /// Why it cannot be used.
    pub error: Option<String>,
    /// Waiting for the service.
    pub busy: bool,
}

/// The **Delete** confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteDialog {
    /// Which connection.
    pub key: Key,
    /// Its name, for the question.
    pub name: String,
    /// Why it failed.
    pub error: Option<String>,
    /// Waiting for the service.
    pub busy: bool,
}

/// The import preview (DEFAULT-UI §4.3 steps 5–7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// The chosen file's name.
    pub file_name: String,
    /// The file's text.
    pub text: String,
    /// What it contains, for the preview.
    pub form: Form,
    /// Why it is not complete (it would be a draft), if so.
    pub incomplete: Option<String>,
    /// The connection name, editable.
    pub name: String,
    /// Why the name or the import failed.
    pub error: Option<String>,
    /// Waiting for the service.
    pub busy: bool,
}

/// The Add / Import window's step.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ImportStep {
    /// The two choices.
    #[default]
    Choice,
    /// The native Open dialog is showing.
    Choosing,
    /// The preview of the chosen file.
    Preview(Box<Preview>),
}

/// Which editor (DEFAULT-UI §4.2 subtitles).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    /// `New Configuration`.
    New,
    /// `Edit Configuration` of a saved connection.
    Edit(ConnectionId),
    /// `Edit Configuration` of a draft.
    EditDraft(u32),
    /// `Duplicate Configuration`.
    Duplicate,
}

impl EditorMode {
    /// The subtitle.
    #[must_use]
    pub fn subtitle(self) -> &'static str {
        match self {
            EditorMode::New => "New Configuration",
            EditorMode::Edit(_) | EditorMode::EditDraft(_) => "Edit Configuration",
            EditorMode::Duplicate => "Duplicate Configuration",
        }
    }
}

/// The open configuration editor.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // independent flags of one dialog
pub struct Editor {
    /// Which editor.
    pub mode: EditorMode,
    /// What is typed.
    pub form: Form,
    /// What it started with (to tell unsaved changes).
    pub original: Form,
    /// Advanced Configuration is expanded (collapsed at first).
    pub advanced: bool,
    /// Why saving failed.
    pub error: Option<String>,
    /// Waiting for the service.
    pub busy: bool,
    /// Asking whether to discard unsaved changes.
    pub confirm_discard: bool,
    /// Waiting for the saved configuration to arrive (Edit).
    pub loading: bool,
    /// Saving was tried (errors are shown from then on).
    pub attempted: bool,
}

impl Editor {
    fn new(mode: EditorMode, form: Form) -> Editor {
        Editor {
            mode,
            original: form.clone(),
            form,
            advanced: false,
            error: None,
            busy: false,
            confirm_discard: false,
            loading: false,
            attempted: false,
        }
    }

    /// True if something was changed and not saved.
    #[must_use]
    pub fn changed(&self) -> bool {
        self.form != self.original
    }
}

/// What the service status says about the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Session {
    /// A connection is wanted (the kill switch is on for it).
    pub connected: bool,
    /// The saved connection the service is using then.
    pub in_use: Option<ConnectionId>,
}

impl Session {
    /// From a status answer.
    #[must_use]
    pub fn of(snapshot: &Snapshot) -> Session {
        let Snapshot::Service(r) = snapshot else {
            return Session::default();
        };
        let state = r.field("state").unwrap_or("");
        let connected = matches!(
            state,
            "blocking" | "failed" | "connected" | "error-kill-switch-missing"
        );
        Session {
            connected,
            in_use: r
                .field("connection")
                .and_then(ConnectionId::parse)
                .filter(|_| connected),
        }
    }
}

enum Waiting {
    Edit(ConnectionId),
    Export(ConnectionId),
    Duplicate(ConnectionId),
}

/// The saved-connections controller.
#[derive(Default)]
pub struct Connections {
    /// Connections and drafts.
    pub lib: Library,
    /// The overflow menu that is open.
    pub menu: Option<Key>,
    /// The row highlighted for inspection (one click).
    pub highlight: Option<Key>,
    /// The Rename dialog.
    pub rename: Option<RenameDialog>,
    /// The Delete confirmation.
    pub delete: Option<DeleteDialog>,
    /// The Add / Import window's step.
    pub import: ImportStep,
    /// The configuration editor.
    pub editor: Option<Editor>,
    /// A message for the Connections and Add / Import windows.
    pub notice: Option<Notice>,
    /// A message for the main window (a failed switch or connect).
    pub main_notice: Option<String>,
    /// Switching: old and new names (`Switching Connection…`).
    pub switching: Option<(String, String)>,
    /// Connecting: the name (`Connecting…`).
    pub connecting: Option<String>,
    /// The profile selector's list is open.
    pub selector_open: bool,
    waiting: Vec<Waiting>,
    delete_after_disconnect: Option<ConnectionId>,
    added_from: Option<AddedFrom>,
    effects: Vec<Effect>,
    last_list: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddedFrom {
    Import,
    Editor(Option<u32>),
}

fn ok_name(text: &str) -> Result<String, String> {
    connection_name(text).map_err(|why| format!("Enter a connection name: {why}."))
}

const NAME_TAKEN: &str = "Another connection already has this name. Choose a different name.";

impl Connections {
    /// A controller with this library; asks the service for the list.
    #[must_use]
    pub fn new(lib: Library) -> Connections {
        let mut c = Connections {
            lib,
            ..Connections::default()
        };
        c.refresh();
        c
    }

    /// The effects to carry out, in order.
    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }

    /// Asks for the list again.
    pub fn refresh(&mut self) {
        self.last_list = Some(Instant::now());
        self.effects.push(Effect::Op(Op::List));
    }

    /// Asks for the list again when it is due.
    pub fn tick(&mut self) {
        if self.last_list.is_none_or(|t| t.elapsed() >= LIST_REFRESH) {
            self.refresh();
        }
    }

    fn get(&mut self, id: ConnectionId) {
        self.effects.push(Effect::Op(Op::Get(id)));
    }

    // --- selection, switching and connecting (DEFAULT-UI §3.1, §7.2) -----

    /// The user chose a connection in the profile selector.
    pub fn select(&mut self, key: Key, session: Session) {
        self.selector_open = false;
        if self.lib.selected() == Some(key) {
            return; // the same one: nothing happens (§3.1 item 6)
        }
        match key {
            Key::Draft(d) => {
                if session.connected {
                    self.main_notice = Some(format!(
                        "{COMPLETE_FIRST} The current connection was not changed."
                    ));
                    return;
                }
                self.lib.draft_selected = Some(d);
                self.effects.push(Effect::RememberDraft(Some(d)));
            }
            Key::Saved(id) => {
                if self.lib.draft_selected.take().is_some() {
                    self.effects.push(Effect::RememberDraft(None));
                }
                if self.lib.service_selected == Some(id) {
                    return;
                }
                if session.connected {
                    let old = session
                        .in_use
                        .and_then(|i| self.lib.name(Key::Saved(i)))
                        .unwrap_or_else(|| "the current connection".into());
                    let new = self.lib.name(key).unwrap_or_default();
                    self.switching = Some((old, new));
                }
                self.main_notice = None;
                self.effects.push(Effect::Op(Op::Select(id)));
            }
        }
    }

    /// The user pressed Connect.
    pub fn connect(&mut self) {
        if let Some(Key::Saved(id)) = self.lib.selected() {
            self.connecting = self.lib.name(Key::Saved(id));
            self.main_notice = None;
            self.effects.push(Effect::Op(Op::Connect));
        }
    }

    // --- service answers ---------------------------------------------------

    /// Takes a service answer.
    pub fn on_reply(&mut self, reply: Reply, session: Session) {
        let Reply { op, result } = reply;
        match op {
            Op::List => match result {
                Ok(r) => {
                    for id in self.lib.apply_list(&r) {
                        self.get(id);
                    }
                }
                Err(why) => self.lib.error = Some(why),
            },
            Op::Get(id) => self.on_get(id, result.as_ref().ok()),
            Op::Select(_) => {
                self.switching = None;
                match result {
                    Ok(_) => self.refresh(),
                    Err(why) => self.main_notice = Some(why),
                }
            }
            Op::Connect => {
                self.connecting = None;
                if let Err(why) = result {
                    self.main_notice = Some(why);
                }
            }
            Op::Add(_) => self.on_added(result, session),
            Op::Replace(id, _) => match result {
                Ok(_) => {
                    self.lib.forget(id);
                    self.editor = None;
                    self.notice = Some(Notice {
                        text: "Configuration saved.".into(),
                        error: false,
                    });
                    self.refresh();
                }
                Err(why) => self.editor_failed(&why),
            },
            Op::Rename(..) => match result {
                Ok(_) => {
                    self.rename = None;
                    self.refresh();
                }
                Err(why) => {
                    if let Some(r) = &mut self.rename {
                        r.busy = false;
                        r.error = Some(why);
                    }
                }
            },
            Op::Delete(_) => match result {
                Ok(_) => {
                    self.delete = None;
                    self.refresh();
                }
                Err(why) => {
                    if let Some(d) = &mut self.delete {
                        d.busy = false;
                        d.error = Some(why);
                    }
                }
            },
        }
    }

    fn on_get(&mut self, id: ConnectionId, r: Option<&septuvo_ipc::Response>) {
        if let Some(r) = r {
            self.lib.apply_get(id, r);
        }
        let text = self.lib.saved(id).and_then(|s| s.text.clone());
        let name = self.lib.name(Key::Saved(id)).unwrap_or_default();
        let waiting = std::mem::take(&mut self.waiting);
        for w in waiting {
            match (w, &text) {
                (Waiting::Edit(i), Some(t)) if i == id => {
                    self.editor = Some(Editor::new(EditorMode::Edit(id), with_name(t, &name)));
                }
                (Waiting::Duplicate(i), Some(t)) if i == id => {
                    self.open_duplicate(&with_name(t, &name));
                }
                (Waiting::Export(i), Some(t)) if i == id => self.export_text(&name, t),
                (Waiting::Edit(i) | Waiting::Duplicate(i) | Waiting::Export(i), None)
                    if i == id =>
                {
                    self.editor = None;
                    self.notice = Some(Notice {
                        text: "This connection's settings could not be read. You may need to be \
                               in the septuvo group."
                            .into(),
                        error: true,
                    });
                }
                (w, _) => self.waiting.push(w),
            }
        }
    }

    fn on_added(&mut self, result: Result<septuvo_ipc::Response, String>, session: Session) {
        let from = self.added_from.take();
        match result {
            Ok(r) => {
                let id = r.field("id").and_then(ConnectionId::parse);
                // Select the new connection, never connect it (§4.3 step 6);
                // while connected selecting would switch, so it is not.
                if let (Some(id), false) = (id, session.connected) {
                    if self.lib.draft_selected.take().is_some() {
                        self.effects.push(Effect::RememberDraft(None));
                    }
                    self.effects.push(Effect::Op(Op::Select(id)));
                }
                let text = if from == Some(AddedFrom::Import) {
                    self.import = ImportStep::Choice;
                    "Configuration imported."
                } else {
                    // A draft that became a saved connection is removed.
                    if let Some(AddedFrom::Editor(Some(d))) = from {
                        let _ = self.lib.drafts.delete(d);
                        if self.lib.draft_selected == Some(d) {
                            self.lib.draft_selected = None;
                            self.effects.push(Effect::RememberDraft(None));
                        }
                    }
                    self.editor = None;
                    "Configuration saved."
                };
                self.notice = Some(Notice {
                    text: text.into(),
                    error: false,
                });
                self.refresh();
            }
            Err(why) => match from {
                Some(AddedFrom::Import) => {
                    if let ImportStep::Preview(p) = &mut self.import {
                        p.busy = false;
                        p.error = Some(why);
                    }
                }
                _ => self.editor_failed(&why),
            },
        }
    }

    fn editor_failed(&mut self, why: &str) {
        if let Some(e) = &mut self.editor {
            e.busy = false;
            e.error = Some(format!("Not saved: {why}"));
        }
    }

    /// The result of the deliberate disconnect made before deleting the
    /// connection in use. Returns true if it was for that.
    pub fn on_disconnect_result(&mut self, result: &Result<(), String>) -> bool {
        let Some(id) = self.delete_after_disconnect.take() else {
            return false;
        };
        match result {
            Ok(()) => self.effects.push(Effect::Op(Op::Delete(id))),
            Err(why) => {
                if let Some(d) = &mut self.delete {
                    d.busy = false;
                    d.error = Some(format!(
                        "Could not disconnect safely: {why} The connection was not deleted."
                    ));
                }
            }
        }
        true
    }

    // --- the Connections manager's menu (DEFAULT-UI §3.2) -------------------

    /// **Edit**.
    pub fn edit(&mut self, key: Key) {
        self.menu = None;
        match key {
            Key::Draft(d) => {
                if let Some(draft) = self.lib.drafts.get(d) {
                    self.editor = Some(Editor::new(EditorMode::EditDraft(d), draft.form.clone()));
                }
            }
            Key::Saved(id) => {
                let name = self.lib.name(key).unwrap_or_default();
                if let Some(t) = self.lib.saved(id).and_then(|s| s.text.clone()) {
                    self.editor = Some(Editor::new(EditorMode::Edit(id), with_name(&t, &name)));
                } else {
                    let mut e = Editor::new(EditorMode::Edit(id), Form::default());
                    e.loading = true;
                    self.editor = Some(e);
                    self.waiting.push(Waiting::Edit(id));
                    self.get(id);
                }
            }
        }
    }

    /// **Rename**: opens the dialog.
    pub fn rename(&mut self, key: Key) {
        self.menu = None;
        let text = self.lib.name(key).unwrap_or_default();
        self.rename = Some(RenameDialog {
            key,
            text,
            error: None,
            busy: false,
        });
    }

    /// Rename: **Save**.
    pub fn rename_submit(&mut self) {
        let Some(r) = &mut self.rename else { return };
        let name = match ok_name(&r.text) {
            Ok(n) => n,
            Err(e) => {
                r.error = Some(e);
                return;
            }
        };
        let key = r.key;
        if self.lib.name_taken(&name, Some(key)) {
            if let Some(r) = &mut self.rename {
                r.error = Some(NAME_TAKEN.into());
            }
            return;
        }
        match key {
            Key::Draft(d) => {
                let Some(mut form) = self.lib.drafts.get(d).map(|x| x.form.clone()) else {
                    self.rename = None;
                    return;
                };
                form.name = name;
                match self.lib.drafts.save(Some(d), &form) {
                    Ok(_) => self.rename = None,
                    Err(e) => {
                        if let Some(r) = &mut self.rename {
                            r.error = Some(e);
                        }
                    }
                }
            }
            Key::Saved(id) => {
                if let Some(r) = &mut self.rename {
                    r.busy = true;
                    r.error = None;
                }
                self.effects.push(Effect::Op(Op::Rename(id, name)));
            }
        }
    }

    /// **Export** (saved connections only: a draft is not a configuration
    /// yet).
    pub fn export(&mut self, key: Key) {
        self.menu = None;
        let Key::Saved(id) = key else {
            self.notice = Some(Notice {
                text: "A draft cannot be exported. Complete it first.".into(),
                error: true,
            });
            return;
        };
        let name = self.lib.name(key).unwrap_or_default();
        if let Some(t) = self.lib.saved(id).and_then(|s| s.text.clone()) {
            self.export_text(&name, &t);
        } else {
            self.waiting.push(Waiting::Export(id));
            self.get(id);
        }
    }

    fn export_text(&mut self, name: &str, text: &str) {
        self.effects.push(Effect::SaveFile {
            file_name: export_file_name(name),
            contents: format!("{EXPORT_HEADER}{text}"),
        });
    }

    /// **Duplicate**.
    pub fn duplicate(&mut self, key: Key) {
        self.menu = None;
        match key {
            Key::Draft(d) => {
                if let Some(f) = self.lib.drafts.get(d).map(|x| x.form.clone()) {
                    self.open_duplicate(&f);
                }
            }
            Key::Saved(id) => {
                let name = self.lib.name(key).unwrap_or_default();
                if let Some(t) = self.lib.saved(id).and_then(|s| s.text.clone()) {
                    self.open_duplicate(&with_name(&t, &name));
                } else {
                    self.waiting.push(Waiting::Duplicate(id));
                    self.get(id);
                }
            }
        }
    }

    fn open_duplicate(&mut self, form: &Form) {
        let lib = &self.lib;
        let copy = form.duplicate(|n| lib.name_taken(n, None));
        let mut e = Editor::new(EditorMode::Duplicate, copy);
        // A duplicate is new: saving it is a change even if untouched.
        e.original = Form::default();
        self.editor = Some(e);
    }

    /// **Delete**: opens the confirmation.
    pub fn delete(&mut self, key: Key) {
        self.menu = None;
        let name = self.lib.name(key).unwrap_or_default();
        self.delete = Some(DeleteDialog {
            key,
            name,
            error: None,
            busy: false,
        });
    }

    /// Delete: **Delete Connection**. The connection in use is first
    /// disconnected; if that fails nothing is deleted (§3.2).
    pub fn delete_confirm(&mut self, session: Session) {
        let Some(d) = &mut self.delete else { return };
        match d.key {
            Key::Draft(id) => match self.lib.drafts.delete(id) {
                Ok(()) => {
                    if self.lib.draft_selected == Some(id) {
                        self.lib.draft_selected = None;
                        self.effects.push(Effect::RememberDraft(None));
                    }
                    self.delete = None;
                }
                Err(e) => d.error = Some(e),
            },
            Key::Saved(id) => {
                d.busy = true;
                d.error = None;
                if session.connected && session.in_use == Some(id) {
                    self.delete_after_disconnect = Some(id);
                    self.effects.push(Effect::Disconnect);
                } else {
                    self.effects.push(Effect::Op(Op::Delete(id)));
                }
            }
        }
    }

    // --- Add / Import (DEFAULT-UI §3.3, §4.3) -------------------------------

    /// **Create New Configuration**.
    pub fn create_new(&mut self) {
        self.editor = Some(Editor::new(EditorMode::New, Form::default()));
    }

    /// **Import Configuration**: the native Open dialog.
    pub fn import(&mut self) {
        self.notice = None;
        self.import = ImportStep::Choosing;
        self.effects.push(Effect::OpenFile);
    }

    /// A file dialog's result.
    pub fn on_file(&mut self, event: FileEvent) {
        match event {
            FileEvent::Opened(Ok(Some(o))) => match preview(&o, &self.lib) {
                Ok(p) => self.import = ImportStep::Preview(Box::new(p)),
                Err(why) => {
                    self.import = ImportStep::Choice;
                    self.notice = Some(Notice {
                        text: why,
                        error: true,
                    });
                }
            },
            FileEvent::Opened(Ok(None)) => self.import = ImportStep::Choice,
            FileEvent::Opened(Err(why)) | FileEvent::Saved(Err(why)) => {
                if self.import == ImportStep::Choosing {
                    self.import = ImportStep::Choice;
                }
                self.notice = Some(Notice {
                    text: why,
                    error: true,
                });
            }
            FileEvent::Saved(Ok(Some(_))) => {
                self.notice = Some(Notice {
                    text: format!("Configuration exported. {BACKUP_WARNING}"),
                    error: false,
                });
            }
            FileEvent::Saved(Ok(None)) => {}
        }
    }

    /// Import preview: **Import Configuration**.
    pub fn import_confirm(&mut self) {
        let ImportStep::Preview(p) = &mut self.import else {
            return;
        };
        let name = match ok_name(&p.name) {
            Ok(n) => n,
            Err(e) => {
                p.error = Some(e);
                return;
            }
        };
        if self.lib.name_taken(&name, None) {
            if let ImportStep::Preview(p) = &mut self.import {
                p.error = Some(NAME_TAKEN.into());
            }
            return;
        }
        let ImportStep::Preview(p) = &mut self.import else {
            return;
        };
        if p.incomplete.is_some() {
            // Not complete: kept as an app draft, labelled Setup Required.
            let form = Form {
                name: name.clone(),
                ..p.form.clone()
            };
            match self.lib.drafts.save(None, &form) {
                Ok(_) => {
                    self.import = ImportStep::Choice;
                    self.notice = Some(Notice {
                        text: format!(
                            "Configuration imported as a draft: “{name}”. Setup Required."
                        ),
                        error: false,
                    });
                }
                Err(e) => p.error = Some(e),
            }
            return;
        }
        let Ok((_, stored)) = split_connection(&p.text) else {
            p.error = Some(NOT_SUPPORTED.into());
            return;
        };
        p.busy = true;
        p.error = None;
        self.added_from = Some(AddedFrom::Import);
        self.effects.push(Effect::Op(Op::Add(format!(
            "{stored}{}",
            name_table(&name)
        ))));
    }

    // --- the editor (DEFAULT-UI §4.2) ---------------------------------------

    /// Checks the editor's form.
    #[must_use]
    pub fn check(&self) -> Checked {
        let Some(e) = &self.editor else {
            return Checked::default();
        };
        let except = match e.mode {
            EditorMode::Edit(id) => Some(Key::Saved(id)),
            EditorMode::EditDraft(d) => Some(Key::Draft(d)),
            EditorMode::New | EditorMode::Duplicate => None,
        };
        e.form.check(|n| self.lib.name_taken(n, except))
    }

    /// Editor: **Save Configuration** (complete only).
    pub fn save_configuration(&mut self) {
        let checked = self.check();
        let Some(e) = &mut self.editor else { return };
        e.attempted = true;
        if !checked.complete {
            e.error = Some(
                checked
                    .errors
                    .first()
                    .map_or("Complete the configuration first.".into(), |(f, m)| {
                        format!("{}: {m}", f.label())
                    }),
            );
            if checked.errors.iter().any(|(f, _)| f.advanced()) {
                e.advanced = true;
            }
            return;
        }
        let text = e.form.to_text();
        e.busy = true;
        e.error = None;
        match e.mode {
            EditorMode::Edit(id) => self.effects.push(Effect::Op(Op::Replace(id, text))),
            mode => {
                let draft = match mode {
                    EditorMode::EditDraft(d) => Some(d),
                    _ => None,
                };
                self.added_from = Some(AddedFrom::Editor(draft));
                self.effects.push(Effect::Op(Op::Add(text)));
            }
        }
    }

    /// Editor: **Save Draft** (any form with a valid, unique name; not for
    /// a saved connection, which stays complete).
    pub fn save_draft(&mut self) {
        let checked = self.check();
        let Some(e) = &mut self.editor else { return };
        e.attempted = true;
        if matches!(e.mode, EditorMode::Edit(_)) {
            return;
        }
        if !checked.name_ok {
            e.error = checked.error(Field::Name).map(str::to_string);
            return;
        }
        let id = match e.mode {
            EditorMode::EditDraft(d) => Some(d),
            _ => None,
        };
        match self.lib.drafts.save(id, &e.form) {
            Ok(_) => {
                self.editor = None;
                self.notice = Some(Notice {
                    text: format!("Draft saved. {COMPLETE_FIRST}"),
                    error: false,
                });
            }
            Err(why) => e.error = Some(why),
        }
    }

    /// Editor: **Cancel**, Escape or the window's X. Unsaved changes ask
    /// first (DEFAULT-UI §4.1).
    pub fn editor_cancel(&mut self) {
        if let Some(e) = &mut self.editor {
            if e.changed() && !e.busy && !e.loading {
                e.confirm_discard = true;
            } else if !e.busy {
                self.editor = None;
            }
        }
    }

    /// Discard confirmed.
    pub fn editor_discard(&mut self) {
        if self.editor.as_ref().is_some_and(|e| !e.busy) {
            self.editor = None;
        }
    }
}

/// A saved configuration text as a form, with its name.
fn with_name(text: &str, name: &str) -> Form {
    Form {
        name: name.to_string(),
        ..Form::from_text(text)
    }
}

/// The import preview for a chosen file, or why it cannot be imported
/// (DEFAULT-UI §4.3 steps 4, 5, 9). A file that is valid except for
/// missing settings becomes a draft; anything else wrong is refused.
///
/// # Errors
/// Not a supported configuration, with the reason.
pub fn preview(o: &Opened, lib: &Library) -> Result<Preview, String> {
    if septuvo_config::toml::parse(&o.text).is_err() {
        return Err(NOT_SUPPORTED.into());
    }
    let form = Form::from_text(&o.text);
    let incomplete = match parse_device(&o.text) {
        Ok(_) => None,
        Err(e) if e.problem == Problem::Missing => Some(if e.path.starts_with("dns") {
            crate::form::NO_DNS.to_string()
        } else {
            format!("Setting missing: {}.", e.path)
        }),
        Err(e) if e.path == "server.endpoints" => {
            return Err("The configuration contains an invalid server address.".into());
        }
        Err(e) => {
            return Err(format!(
                "{NOT_SUPPORTED} {}",
                crate::state::plain(&e.to_string())
            ));
        }
    };
    let suggested = if form.name.trim().is_empty() {
        o.file_name
            .strip_suffix(".toml")
            .unwrap_or(&o.file_name)
            .trim()
            .to_string()
    } else {
        form.name.trim().to_string()
    };
    let error = (!suggested.is_empty() && lib.name_taken(&suggested, None)).then(|| {
        "A connection with this name already exists. Choose a different name.".to_string()
    });
    Ok(Preview {
        file_name: o.file_name.clone(),
        text: o.text.clone(),
        form,
        incomplete,
        name: suggested.chars().take(64).collect(),
        error,
        busy: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drafts::Drafts;
    use septuvo_ipc::Response;

    const HOME: &str = "version = 1\n[server]\nendpoints = [\"203.0.113.7:51820\"]\n\
        [routing]\nmode = \"full-tunnel\"\n[dns]\nresolvers = [\"10.77.0.1\"]\n";

    fn id(n: u16) -> ConnectionId {
        ConnectionId::new(n).unwrap()
    }

    fn loaded() -> Connections {
        let mut c = Connections::new(Library::new(Drafts::in_memory(), None));
        let _ = c.take_effects();
        let list = Response::ok()
            .with("selected", "c0001")
            .with_body("c0001\tHome\nc0002\tOffice\n");
        c.on_reply(
            Reply {
                op: Op::List,
                result: Ok(list),
            },
            Session::default(),
        );
        assert_eq!(
            c.take_effects(),
            vec![Effect::Op(Op::Get(id(1))), Effect::Op(Op::Get(id(2)))]
        );
        c
    }

    fn connected(n: u16) -> Session {
        Session {
            connected: true,
            in_use: Some(id(n)),
        }
    }

    #[test]
    fn selecting_while_disconnected_only_selects() {
        let mut c = loaded();
        c.select(Key::Saved(id(1)), Session::default());
        assert_eq!(c.take_effects(), vec![], "the same one: nothing");
        c.select(Key::Saved(id(2)), Session::default());
        assert_eq!(c.take_effects(), vec![Effect::Op(Op::Select(id(2)))]);
        assert_eq!(c.switching, None);
    }

    #[test]
    fn selecting_while_connected_switches_and_a_failure_is_reported() {
        let mut c = loaded();
        c.select(Key::Saved(id(2)), connected(1));
        assert_eq!(c.switching, Some(("Home".into(), "Office".into())));
        assert_eq!(c.take_effects(), vec![Effect::Op(Op::Select(id(2)))]);
        c.on_reply(
            Reply {
                op: Op::Select(id(2)),
                result: Err("The switch failed.".into()),
            },
            connected(1),
        );
        assert_eq!(c.switching, None);
        assert_eq!(c.main_notice.as_deref(), Some("The switch failed."));
    }

    #[test]
    fn deleting_the_connection_in_use_disconnects_first() {
        let mut c = loaded();
        c.delete(Key::Saved(id(1)));
        c.delete_confirm(connected(1));
        assert_eq!(c.take_effects(), vec![Effect::Disconnect]);
        assert!(c.on_disconnect_result(&Err("refused".into())));
        assert!(c.delete.as_ref().unwrap().error.is_some());
        assert_eq!(
            c.take_effects(),
            vec![],
            "nothing deleted after a failed disconnect"
        );
        c.delete_confirm(connected(1));
        let _ = c.take_effects();
        assert!(c.on_disconnect_result(&Ok(())));
        assert_eq!(c.take_effects(), vec![Effect::Op(Op::Delete(id(1)))]);
        // Not in use: deleted directly.
        c.delete(Key::Saved(id(2)));
        c.delete_confirm(connected(1));
        assert_eq!(c.take_effects(), vec![Effect::Op(Op::Delete(id(2)))]);
    }

    #[test]
    fn rename_refuses_bad_and_duplicate_names_before_asking() {
        let mut c = loaded();
        c.rename(Key::Saved(id(2)));
        c.rename.as_mut().unwrap().text = " HOME ".into();
        c.rename_submit();
        assert_eq!(
            c.rename.as_ref().unwrap().error.as_deref(),
            Some(NAME_TAKEN)
        );
        c.rename.as_mut().unwrap().text = String::new();
        c.rename_submit();
        assert!(c.rename.as_ref().unwrap().error.is_some());
        c.rename.as_mut().unwrap().text = "Work".into();
        c.rename_submit();
        assert_eq!(
            c.take_effects(),
            vec![Effect::Op(Op::Rename(id(2), "Work".into()))]
        );
    }

    #[test]
    fn import_previews_and_never_overwrites() {
        let mut c = loaded();
        c.import();
        assert_eq!(c.take_effects(), vec![Effect::OpenFile]);
        c.on_file(FileEvent::Opened(Ok(Some(Opened {
            file_name: "Home.toml".into(),
            text: HOME.into(),
        }))));
        let ImportStep::Preview(p) = &c.import else {
            panic!("no preview")
        };
        assert!(p.error.is_some(), "the name Home is taken");
        c.import_confirm();
        assert_eq!(c.take_effects(), vec![]);
        if let ImportStep::Preview(p) = &mut c.import {
            p.name = "Cabin".into();
        }
        c.import_confirm();
        let effects = c.take_effects();
        let Some(Effect::Op(Op::Add(text))) = effects.first() else {
            panic!("{effects:?}")
        };
        assert!(text.contains("name = \"Cabin\""));
        c.on_reply(
            Reply {
                op: Op::Add(text.clone()),
                result: Ok(Response::ok().with("id", "c0003")),
            },
            Session::default(),
        );
        assert_eq!(c.import, ImportStep::Choice);
        assert_eq!(c.notice.as_ref().unwrap().text, "Configuration imported.");
        assert!(
            c.take_effects().contains(&Effect::Op(Op::Select(id(3)))),
            "selected, not connected"
        );
    }

    #[test]
    fn imports_that_are_wrong_are_refused_and_incomplete_ones_become_drafts() {
        let lib = Library::default();
        let file = |t: &str| Opened {
            file_name: "x.toml".into(),
            text: t.into(),
        };
        assert_eq!(
            preview(&file("not [toml"), &lib).unwrap_err(),
            NOT_SUPPORTED
        );
        assert_eq!(
            preview(
                &file(&HOME.replace("203.0.113.7:51820", "vpn.example.com:1")),
                &lib
            )
            .unwrap_err(),
            "The configuration contains an invalid server address."
        );
        assert!(
            preview(
                &file(&format!("{HOME}[behavior]\nalways_on = true\n")),
                &lib
            )
            .is_err()
        );
        let p = preview(
            &file(&HOME.replace("[dns]\nresolvers = [\"10.77.0.1\"]\n", "")),
            &lib,
        )
        .unwrap();
        assert!(p.incomplete.is_some());
        assert_eq!(p.name, "x");
        let mut c = Connections::new(Library::default());
        c.import = ImportStep::Preview(Box::new(p));
        c.import_confirm();
        assert_eq!(c.lib.drafts.list().len(), 1, "kept as a draft");
    }

    #[test]
    fn the_editor_saves_complete_configurations_and_drafts() {
        let mut c = loaded();
        c.create_new();
        let e = c.editor.as_mut().unwrap();
        e.form.name = "Lab".into();
        e.form.server = "203.0.113.9:51820".into();
        c.save_configuration();
        assert_eq!(c.take_effects(), vec![], "no DNS: not complete");
        assert!(c.editor.as_ref().unwrap().error.is_some());
        c.save_draft();
        assert!(c.editor.is_none());
        assert_eq!(c.lib.drafts.list().len(), 1);
        // Editing a saved connection replaces it.
        c.lib.apply_get(id(1), &Response::ok().with_body(HOME));
        c.edit(Key::Saved(id(1)));
        let e = c.editor.as_mut().unwrap();
        assert_eq!(e.form.server, "203.0.113.7:51820", "prefilled");
        e.form.mtu = "1400".into();
        c.save_configuration();
        let effects = c.take_effects();
        assert!(matches!(effects.as_slice(), [Effect::Op(Op::Replace(i, _))] if *i == id(1)));
    }

    #[test]
    fn cancelling_with_changes_asks_first() {
        let mut c = loaded();
        c.create_new();
        c.editor_cancel();
        assert!(c.editor.is_none(), "nothing typed: closes");
        c.create_new();
        c.editor.as_mut().unwrap().form.name = "x".into();
        c.editor_cancel();
        assert!(c.editor.as_ref().unwrap().confirm_discard);
        c.editor_discard();
        assert!(c.editor.is_none());
    }

    #[test]
    fn export_writes_the_backup_warning() {
        let mut c = loaded();
        c.lib.apply_get(
            id(1),
            &Response::ok().with_body(&format!("{HOME}{}", name_table("Home"))),
        );
        c.export(Key::Saved(id(1)));
        let effects = c.take_effects();
        let [
            Effect::SaveFile {
                file_name,
                contents,
            },
        ] = effects.as_slice()
        else {
            panic!("{effects:?}")
        };
        assert_eq!(file_name, "Home.toml");
        assert!(contents.starts_with(EXPORT_HEADER));
        assert!(parse_device(contents).is_ok(), "re-importable");
        c.on_file(FileEvent::Saved(Ok(Some("Home.toml".into()))));
        assert!(c.notice.as_ref().unwrap().text.contains(BACKUP_WARNING));
    }

    #[test]
    fn a_draft_cannot_be_selected_while_connected() {
        let mut c = loaded();
        let d = c
            .lib
            .drafts
            .save(
                None,
                &Form {
                    name: "D".into(),
                    ..Form::default()
                },
            )
            .unwrap();
        c.select(Key::Draft(d), connected(1));
        assert_eq!(c.lib.draft_selected, None);
        assert!(c.main_notice.is_some());
        c.select(Key::Draft(d), Session::default());
        assert_eq!(c.lib.selected(), Some(Key::Draft(d)));
        assert_eq!(c.take_effects(), vec![Effect::RememberDraft(Some(d))]);
    }
}
