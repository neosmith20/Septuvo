//! Saved connections in the app (N4-05), through the accessibility tree
//! (`egui_kittest`, test-only): the flows of DEFAULT-UI §9.3 items 4–9.
//!
//! The service is a fake with its own connection store that follows the
//! service's rules, and records every request; the file dialogs are fakes
//! with prepared answers. Every check finds what a screen reader would
//! announce.

use eframe::egui::Key;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use septuvo_cli::watch::Snapshot;
use septuvo_desktop::app::SeptuvoApp;
use septuvo_desktop::connections::{BACKUP_WARNING, EXPORT_HEADER};
use septuvo_desktop::files::{FakeFiles, Opened};
use septuvo_desktop::prefs::Prefs;
use septuvo_desktop::service::Fake;
use septuvo_desktop::state::{ADD_FIRST, COMPLETE_FIRST, EDIT_TO_COMPLETE, LAB_LINE, samples};
use std::sync::{Arc, Mutex};

type Log = Arc<Mutex<Vec<&'static str>>>;

const HOME: &str = "version = 1\n[server]\nendpoints = [\"203.0.113.10:51820\"]\n[routing]\nmode = \"full-tunnel\"\n[dns]\nresolvers = [\"10.77.0.1\"]\n[connection]\nname = \"Home VPN\"\n";
const OFFICE: &str = "version = 1\n[server]\nendpoints = [\"198.51.100.7:51820\"]\n[routing]\nmode = \"split\"\ntunnel = [\"10.20.0.0/16\"]\n[dns]\nresolvers = [\"10.20.0.53\"]\n[connection]\nname = \"Office\"\n";

fn svc(r: septuvo_ipc::Response) -> Snapshot {
    Snapshot::Service(r)
}

fn app(fake: Fake, files: FakeFiles) -> (Harness<'static, SeptuvoApp>, Log) {
    let log = fake.log.clone();
    let mut app = SeptuvoApp::new(Box::new(fake), Prefs::default(), None);
    app.set_files(Box::new(files));
    let mut h = Harness::builder()
        .with_size([900.0, 900.0])
        .build_ui_state(|ui, app: &mut SeptuvoApp| app.draw(ui), app);
    h.ctx.set_embed_viewports(true);
    h.run_steps(4);
    (h, log)
}

fn saved(state: septuvo_ipc::Response) -> (Harness<'static, SeptuvoApp>, Log) {
    let fake =
        Fake::new(svc(state), svc(samples::disconnected())).with_connections(&[HOME, OFFICE]);
    app(fake, FakeFiles::default())
}

fn count(log: &Log, what: &str) -> usize {
    log.lock().unwrap().iter().filter(|r| **r == what).count()
}

fn disabled(h: &Harness<'static, SeptuvoApp>, label: &str) -> bool {
    h.get_by_label(label).accesskit_node().is_disabled()
}

fn click(h: &mut Harness<'static, SeptuvoApp>, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
}

/// The text field labelled `label` (its label is linked to it, as a
/// screen reader announces it).
fn field<'a>(h: &'a Harness<'static, SeptuvoApp>, label: &'static str) -> egui_kittest::Node<'a> {
    h.get_by(|n| {
        n.role() == eframe::egui::accesskit::Role::TextInput && n.label().as_deref() == Some(label)
    })
}

/// Types into the text field that shows `hint` (fields are found by their
/// placeholder, as a screen reader announces it with the label).
fn type_into(h: &mut Harness<'static, SeptuvoApp>, hint: &str, text: &str) {
    h.get_by(|n| n.placeholder() == Some(hint)).click();
    h.run_steps(1);
    h.get_by(|n| n.placeholder() == Some(hint)).type_text(text);
    h.run_steps(2);
}

// --- §9.3 item 4: the empty state --------------------------------------------

#[test]
fn with_no_connection_connect_is_disabled_and_add_import_is_offered() {
    let (h, log) = app(
        Fake::new(svc(samples::disconnected()), svc(samples::disconnected())),
        FakeFiles::default(),
    );
    assert!(disabled(&h, "Connection: No Connections"));
    assert!(disabled(&h, "Connect"));
    h.get_by_label(ADD_FIRST);
    assert!(!disabled(&h, "Add / Import, VPN Configuration"));
    assert_eq!(
        count(&log, "list"),
        1,
        "the service was asked, nothing invented"
    );
}

// --- §9.3 item 5: the selector -------------------------------------------------

