//! `septuvod --settings FILE`: the privileged Septuvo VPN service (lab only).
//!
//! Exit status: 1 invalid settings or configuration, 2 usage error,
//! 3 refused by a safety guard, 4 could not set up its sockets, 5 the
//! Lockdown Mode rules could not be installed or removed.
//! It runs until it is stopped. Stopping it never removes the kill switch:
//! only a deliberate `septuvo disconnect` does (owner decision Q2).

use septuvo_daemon::addresses::{self, IpCommand};
use septuvo_daemon::engine::{Engine, Nftables, SysClassNet};
use septuvo_daemon::resolver::ResolvConf;
use septuvo_daemon::store::{ServiceSettings, Store};
use septuvo_daemon::{guard, settings, sockets};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

const USAGE: &str = "usage: septuvod --settings FILE [--apply-lockdown | --recover]\n\n\
The privileged Septuvo VPN service. Development preview, lab only: it runs\n\
as root inside a named test network namespace and turns the kill switch on\n\
and off. It has no tunnel and protects nothing outside the lab.\n\n\
  --apply-lockdown  start-up step (run at boot before the network): if\n\
                    Lockdown Mode is on, install its blocking rules, then exit\n\
  --recover         administrator recovery (stop the service first): turn\n\
                    Lockdown Mode off and remove every Septuvo rule, then exit";

type Shared = Arc<Mutex<Engine<Nftables, SysClassNet, ResolvConf>>>;

fn serve(listener: &UnixListener, engine: &Shared, control: bool) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut handle = |request: &septuvo_ipc::Request| {
            engine
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .handle_request(request, control)
        };
        sockets::serve_one(&mut stream, &mut handle);
    }
}

fn fail(code: u8, message: &str) -> ExitCode {
    eprintln!("septuvod: {message}");
    ExitCode::from(code)
}

/// What this run does.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Serve,
    ApplyLockdown,
    Recover,
}

fn parse_args(args: &[String]) -> Option<(PathBuf, Mode)> {
    match args {
        [flag, path] if flag == "--settings" => Some((PathBuf::from(path), Mode::Serve)),
        [flag, path, m] if flag == "--settings" && m == "--apply-lockdown" => {
            Some((PathBuf::from(path), Mode::ApplyLockdown))
        }
        [flag, path, m] if flag == "--settings" && m == "--recover" => {
            Some((PathBuf::from(path), Mode::Recover))
        }
        _ => None,
    }
}

/// The captive-portal helper's user ID (N3-02): a dedicated account, never
/// root.
fn captive_helper(user: Option<&String>) -> Result<Option<u32>, String> {
    let Some(user) = user else {
        return Ok(None);
    };
    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    match guard::user_id(&passwd, user) {
        Some(0) => Err(format!("captive helper account '{user}' is root; refusing")),
        Some(uid) => Ok(Some(uid)),
        None => Err(format!("captive helper account '{user}' does not exist")),
    }
}

/// Follows address changes for Local Network Access (NP-L6): `ip monitor`
/// for immediate updates, and a slow poll in case it stops.
fn watch_addresses(engine: &Shared) {
    let monitor = Arc::clone(engine);
    thread::spawn(move || {
        loop {
            addresses::watch(|| {
                let _ = monitor
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .addresses_changed();
            });
            thread::sleep(Duration::from_secs(5));
        }
    });
    let poll = Arc::clone(engine);
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(10));
            let _ = poll
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .addresses_changed();
        }
    });
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.as_slice(), [flag] if flag == "--help" || flag == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let Some((path, mode)) = parse_args(&args) else {
        return fail(2, USAGE);
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
    let captive_helper = match captive_helper(settings.captive_helper_user.as_ref()) {
        Ok(h) => h,
        Err(e) => return fail(1, &e),
    };
    // Saved connections and service settings (N4-03, N4-04): a damaged or
    // unsafe store stops the service rather than being guessed at; rules
    // already installed stay.
    let mut store = match Store::open(&settings.connections_dir, true) {
        Ok(s) => s,
        Err(e) => return fail(1, &format!("saved connections: {e}")),
    };
    if mode == Mode::Recover {
        return recover(&mut store);
    }
    let resolver = ResolvConf::for_lab_namespace(&settings.lab_namespace);
    let mut engine = Engine::new(config, Nftables, SysClassNet, resolver)
        .with_captive_helper(captive_helper)
        .with_store(store)
        .with_addresses(Box::new(IpCommand));
    // Lockdown Mode (D-038): blocking from start-up when no connection is.
    let lockdown = engine.apply_startup();
    if mode == Mode::ApplyLockdown {
        return match lockdown {
            Ok(true) => {
                println!("septuvod: Lockdown Mode is on: blocking rules installed");
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("septuvod: Lockdown Mode is off: nothing to do");
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                5,
                &format!("Lockdown Mode rules could not be installed: {e}"),
            ),
        };
    }
    if let Err(e) = lockdown {
        eprintln!("septuvod: Lockdown Mode rules could not be installed: {e}");
    }
    serve_all(&settings, engine)
}

/// Administrator recovery (D-038): Lockdown Mode off and every Septuvo rule
/// removed. Run with the service stopped.
fn recover(store: &mut Store) -> ExitCode {
    let settings = ServiceSettings {
        lockdown: false,
        ..store.service()
    };
    if let Err(e) = store.set_service(settings) {
        return fail(1, &format!("cannot turn Lockdown Mode off: {}", e.message));
    }
    let mut firewall = Nftables;
    if let Err(e) = septuvo_daemon::engine::Firewall::apply(
        &mut firewall,
        &septuvo_policy::nft::removal_script(),
    ) {
        return fail(
            5,
            &format!("Lockdown Mode is off, but the rules could not be removed: {e}"),
        );
    }
    println!("septuvod: recovery done: Lockdown Mode is off and every Septuvo rule was removed");
    ExitCode::SUCCESS
}

fn serve_all(
    settings: &settings::Settings,
    engine: Engine<Nftables, SysClassNet, ResolvConf>,
) -> ExitCode {
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
    let engine: Shared = Arc::new(Mutex::new(engine));
    watch_addresses(&engine);
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
