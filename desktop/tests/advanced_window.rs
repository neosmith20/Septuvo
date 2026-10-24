//! Advanced main window and Interface Mode tests, through the accessibility
//! tree (`egui_kittest`, test-only). The service is a fake that records
//! every request, so the tests can prove what the app asked it.

use eframe::egui::Key;
use eframe::egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use septuvo_cli::watch::Snapshot;
use septuvo_desktop::advanced_window::Page;
use septuvo_desktop::app::SeptuvoApp;
use septuvo_desktop::prefs::{InterfaceMode, Prefs};
use septuvo_desktop::service::Fake;
use septuvo_desktop::state::samples;
use std::sync::{Arc, Mutex};

type Log = Arc<Mutex<Vec<&'static str>>>;

/// Requests that only read: status, and the saved connections' list and
/// settings (N4-05). Anything else changes something.
const READ_ONLY: [&str; 3] = ["status", "list", "get"];

fn svc(r: septuvo_ipc::Response) -> Snapshot {
    Snapshot::Service(r)
}

fn harness(
    s: Snapshot,
    mode: InterfaceMode,
    size: [f32; 2],
) -> (Harness<'static, SeptuvoApp>, Log) {
    with_fake(Fake::new(s, svc(samples::disconnected())), mode, size)
}

fn with_fake(
    fake: Fake,
    mode: InterfaceMode,
    size: [f32; 2],
) -> (Harness<'static, SeptuvoApp>, Log) {
    let log = fake.log.clone();
    let prefs = Prefs {
        interface_mode: mode,
        ..Prefs::default()
    };
    let mut h = Harness::builder().with_size(size).build_ui_state(
        |ui, app: &mut SeptuvoApp| app.draw(ui),
        SeptuvoApp::new(Box::new(fake), prefs, None),
    );
    h.ctx.set_embed_viewports(true);
    h.run_steps(3);
    (h, log)
}

fn advanced(s: Snapshot) -> (Harness<'static, SeptuvoApp>, Log) {
    harness(s, InterfaceMode::Advanced, [980.0, 640.0])
}

fn role(h: &Harness<'static, SeptuvoApp>, label: &str) -> Role {
    h.get_by_label(label).accesskit_node().role()
}

fn disabled(h: &Harness<'static, SeptuvoApp>, label: &str) -> bool {
    h.get_by_label(label).accesskit_node().is_disabled()
}

#[test]
fn the_approved_structure_is_announced() {
    for size in [[980.0, 640.0], [760.0, 560.0], [1400.0, 900.0]] {
        let (h, _) = harness(svc(samples::blocking()), InterfaceMode::Advanced, size);
        h.get_by_label("Septuvo VPN");
        for tab in ["Home", "Connections", "Settings", "Diagnostics"] {
            h.get_by_label(tab);
        }
        h.get_by_label("Traffic Protected by Kill Switch");
        h.get_by_label("Disconnect");
        h.get_by_label("Connection: No Connections, Add or import a VPN configuration.");
        h.get_by_label("Your New IP Address: Not Connected");
        h.get_by_label("Kill Switch, On, status");
        h.get_by_label("DNS Protection, Unknown, status");
        h.get_by_label("Local Network Access, Unknown, button");
    }
}

#[test]
fn kill_switch_and_dns_are_status_only_and_lan_needs_a_value_from_the_service() {
    let (h, _) = advanced(svc(samples::blocking()));
    assert_ne!(role(&h, "Kill Switch, On, status"), Role::Button);
    assert_ne!(role(&h, "DNS Protection, Unknown, status"), Role::Button);
    assert_eq!(
        role(&h, "Local Network Access, Unknown, button"),
        Role::Button
    );
    // The service reports no value: nothing to toggle from.
    assert!(disabled(&h, "Local Network Access, Unknown, button"));
}

fn requests(log: &Log, what: &str) -> usize {
    log.lock().unwrap().iter().filter(|r| **r == what).count()
}