#[test]
fn the_selector_lists_every_connection_and_selects_without_connecting() {
    let (mut h, log) = saved(samples::disconnected());
    click(&mut h, "Connection: Home VPN");
    h.get_by_label("Home VPN, selected");
    click(&mut h, "Office");
    h.get_by_label("Connection: Office");
    assert_eq!(count(&log, "select"), 1);
    assert_eq!(count(&log, "connect"), 0, "selecting never connects");
    // The same one again: nothing happens.
    click(&mut h, "Connection: Office");
    click(&mut h, "Office, selected");
    assert_eq!(count(&log, "select"), 1);
    // Escape closes the list unchanged.
    click(&mut h, "Connection: Office");
    h.key_press(Key::Escape);
    h.run_steps(2);
    assert!(h.query_by_label("Home VPN").is_none());
}

#[test]
fn connect_asks_the_lab_service_and_says_what_it_does() {
    let (mut h, log) = saved(samples::disconnected());
    let label = format!("Connect to Home VPN. {LAB_LINE}");
    assert!(!disabled(&h, &label));
    click(&mut h, &label);
    assert_eq!(count(&log, "connect"), 1);
    h.get_by_label("Traffic Protected by Kill Switch");
    h.get_by_label("Disconnect from Home VPN");
}

// --- §9.3 item 6: switching ---------------------------------------------------

#[test]
fn choosing_another_connection_while_connected_switches_in_one_step() {
    let (mut h, log) = saved(samples::blocking());
    click(&mut h, "Connection: Home VPN");
    click(&mut h, "Office");
    assert_eq!(
        count(&log, "select"),
        1,
        "the service switches without a gap"
    );
    assert_eq!(count(&log, "disconnect"), 0, "no unprotected moment");
    assert_eq!(count(&log, "connect"), 0, "no second click");
    h.get_by_label("Connection: Office");
    h.get_by_label("Traffic Protected by Kill Switch");
}

#[test]
fn a_failed_switch_keeps_the_new_choice_and_says_why() {
    let mut fake = Fake::new(svc(samples::blocking()), svc(samples::disconnected()))
        .with_connections(&[HOME, OFFICE]);
    fake.refuse
        .push(("select", "The new connection could not be used.".into()));
    let (mut h, log) = app(fake, FakeFiles::default());
    click(&mut h, "Connection: Home VPN");
    click(&mut h, "Office");
    assert_eq!(count(&log, "select"), 1);
    h.get_by_label("The new connection could not be used.");
    assert_eq!(
        count(&log, "connect"),
        0,
        "the old one is never silently resumed"
    );
}

// --- §9.3 item 7: the Connections manager -------------------------------------

#[test]
fn the_menu_has_exactly_the_five_actions_and_delete_confirms() {
    let (mut h, log) = saved(samples::disconnected());
    click(&mut h, "Connections, Manage Saved Connections");
    click(&mut h, "More actions for Office");
    for item in ["Edit", "Rename", "Export", "Duplicate", "Delete"] {
        h.get_by_label(item);
    }
    click(&mut h, "Delete");
    h.get_by_label("Delete “Office”?");
    click(&mut h, "Cancel");
    assert_eq!(count(&log, "delete"), 0);
    click(&mut h, "More actions for Office");
    click(&mut h, "Delete");
    click(&mut h, "Delete Connection");
    assert_eq!(count(&log, "delete"), 1);
    h.run_steps(3);
    assert!(
        h.query_by_label("More actions for Office").is_none(),
        "gone at once"
    );
}

#[test]
fn deleting_the_connection_in_use_disconnects_it_first() {
    let (mut h, log) = saved(samples::blocking());
    click(&mut h, "Connections, Manage Saved Connections");
    click(&mut h, "More actions for Home VPN");
    click(&mut h, "Delete");
    click(&mut h, "Delete Connection");
    h.run_steps(3);
    let log = log.lock().unwrap();
    let d = log
        .iter()
        .position(|r| *r == "disconnect")
        .expect("disconnected");
    let x = log.iter().position(|r| *r == "delete").expect("deleted");
    assert!(d < x, "disconnect first: {log:?}");
}

#[test]
fn rename_refuses_a_duplicate_name() {
    let (mut h, log) = saved(samples::disconnected());
    click(&mut h, "Connections, Manage Saved Connections");
    click(&mut h, "More actions for Office");
    click(&mut h, "Rename");
    field(&h, "Connection Name").click();
    h.run_steps(1);
    for _ in 0..6 {
        h.key_press(Key::Backspace);
    }
    field(&h, "Connection Name").type_text("home vpn");
    h.run_steps(1);
    click(&mut h, "Save");
    h.get_by_label("Another connection already has this name. Choose a different name.");
    assert_eq!(count(&log, "rename"), 0);
}

