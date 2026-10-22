//! The window's X against the real lab service (N4-01 acceptance).
//!
//! Ignored by default: `tests/lab/namespaces/desktop_close.sh` runs it on a
//! disposable lab machine or CI runner, after starting `septuvod` in a lab
//! namespace and asking it to connect (kill switch blocking). It runs the
//! app's real service link and close logic; only the X click itself is
//! simulated (the close request a window system sends).
//!
//! - `SEPTUVO_LAB_RUNTIME_DIR`: the service's runtime directory.
//! - `SEPTUVO_LAB_CASE`: `member` (in the control group: X disconnects, then
//!   the app exits) or `other` (not in it: disconnect is refused, the window
//!   stays with the error; a second X exits and leaves the service as it was).

use eframe::egui::{ViewportEvent, ViewportId};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use septuvo_cli::watch::{Snapshot, snapshot};
use septuvo_desktop::app::SeptuvoApp;
use septuvo_desktop::prefs::Prefs;
use septuvo_desktop::service::Worker;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
}

/// Steps the app in real time until `done` or 10 s.
fn wait(
    h: &mut Harness<'static, SeptuvoApp>,
    done: impl Fn(&Harness<'static, SeptuvoApp>) -> bool,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        h.step();
        if done(h) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn service_state(dir: &Path) -> String {
    match snapshot(dir) {
        Snapshot::Service(r) => r.field("state").unwrap_or("?").to_string(),
        Snapshot::Lost(why) => format!("lost: {why}"),
    }
}

#[test]
#[ignore = "lab only: run by tests/lab/namespaces/desktop_close.sh"]
fn the_x_against_the_lab_service() {
    let dir =
        PathBuf::from(std::env::var("SEPTUVO_LAB_RUNTIME_DIR").expect("SEPTUVO_LAB_RUNTIME_DIR"));
    let case = std::env::var("SEPTUVO_LAB_CASE").expect("SEPTUVO_LAB_CASE");
    assert_eq!(
        service_state(&dir),
        "blocking",
        "the lab script connects first"
    );

    let link = Worker::start(dir.clone(), || {});
    let mut h = Harness::builder().with_size([780.0, 760.0]).build_ui_state(
        |ui, app: &mut SeptuvoApp| app.draw(ui),
        SeptuvoApp::new(Box::new(link), Prefs::default(), None),
    );
    h.run_steps(3);
    h.get_by_label("Traffic Protected by Kill Switch");
    println!("LAB the window shows the live blocking state");

    press_x(&mut h);
    match case.as_str() {
        "member" => {
            assert!(
                wait(&mut h, |h| h.state().exit_requested()),
                "the app did not exit after disconnecting"
            );
            assert_eq!(
                service_state(&dir),
                "disconnected",
                "X must disconnect before exiting"
            );
            println!("LAB member: X disconnected the service, then the app exited");
        }
        "other" => {
            assert!(
                wait(&mut h, |h| h
                    .query_by_label_contains("Could not disconnect safely.")
                    .is_some()),
                "no error shown"
            );
            assert!(
                !h.state().exit_requested(),
                "the app exited although disconnect failed"
            );
            assert_eq!(service_state(&dir), "blocking");
            println!("LAB other: disconnect refused; the window stayed open with the error");
            press_x(&mut h);
            assert!(h.state().exit_requested(), "a second X exits the app");
            assert_eq!(
                service_state(&dir),
                "blocking",
                "exiting the app never changes the service"
            );
            println!("LAB other: a second X exited the app; the service kept blocking");
        }
        other => panic!("unknown SEPTUVO_LAB_CASE {other}"),
    }
}