/// N4-04: the tile asks the service, and shows only what it then reports,
/// with the one short note after allowing.
#[test]
fn local_network_access_is_changed_by_the_service() {
    let (mut h, log) = advanced(svc(samples::lockdown()));
    let tile = "Local Network Access, Not Allowed, button";
    assert!(!disabled(&h, tile));
    h.get_by_label(tile).click();
    h.run_steps(3);
    assert_eq!(requests(&log, "local-network allowed"), 1);
    h.get_by_label("Local Network Access, Allowed, button");
    h.get_by_label(septuvo_desktop::home::LAN_ALLOWED_NOTE);
    h.get_by_label("Local Network Access, Allowed, button")
        .click();
    h.run_steps(3);
    assert_eq!(requests(&log, "local-network not-allowed"), 1);
    h.get_by_label(tile);
    assert!(
        h.query_by_label(septuvo_desktop::home::LAN_ALLOWED_NOTE)
            .is_none()
    );
    assert_eq!(
        requests(&log, "disconnect"),
        0,
        "the connection is untouched"
    );
}

#[test]
fn a_refused_change_keeps_the_value_and_says_why() {
    let mut fake = Fake::new(svc(samples::lockdown()), svc(samples::disconnected()));
    fake.setting_outcome = Err("Local Network Access was not changed: nft failed".into());
    let (mut h, log) = with_fake(fake, InterfaceMode::Advanced, [980.0, 640.0]);
    let tile = "Local Network Access, Not Allowed, button";
    h.get_by_label(tile).click();
    h.run_steps(3);
    assert_eq!(requests(&log, "local-network allowed"), 1);
    h.get_by_label(tile);
    h.get_by_label("Not changed: Local Network Access was not changed: nft failed");
}