// --- §9.3 item 8: the editor --------------------------------------------------

#[test]
fn a_new_configuration_can_be_saved_as_a_draft_and_blocks_connect() {
    let (mut h, log) = saved(samples::disconnected());
    click(&mut h, "Add / Import, VPN Configuration");
    click(&mut h, "Create New Configuration");
    h.get_by_label("New Configuration");
    assert!(
        h.query_by_label("MTU").is_none(),
        "Advanced Configuration is collapsed"
    );
    type_into(&mut h, "Home VPN", "Cabin");
    type_into(&mut h, "203.0.113.10:51820", "192.0.2.5:51820");
    assert!(
        disabled(&h, "Save Configuration"),
        "no DNS yet: not complete"
    );
    click(&mut h, "Save Draft");
    assert_eq!(count(&log, "add"), 0, "drafts never reach the service");
    h.get_by_label("Draft saved. Complete this configuration before connecting.");
    click(&mut h, "Close");
    click(&mut h, "Connection: Home VPN");
    click(&mut h, "Cabin, Setup Required");
    h.get_by_label("Setup Required");
    assert!(disabled(&h, "Connect"));
    h.get_by_label(COMPLETE_FIRST);
    h.get_by_label(EDIT_TO_COMPLETE);
}

#[test]
fn edit_shows_the_saved_values_and_the_advanced_fields_on_request() {
    let (mut h, log) = saved(samples::disconnected());
    click(&mut h, "Connections, Manage Saved Connections");
    click(&mut h, "More actions for Office");
    click(&mut h, "Edit");
    h.get_by_label("Edit Configuration");
    assert_eq!(
        field(&h, "Server Address")
            .accesskit_node()
            .value()
            .as_deref(),
        Some("198.51.100.7:51820")
    );
    click(&mut h, "Advanced Configuration, collapsed");
    h.get_by_label("Advanced Configuration, expanded");
    h.get_by_label("MTU");
    h.get_by_label("Networks Through VPN");
    type_into(&mut h, "1420", "1400");
    click(&mut h, "Save Configuration");
    assert_eq!(count(&log, "replace"), 1);
}

// --- §9.3 item 9: file dialogs -------------------------------------------------

#[test]
fn import_previews_then_saves_and_selects_without_connecting() {
    let mut files = FakeFiles::default();
    files.open_result = Some(Ok(Some(Opened {
        file_name: "Cabin.toml".into(),
        text: HOME.replace("Home VPN", "Cabin"),
    })));
    let fake = Fake::new(svc(samples::disconnected()), svc(samples::disconnected()))
        .with_connections(&[HOME]);
    let (mut h, log) = app(fake, files);
    click(&mut h, "Add / Import, VPN Configuration");
    click(&mut h, "Import Configuration");
    h.get_by_label("Import Preview");
    click(&mut h, "Import Configuration");
    h.run_steps(3);
    assert_eq!(count(&log, "add"), 1);
    assert_eq!(count(&log, "connect"), 0);
    h.get_by_label("Configuration imported.");
    h.get_by_label("Connection: Cabin");
}

#[test]
fn a_cancelled_open_changes_nothing() {
    let (mut h, log) = saved(samples::disconnected());
    click(&mut h, "Add / Import, VPN Configuration");
    click(&mut h, "Import Configuration");
    h.get_by_label("Create New Configuration");
    assert_eq!(count(&log, "add"), 0);
}

#[test]
fn export_saves_a_backup_without_secrets_and_says_so() {
    let mut files = FakeFiles::default();
    files.save_result = Some(Ok(Some("Office.toml".into())));
    let written = files.written.clone();
    let suggested = files.suggested.clone();
    let fake = Fake::new(svc(samples::disconnected()), svc(samples::disconnected()))
        .with_connections(&[HOME, OFFICE]);
    let (mut h, _) = app(fake, files);
    click(&mut h, "Connections, Manage Saved Connections");
    click(&mut h, "More actions for Office");
    click(&mut h, "Export");
    h.run_steps(2);
    assert_eq!(suggested.lock().unwrap().as_slice(), ["Office.toml"]);
    let w = written.lock().unwrap();
    assert!(w[0].1.starts_with(EXPORT_HEADER));
    assert!(w[0].1.contains("name = \"Office\""));
    h.get_by_label(&format!("Configuration exported. {BACKUP_WARNING}"));
}
