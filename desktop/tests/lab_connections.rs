//! Saved connections against the real lab service (N4-05 acceptance):
//! import, select, connect, switch and delete, through the app.
//!
//! Ignored by default: `tests/lab/namespaces/desktop_connections.sh` runs it
//! on a disposable lab machine or CI runner, as a member of the control
//! group, with `septuvod` running in a lab namespace and no saved
//! connections. It drives the app's real service link and controller; the
//! native Open dialog is replaced by the files it would return (read with
//! the app's own bounded reader), since a CI runner has no desktop portal.
//!
//! - `SEPTUVO_LAB_RUNTIME_DIR`: the service's runtime directory.
//! - `SEPTUVO_LAB_FILES`: two configuration files, separated by `:`
//!   (connections `Lab A` and `Lab B`, without names in the files).
//!
//! Each step prints a `LAB` line; the script checks the service itself
//! (its list and its rules) between the steps it can see.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use septuvo_cli::watch::{Snapshot, snapshot};
use septuvo_desktop::app::SeptuvoApp;
use septuvo_desktop::files::{FakeFiles, read_bounded};
use septuvo_desktop::prefs::Prefs;
use septuvo_desktop::service::Worker;
use septuvo_ipc::{Command, Outcome, Request};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

type H = Harness<'static, SeptuvoApp>;

/// Steps the app in real time until `label` is announced, or 10 s.
fn wait_for(h: &mut H, label: &str) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        h.step();
        if h.query_by_label(label).is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn click(h: &mut H, label: &str) {
    assert!(wait_for(h, label), "not shown: {label}");
    h.get_by_label(label).click();
    h.run_steps(2);
}

fn service(dir: &Path) -> (String, String) {
    match snapshot(dir) {
        Snapshot::Service(r) => (
            r.field("state").unwrap_or("?").to_string(),
            r.field("connection").unwrap_or("?").to_string(),
        ),
        Snapshot::Lost(why) => (format!("lost: {why}"), String::new()),
    }
}

fn names(dir: &Path) -> String {
    let r = septuvo_cli::service::call(dir, &Request::new(Command::List)).expect("LIST");
    assert_eq!(r.outcome, Outcome::Ok);
    r.body.unwrap_or_default().replace('\n', ";")
}

/// Imports `file` under `name` through Add / Import.
fn import(h: &mut H, file: &Path, name: &str) {
    let mut files = FakeFiles::default();
    files.open_result = Some(Ok(Some(read_bounded(file).expect("lab file"))));
    h.state_mut().set_files(Box::new(files));
    click(h, "Add / Import, VPN Configuration");
    click(h, "Import Configuration");
    assert!(wait_for(h, "Import Preview"));
    let field = h.get_by(|n| {
        n.role() == eframe::egui::accesskit::Role::TextInput
            && n.label().as_deref() == Some("Connection Name")
    });
    field.click();
    h.run_steps(1);
    for _ in 0..64 {
        h.key_press(eframe::egui::Key::Backspace);
    }
    h.get_by(|n| {
        n.role() == eframe::egui::accesskit::Role::TextInput
            && n.label().as_deref() == Some("Connection Name")
    })
    .type_text(name);
    h.run_steps(1);
    click(h, "Import Configuration");
    assert!(
        wait_for(h, "Configuration imported."),
        "{name} not imported"
    );
    click(h, "Close");
}

#[test]
#[ignore = "lab only: run by tests/lab/namespaces/desktop_connections.sh"]
fn import_select_connect_switch_and_delete_against_the_lab_service() {
    let dir =
        PathBuf::from(std::env::var("SEPTUVO_LAB_RUNTIME_DIR").expect("SEPTUVO_LAB_RUNTIME_DIR"));
    let files = std::env::var("SEPTUVO_LAB_FILES").expect("SEPTUVO_LAB_FILES");
    let (a, b) = files.split_once(':').expect("two files");
    assert_eq!(names(&dir), "", "the lab starts with no saved connection");

    let link = Worker::start(dir.clone(), || {});
    let mut h = Harness::builder().with_size([900.0, 900.0]).build_ui_state(
        |ui, app: &mut SeptuvoApp| app.draw(ui),
        SeptuvoApp::new(Box::new(link), Prefs::default(), None),
    );
    h.ctx.set_embed_viewports(true);
    h.run_steps(3);
    assert!(wait_for(&mut h, "Connection: No Connections"));

    import(&mut h, Path::new(a), "Lab A");
    import(&mut h, Path::new(b), "Lab B");
    let list = names(&dir);
    assert!(
        list.contains("\tLab A;") && list.contains("\tLab B;"),
        "{list}"
    );
    println!("LAB imported two connections through the app: {list}");
    assert!(
        wait_for(&mut h, "Connection: Lab B"),
        "the last import is selected"
    );
    assert_eq!(service(&dir).0, "disconnected", "importing never connects");

    click(&mut h, "Connection: Lab B");
    click(&mut h, "Lab A");
    assert!(wait_for(&mut h, "Connection: Lab A"));
    assert_eq!(service(&dir), ("disconnected".into(), "c0001".into()));
    println!("LAB selected Lab A without connecting");

    let connect = format!("Connect to Lab A. {}", septuvo_desktop::state::LAB_LINE);
    click(&mut h, &connect);
    assert!(wait_for(&mut h, "Disconnect from Lab A"));
    assert_eq!(service(&dir), ("blocking".into(), "c0001".into()));
    println!("LAB connected Lab A (kill switch on)");

    click(&mut h, "Connection: Lab A");
    click(&mut h, "Lab B");
    assert!(wait_for(&mut h, "Disconnect from Lab B"));
    assert_eq!(service(&dir), ("blocking".into(), "c0002".into()));
    println!("LAB switched to Lab B while connected, in one step");

    click(&mut h, "Connections, Manage Saved Connections");
    click(&mut h, "More actions for Lab B");
    click(&mut h, "Delete");
    click(&mut h, "Delete Connection");
    let start = Instant::now();
    while names(&dir).contains("Lab B") && start.elapsed() < Duration::from_secs(10) {
        h.step();
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(!names(&dir).contains("Lab B"), "Lab B deleted");
    let (state, _) = service(&dir);
    assert_eq!(
        state, "disconnected",
        "the connection in use was disconnected first"
    );
    println!("LAB deleted Lab B, which was in use: disconnected first, then deleted");
}