#[test]
fn lockdown_mode_is_internet_blocked_and_the_x_simply_exits() {
    let (mut h, log) = advanced(svc(samples::lockdown()));
    h.get_by_label("Internet Blocked");
    h.get_by_label("Lockdown Mode is on. Internet access is blocked until you connect.");
    h.get_by_label("Kill Switch, On, status");
    assert!(disabled(&h, "Connect"));
    assert!(h.query_by_label("Disconnect").is_none());
    h.input_mut()
        .viewports
        .entry(eframe::egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(eframe::egui::ViewportEvent::Close);
    h.step();
    assert!(h.state().exit_requested());
    assert_eq!(
        requests(&log, "disconnect"),
        0,
        "Lockdown Mode is a setting, not a session: it stays after exit"
    );
}

#[test]
fn every_home_state_is_truthful() {
    let cases = [
        (
            svc(samples::disconnected()),
            "VPN Not Yet Available",
            ["Off", "Off"],
            "Not Connected",
        ),
        (
            svc(samples::captive()),
            "Traffic Protected by Kill Switch",
            ["On", "Unknown"],
            "Not Connected",
        ),
        (
            svc(samples::kill_switch_missing()),
            "Connection Failed",
            ["Failed", "Failed"],
            "Not Connected",
        ),
        (
            Snapshot::Lost("gone".into()),
            "Service Unavailable",
            ["Unknown", "Unknown"],
            "Not Connected",
        ),
    ];
    for (s, headline, [ks, dns], ip) in cases {
        let (h, _) = advanced(s);
        h.get_by_label(headline);
        h.get_by_label(&format!("Kill Switch, {ks}, status"));
        h.get_by_label(&format!("DNS Protection, {dns}, status"));
        h.get_by_label(&format!("Your New IP Address: {ip}"));
        assert!(h.query_by_label("You’re Connected").is_none(), "{headline}");
        assert!(h.query_by_label("Connected").is_none(), "{headline}");
    }
}

#[test]
fn switching_modes_sends_nothing_but_status_and_keeps_the_state() {
    // Kill switch blocking: a live state that switching must not touch.
    let (mut h, log) = harness(
        svc(samples::blocking()),
        InterfaceMode::Simple,
        [780.0, 760.0],
    );
    h.get_by_label("Settings").click();
    h.run_steps(2);
    h.get_by_label("Interface Mode: Advanced").click();
    h.run_steps(3);
    assert_eq!(h.state().mode(), InterfaceMode::Advanced);
    assert_eq!(h.state().page(), Page::Settings, "Settings stays in view");
    h.get_by_label("Interface Mode: Advanced");
    h.get_by_label("Home").click();
    h.run_steps(2);
    h.get_by_label("Traffic Protected by Kill Switch");
    h.get_by_label("Kill Switch, On, status");

    h.get_by_label("Settings").click();
    h.run_steps(2);
    h.get_by_label("Interface Mode: Simple").click();
    h.run_steps(3);
    assert_eq!(h.state().mode(), InterfaceMode::Simple);
    h.get_by_label("Traffic Protected by Kill Switch");
    h.get_by_label("Connection: No Connections");

    let log = log.lock().unwrap();
    assert!(!log.is_empty());
    assert!(
        log.iter().all(|r| READ_ONLY.contains(r)),
        "requests that change something: {log:?}"
    );
}

#[test]
fn tabs_and_the_connection_card_open_their_pages() {
    let (mut h, log) = advanced(svc(samples::blocking()));
    for (tab, page) in [
        ("Connections", Page::Connections),
        ("Settings", Page::Settings),
        ("Diagnostics", Page::Diagnostics),
        ("Home", Page::Home),
    ] {
        h.get_by_label(tab).click();
        h.run_steps(2);
        assert_eq!(h.state().page(), page, "{tab}");
    }
    h.get_by_label("Connection: No Connections, Add or import a VPN configuration.")
        .click();
    h.run_steps(2);
    assert_eq!(h.state().page(), Page::Connections);
    h.get_by_label("Manage Your Saved Connections");
    assert!(log.lock().unwrap().iter().all(|r| READ_ONLY.contains(r)));
}

#[test]
fn the_disconnect_button_works_in_advanced_mode() {
    let (mut h, log) = advanced(svc(samples::blocking()));
    h.get_by_label("Disconnect").click();
    h.run_steps(3);
    h.get_by_label("VPN Not Yet Available");
    h.get_by_label("Kill Switch, Off, status");
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|r| **r == "disconnect")
            .count(),
        1
    );
}

#[test]
fn with_no_saved_connection_connect_is_disabled_and_says_why() {
    let (h, _) = advanced(svc(samples::disconnected()));
    assert!(disabled(&h, "Connect"));
    h.get_by_label("Add or import a VPN configuration to get started.");
}

#[test]
#[allow(clippy::redundant_closure_for_method_calls)]
fn keyboard_focus_follows_the_visual_order() {
    let (mut h, _) = advanced(svc(samples::blocking()));
    let mut order = Vec::new();
    for _ in 0..6 {
        h.key_press(Key::Tab);
        h.run_steps(1);
        // A closure: the method path is not general enough for query_by.
        let focused = h
            .query_by(|n| n.is_focused())
            .map(|n| n.accesskit_node().label().unwrap_or_default());
        order.push(focused.unwrap_or_default());
    }
    assert_eq!(
        order,
        [
            "Home",
            "Connections",
            "Settings",
            "Diagnostics",
            "Disconnect",
            "Connection: No Connections, Add or import a VPN configuration.",
        ]
    );
}

#[test]
fn the_x_behaves_as_in_simple_mode() {
    let (mut h, log) = advanced(svc(samples::blocking()));
    h.input_mut()
        .viewports
        .entry(eframe::egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(eframe::egui::ViewportEvent::Close);
    h.step();
    h.run_steps(3);
    assert!(h.state().exit_requested());
    assert!(
        log.lock().unwrap().contains(&"disconnect"),
        "X disconnects first"
    );
}
