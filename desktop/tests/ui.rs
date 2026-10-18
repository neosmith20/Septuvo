//! UI tests through the accessibility tree (`egui_kittest`, test-only).
//!
//! They find what a screen reader would announce, by label, so they test
//! both what is shown and that it is exposed to assistive technology. No
//! display or GPU is needed.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use septuvo_cli::watch::Snapshot;
use septuvo_desktop::app::{SeptuvoApp, StatusSource};
use septuvo_desktop::view::{PREVIEW_NOTICE, StatusView, samples};

fn harness(source: StatusSource, width: f32) -> Harness<'static, SeptuvoApp> {
    Harness::builder().with_size([width, 900.0]).build_ui_state(
        |ui, app: &mut SeptuvoApp| app.draw(ui),
        SeptuvoApp::new(source),
    )
}

fn sample(s: &Snapshot) -> StatusSource {
    StatusSource::Sample(StatusView::from_snapshot(s))
}

#[test]
fn the_preview_notice_and_product_name_are_always_announced() {
    for width in [1100.0, 390.0] {
        let h = harness(StatusSource::Pending, width);
        h.get_by_label(PREVIEW_NOTICE);
        h.get_by_label("Septuvo VPN");
        h.get_by_label("DEVELOPMENT PREVIEW");
    }
}

#[test]
fn before_live_status_the_app_shows_no_state_at_all() {
    let h = harness(StatusSource::Pending, 1100.0);
    h.get_by_label("Live status is coming in the next update");
    for claim in [
        "Connected",
        "Not connected",
        "Service not running",
        "Passed",
        "Failed",
    ] {
        assert!(
            h.query_by_label(claim).is_none(),
            "{claim} shown without a status"
        );
    }
}

#[test]
fn todays_lab_states_never_announce_connected() {
    for s in [
        samples::disconnected(),
        samples::blocking(),
        samples::captive(),
    ] {
        let h = harness(sample(&Snapshot::Service(s)), 1100.0);
        h.get_by_label("Not connected");
        assert!(h.query_by_label("Connected").is_none());
        h.get_by_label_contains("SAMPLE DATA");
        assert_eq!(h.get_all_by_label("Failed").count(), 2);
        assert_eq!(h.get_all_by_label("Unknown").count(), 2);
        h.get_by_label("Server identity verified");
    }
}

#[test]
fn a_lost_service_is_announced_as_unprotected() {
    let h = harness(
        sample(&Snapshot::Lost("no Septuvo service is running".into())),
        1100.0,
    );
    h.get_by_label("Service not running");
    h.get_by_label("SERVICE LOST");
    h.get_by_label("Reason: no Septuvo service is running");
    assert_eq!(h.get_all_by_label("Unknown").count(), 4);
    assert!(
        h.query_by_label("Protection").is_none(),
        "no last known details"
    );
}

#[test]
fn the_captive_window_warning_is_announced() {
    let h = harness(sample(&Snapshot::Service(samples::captive())), 390.0);
    h.get_by_label("Captive-portal window open");
    h.get_by_label("4:32 left");
    h.get_by_label_contains("OUTSIDE the tunnel");
}

#[test]
fn navigation_works_from_the_accessibility_tree_on_wide_and_narrow_windows() {
    for width in [1100.0, 390.0] {
        let mut h = harness(StatusSource::Pending, width);
        h.get_by_label("About").click();
        h.run();
        h.get_by_label("Version 0.0.0");
        h.get_by_label("Security Without Compromise.");
        assert_eq!(
            h.get_all_by_label("Septuvo VPN logo").count(),
            2,
            "header and About logos have alt text"
        );
        h.get_by_label("Status").click();
        h.run();
        h.get_by_label("Live status is coming in the next update");
    }
}
