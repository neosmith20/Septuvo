//! Main-window tests through the accessibility tree (`egui_kittest`, test-only).
//!
//! They find what a screen reader would announce, by label, so they test
//! both what is shown and that it is exposed to assistive technology. The
//! service is a fake; no display or GPU is needed.

use eframe::egui::{Key, ViewportEvent, ViewportId};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use septuvo_cli::watch::Snapshot;
use septuvo_desktop::app::SeptuvoApp;
use septuvo_desktop::main_window::Shell;
use septuvo_desktop::prefs::Prefs;
use septuvo_desktop::service::Fake;
use septuvo_desktop::state::samples;

fn svc(r: septuvo_ipc::Response) -> Snapshot {
    Snapshot::Service(r)
}

fn lost() -> Snapshot {
    Snapshot::Lost("no Septuvo service is running".into())
}

fn harness(fake: Fake, width: f32, height: f32) -> Harness<'static, SeptuvoApp> {
    let mut h = Harness::builder()
        .with_size([width, height])
        .build_ui_state(
            |ui, app: &mut SeptuvoApp| app.draw(ui),
            SeptuvoApp::new(Box::new(fake), Prefs::default(), None),
        );
    h.ctx.set_embed_viewports(true);
    h.run_steps(3);
    h
}

fn simple(s: Snapshot) -> Harness<'static, SeptuvoApp> {
    harness(Fake::new(s, svc(samples::disconnected())), 780.0, 760.0)
}

/// Presses the window's X (the close request the window system sends).
fn press_x(h: &mut Harness<'static, SeptuvoApp>) {
    h.input_mut()
        .viewports
        .entry(ViewportId::ROOT)
        .or_default()
        .events
        .push(ViewportEvent::Close);
    h.step();
    if let Some(v) = h.input_mut().viewports.get_mut(&ViewportId::ROOT) {
        v.events.clear();
    }
    h.run_steps(2);
}

fn disabled(h: &Harness<'static, SeptuvoApp>, label: &str) -> bool {
    h.get_by_label(label).accesskit_node().is_disabled()
}

#[test]
fn the_header_selector_and_tiles_are_always_announced() {
    for (w, hgt) in [(780.0, 760.0), (520.0, 580.0), (1400.0, 900.0)] {
        let h = harness(
            Fake::new(svc(samples::disconnected()), svc(samples::disconnected())),
            w,
            hgt,
        );
        h.get_by_label("Septuvo VPN");
        h.get_by_label("Connection: No Connections");
        h.get_by_label("Connections, Manage Saved Connections");
        h.get_by_label("Add / Import, VPN Configuration");
        h.get_by_label("Help & Diagnostics, Get Help or Run Tests");
        h.get_by_label("Settings");
    }
}

#[test]
fn with_no_saved_connection_connect_is_disabled_and_says_why() {
    let h = simple(svc(samples::disconnected()));
    h.get_by_label("VPN Not Yet Available");
    h.get_by_label("Not connected");
    assert!(disabled(&h, "Connect"));
    h.get_by_label("Add or import a VPN configuration to get started.");
    assert!(disabled(&h, "Connection: No Connections"));
    assert!(h.query_by_label("Protected").is_none());
    assert!(h.query_by_label("Connected").is_none());
}

#[test]
fn a_lost_service_is_shown_at_once_and_never_as_protected() {
    let h = simple(lost());
    h.get_by_label("Service Unavailable");
    h.get_by_label("Septuvo cannot communicate with its background service.");
    h.get_by_label("Status unknown");
    assert!(disabled(&h, "Connect"));
    h.get_by_label("The Septuvo service is not running.");
    assert!(h.query_by_label("Protected").is_none());
}

#[test]
fn todays_lab_states_never_announce_protected_or_connected() {
    for (s, heading) in [
        (samples::blocking(), "Traffic Protected by Kill Switch"),
        (samples::captive(), "Traffic Protected by Kill Switch"),
        (samples::kill_switch_missing(), "Connection Failed"),
    ] {
        let h = simple(svc(s));
        h.get_by_label(heading);
        h.get_by_label("Not connected");
        assert!(!disabled(&h, "Disconnect"));
        assert!(h.query_by_label("Protected").is_none());
        assert!(h.query_by_label("Connected").is_none());
    }
}

#[test]
fn the_status_follows_the_service_without_a_restart() {
    let mut h = simple(svc(samples::disconnected()));
    h.get_by_label("VPN Not Yet Available");
    // The service starts blocking (for example `septuvo connect` in the lab).
    h.state_mut().set_link(Box::new(Fake::new(
        svc(samples::blocking()),
        svc(samples::disconnected()),
    )));
    h.run_steps(2);
    h.get_by_label("Traffic Protected by Kill Switch");
    h.state_mut().set_link(Box::new(Fake::new(lost(), lost())));
    h.run_steps(2);
    h.get_by_label("Service Unavailable");
}

