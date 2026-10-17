//! `septuvod --settings FILE`: the privileged Septuvo VPN service (lab only).
//!
//! Exit status: 1 invalid settings or configuration, 2 usage error,
//! 3 refused by a safety guard, 4 could not set up its sockets.
//! It runs until it is stopped. Stopping it never removes the kill switch:
//! only a deliberate `septuvo disconnect` does (owner decision Q2).

use septuvo_daemon::engine::{Engine, Nftables, SysClassNet};
use septuvo_daemon::resolver::ResolvConf;
use septuvo_daemon::{guard, settings, sockets};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

const USAGE: &str = "usage: septuvod --settings FILE\n\n\
The privileged Septuvo VPN service. Development preview, lab only: it runs\n\
as root inside a named test network namespace and turns the kill switch on\n\
and off. It has no tunnel and protects nothing outside the lab.";

type Shared = Arc<Mutex<Engine<Nftables, SysClassNet, ResolvConf>>>;

fn serve(listener: &UnixListener, engine: &Shared, control: bool) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut handle = |command| {
            engine
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .handle(command, control)
        };
        sockets::serve_one(&mut stream, &mut handle);
    }
}

fn fail(code: u8, message: &str) -> ExitCode {
    eprintln!("septuvod: {message}");
    ExitCode::from(code)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = match args.as_slice() {
        [flag, path] if flag == "--settings" => PathBuf::from(path),
        [flag] if flag == "--help" || flag == "-h" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        _ => return fail(2, USAGE),
    };
    let settings = match settings::load(&path) {
        Ok(s) => s,
        Err(e) => return fail(1, &e.to_string()),
    };
    if !guard::running_as_root() {
        return fail(3, "refusing to start: the service must run as root");
    }
    if !guard::in_named_namespace(&settings.lab_namespace) {
        return fail(
            3,
            &format!(
                "refusing to start: not running inside the lab network namespace '{}' \
                 (start it with: ip netns exec {} septuvod ...). Before the G1 review \
                 the service never changes a real machine's network.",
                settings.lab_namespace, settings.lab_namespace
            ),
        );
    }
    let config = match septuvo_config::load_device(&settings.device_config) {
        Ok(c) => c,
        Err(e) => return fail(1, &format!("{}: {e}", settings.device_config.display())),
    };
    let groups = std::fs::read_to_string("/etc/group").unwrap_or_default();
    let Some(gid) = guard::group_id(&groups, &settings.control_group) else {
        return fail(
            1,
            &format!("control group '{}' does not exist", settings.control_group),
        );
    };
    let listeners = match sockets::listen(&settings.runtime_dir, gid) {
        Ok(l) => l,
        Err(e) => return fail(4, &e.to_string()),
    };
    let resolver = ResolvConf::for_lab_namespace(&settings.lab_namespace);
    let engine: Shared = Arc::new(Mutex::new(Engine::new(
        config,
        Nftables,
        SysClassNet,
        resolver,
    )));
    println!(
        "septuvod: ready (lab mode, namespace {}, sockets in {})",
        settings.lab_namespace,
        settings.runtime_dir.display()
    );
    let control_engine = Arc::clone(&engine);
    let control = listeners.control;
    let worker = thread::spawn(move || serve(&control, &control_engine, true));
    serve(&listeners.status, &engine, false);
    let _ = worker.join();
    ExitCode::SUCCESS
}