#[test]
fn disconnect_asks_the_service_and_shows_the_result() {
    let mut h = simple(svc(samples::blocking()));
    h.get_by_label("Disconnect").click();
    h.run_steps(3);
    h.get_by_label("VPN Not Yet Available");
    assert!(
        !h.state().exit_requested(),
        "the Disconnect button never exits"
    );
}

#[test]
fn a_failed_disconnect_is_reported_and_nothing_is_called_safe() {
    let mut fake = Fake::new(svc(samples::blocking()), svc(samples::disconnected()));
    fake.disconnect_result = Err("Permission denied.".into());
    let mut h = harness(fake, 780.0, 760.0);
    h.get_by_label("Disconnect").click();
    h.run_steps(3);
    h.get_by_label_contains("Could not disconnect safely.");
    h.get_by_label("Traffic Protected by Kill Switch");
}

#[test]
fn tiles_and_settings_open_their_windows_without_touching_the_connection() {
    let mut h = simple(svc(samples::blocking()));
    for (label, shell) in [
        ("Connections, Manage Saved Connections", Shell::Connections),
        ("Add / Import, VPN Configuration", Shell::AddImport),
        (
            "Help & Diagnostics, Get Help or Run Tests",
            Shell::HelpDiagnostics,
        ),
        ("Settings", Shell::Settings),
    ] {
        h.get_by_label(label).click();
        h.run_steps(2);
        assert!(h.state().shells().is_open(shell), "{label}");
    }
    assert!(
        !h.get_all_by_label_contains("not available yet")
            .collect::<Vec<_>>()
            .is_empty()
    );
    h.get_by_label_contains("Version 0.0.0");
    h.get_by_label("Traffic Protected by Kill Switch");
}

#[test]
#[allow(clippy::redundant_closure_for_method_calls)]
fn keyboard_focus_follows_the_approved_order() {
    // DEFAULT-UI §9.1: selector (when enabled), main button (when enabled),
    // the three tiles left to right, then Settings. Disabled controls are
    // skipped by the keyboard and announced as unavailable instead.
    let mut h = simple(svc(samples::blocking()));
    let mut order = Vec::new();
    for _ in 0..5 {
        h.key_press(Key::Tab);
        h.run_steps(1);
        let focused = h
            // A closure: the method path is not general enough for query_by.
            .query_by(|n| n.is_focused())
            .map(|n| n.accesskit_node().label().unwrap_or_default());
        order.push(focused.unwrap_or_default());
    }
    assert_eq!(
        order,
        [
            "Disconnect",
            "Connections, Manage Saved Connections",
            "Add / Import, VPN Configuration",
            "Help & Diagnostics, Get Help or Run Tests",
            "Settings",
        ]
    );
}

#[test]
fn the_x_with_nothing_active_exits() {
    let mut h = simple(svc(samples::disconnected()));
    press_x(&mut h);
    assert!(h.state().exit_requested());
}

#[test]
fn the_x_disconnects_first_and_then_exits() {
    let mut h = simple(svc(samples::blocking()));
    press_x(&mut h);
    h.run_steps(3);
    assert!(h.state().exit_requested());
}

#[test]
fn the_x_stays_open_with_the_error_when_disconnect_fails() {
    let mut fake = Fake::new(svc(samples::blocking()), svc(samples::disconnected()));
    fake.disconnect_result = Err("Permission denied.".into());
    let mut h = harness(fake, 780.0, 760.0);
    press_x(&mut h);
    h.run_steps(3);
    assert!(
        !h.state().exit_requested(),
        "must not exit while traffic may still be blocked or tunnelled"
    );
    h.get_by_label_contains("Could not disconnect safely.");
    // A second X exits the app only; the service keeps its state.
    press_x(&mut h);
    assert!(h.state().exit_requested());
}

#[test]
fn the_x_with_the_service_lost_stays_and_explains() {
    let mut h = simple(lost());
    press_x(&mut h);
    assert!(!h.state().exit_requested());
    h.get_by_label_contains("Could not disconnect safely.");
}

#[test]
fn narrow_windows_keep_every_control() {
    let h = harness(
        Fake::new(svc(samples::blocking()), svc(samples::disconnected())),
        520.0,
        580.0,
    );
    for label in [
        "Disconnect",
        "Connections, Manage Saved Connections",
        "Add / Import, VPN Configuration",
        "Help & Diagnostics, Get Help or Run Tests",
        "Settings",
    ] {
        h.get_by_label(label);
    }
}
