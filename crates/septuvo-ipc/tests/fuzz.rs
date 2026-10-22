//! Fuzz targets for `septuvo-ipc` (N3-03): requests, responses and the
//! response builder of the local control protocol.
//! Longer runs: `SEPTUVO_FUZZ_SECONDS=60 cargo test -p septuvo-ipc --test fuzz -- --nocapture`.

// The check functions are public so the cargo-fuzz targets (fuzz/) can call
// them; they panic on purpose when a property fails.
#![allow(clippy::must_use_candidate, clippy::missing_panics_doc)]

#[path = "../../septuvo-policy/tests/support/fuzzer.rs"]
mod fuzzer;

use fuzzer::{Corpus, Rng, run, word};
use septuvo_ipc::{
    Command, MAX_BODY_BYTES, MAX_FIELDS, MAX_KEY_BYTES, MAX_RESPONSE_BYTES, MAX_VALUE_BYTES,
    Outcome, Response, parse_framed, parse_request, parse_response, read_request,
};
use std::fmt::Write as _;

const DICT: &[&str] = &[
    "SEPTUVO/1",
    "SEPTUVO/2",
    " ",
    "STATUS",
    "CONNECT",
    "DISCONNECT",
    "CAPTIVE",
    "CAPTIVE 5",
    "NETWORK-DNS",
    "\n",
    "\r\n",
    "OK",
    "ERR",
    "END\n",
    "=",
    "\0",
    "\u{1b}[2J",
    "state=blocking\n",
    "LIST",
    "GET c0001",
    "DELETE c9999",
    "SELECT c0002",
    "ADD 5\n",
    "REPLACE c0001 3\n",
    "RENAME c0001 4\n",
    "BODY 4\n",
    "c0001\tHome\n",
    "\u{e9}",
    "\u{1f3e0}",
];

#[test]
fn fuzz_ipc_request() {
    let corpus = Corpus {
        seeds: &[
            "SEPTUVO/1 STATUS\n",
            "SEPTUVO/1 CONNECT\n",
            "SEPTUVO/1 DISCONNECT\n",
            "SEPTUVO/1 CAPTIVE 5\n",
            "SEPTUVO/1 CAPTIVE 1 NETWORK-DNS\n",
        ],
        dict: DICT,
    };
    let structured = |rng: &mut Rng| {
        let words = ["STATUS", "CONNECT", "DISCONNECT", "CAPTIVE"];
        let mut s = format!("SEPTUVO/1 {}", rng.pick(&words));
        if rng.chance(50) {
            let _ = write!(s, " {}", rng.interesting_number() % 12);
        }
        if rng.chance(30) {
            s.push_str(" NETWORK-DNS");
        }
        s.push('\n');
        s.into_bytes()
    };
    run("fuzz_ipc_request", &corpus, structured, check_ipc_request);
}

/// Requests with bodies (N4-03): the header announces a length; the body
/// may be shorter, longer, or not text at all.
#[test]
fn fuzz_ipc_framed() {
    let corpus = Corpus {
        seeds: &[
            "SEPTUVO/1 ADD 5\nabcde",
            "SEPTUVO/1 RENAME c0001 8\nHome VPN",
            "SEPTUVO/1 REPLACE c0002 12\nversion = 1\n",
            "SEPTUVO/1 LIST\n",
            "SEPTUVO/1 SELECT c0001\n",
        ],
        dict: DICT,
    };
    let structured = |rng: &mut Rng| {
        let body = word(rng, b"ab =\n\t\"[]\xc3\xa9", 40);
        let len = if rng.chance(80) {
            body.len()
        } else {
            usize::try_from(rng.interesting_number() % 70_000).unwrap_or(0)
        };
        let id = rng.interesting_number() % 10_001;
        let header = match rng.below(4) {
            0 => format!("SEPTUVO/1 ADD {len}"),
            1 => format!("SEPTUVO/1 REPLACE c{id:04} {len}"),
            2 => format!("SEPTUVO/1 RENAME c{id:04} {len}"),
            _ => format!(
                "SEPTUVO/1 {} c{id:04}",
                rng.pick(&["GET", "DELETE", "SELECT"])
            ),
        };
        let mut out = format!("{header}\n").into_bytes();
        out.extend_from_slice(body.as_bytes());
        out
    };
    run("fuzz_ipc_framed", &corpus, structured, check_ipc_framed);
}

/// Property check for `fuzz_ipc_framed` (also called by the cargo-fuzz target).
pub fn check_ipc_framed(data: &[u8]) -> bool {
    // Reading never takes more than one request, and what it takes parses
    // the same way.
    let mut reader = data;
    let read = read_request(&mut reader).expect("reading from memory cannot fail");
    assert!(data.starts_with(&read));
    assert!(read.len() <= septuvo_ipc::MAX_REQUEST_BYTES + 1 + MAX_BODY_BYTES);
    match parse_framed(data) {
        Ok(request) => {
            // Only the canonical encoding is accepted, body included.
            assert_eq!(request.encode(), data, "canonical form");
            assert_eq!(request.body.is_some(), request.command.has_body());
            assert_eq!(parse_framed(&read).as_ref(), Ok(&request), "read the same");
            true
        }
        Err(_) => false,
    }
}

fn structured_response(rng: &mut Rng) -> Vec<u8> {
    let mut s = if rng.chance(70) {
        String::from("SEPTUVO/1 OK\n")
    } else {
        format!(
            "SEPTUVO/1 ERR {} {}\n",
            word(rng, b"abcdefghijklmnopqrstuvwxyz_0123456789", 34),
            word(rng, b"abc ~!=", 210)
        )
    };
    for _ in 0..rng.below(40) {
        let key = word(rng, b"abcdefghijklmnopqrstuvwxyz_0123456789A", 34);
        let value = word(rng, b"abc =~\t", 210);
        let _ = writeln!(s, "{key}={value}");
    }
    if rng.chance(30) {
        let body = word(rng, b"c01\tHome\nEND\xc3\xa9", 60);
        let len = if rng.chance(85) {
            body.len()
        } else {
            usize::try_from(rng.interesting_number() % 70_000).unwrap_or(0)
        };
        let _ = write!(s, "BODY {len}\n{body}");
    }
    s.push_str("END\n");
    s.into_bytes()
}

#[test]
fn fuzz_ipc_response() {
    let corpus = Corpus {
        seeds: &[
            "SEPTUVO/1 OK\nstate=blocking\nkill_switch=on\nEND\n",
            "SEPTUVO/1 ERR not_permitted use the control socket\nEND\n",
            "SEPTUVO/1 ERR limit\nEND\n",
            "SEPTUVO/1 OK\nEND\n",
        ],
        dict: DICT,
    };
    run(
        "fuzz_ipc_response",
        &corpus,
        structured_response,
        check_ipc_response,
    );
}

/// The service builds every response with `Response::ok/error/with`. Whatever
/// it is given (hostile interface names, long notes, many fields), the result
/// must always be a response the client can read.
#[test]
fn fuzz_ipc_builder() {
    let corpus = Corpus {
        seeds: &[],
        dict: &[],
    };
    let structured = |rng: &mut Rng| rng.next_u64().to_le_bytes().to_vec();
    run("fuzz_ipc_builder", &corpus, structured, check_ipc_builder);
}

/// Property check for `fuzz_ipc_request` (also called by the cargo-fuzz target).
pub fn check_ipc_request(data: &[u8]) -> bool {
    match parse_request(data) {
        // Only the canonical encoding of a command is ever accepted.
        Ok(command) => {
            assert_eq!(command.encode().as_bytes(), data, "canonical form");
            if let Command::Captive { minutes, .. } = command {
                assert!((1..=septuvo_ipc::MAX_CAPTIVE_MINUTES).contains(&minutes));
            }
            true
        }
        Err(_) => false,
    }
}

/// Property check for `fuzz_ipc_response` (also called by the cargo-fuzz target).
pub fn check_ipc_response(data: &[u8]) -> bool {
    match parse_response(data) {
        Ok(r) => {
            assert!(r.fields.len() <= MAX_FIELDS);
            // What the client accepted re-encodes to something it accepts
            // again, unchanged.
            let again = parse_response(r.encode().as_bytes());
            assert_eq!(again.as_ref(), Ok(&r), "round trip");
            true
        }
        Err(_) => false,
    }
}

/// Property check for `fuzz_ipc_builder` (also called by the cargo-fuzz target).
pub fn check_ipc_builder(data: &[u8]) -> bool {
    let mut seed = [0u8; 8];
    for (i, b) in data.iter().take(8).enumerate() {
        seed[i] = *b;
    }
    let mut rng = Rng::new(u64::from_le_bytes(seed));
    let any = |rng: &mut Rng, max: usize| -> String {
        let n = rng.below(max + 1);
        (0..n)
            .map(|_| {
                char::from_u32(u32::try_from(rng.next_u64() % 0x2_0000).unwrap_or(0)).unwrap_or('?')
            })
            .collect()
    };
    let mut r = if rng.chance(50) {
        Response::ok()
    } else {
        let code = if rng.chance(50) {
            word(&mut rng, b"abc_", MAX_KEY_BYTES + 2)
        } else {
            any(&mut rng, 40)
        };
        let message = any(&mut rng, MAX_VALUE_BYTES + 50);
        Response::error(&code, &message)
    };
    for _ in 0..rng.below(MAX_FIELDS + 10) {
        let key = if rng.chance(80) {
            word(&mut rng, b"abcdefghijklmnopqrstuvwxyz_", MAX_KEY_BYTES + 2)
        } else {
            any(&mut rng, 40)
        };
        let value = any(&mut rng, MAX_VALUE_BYTES + 50);
        r = r.with(&key, &value);
    }
    if rng.chance(30) {
        r = r.with_body(&any(&mut rng, 300));
    }
    let encoded = r.encode();
    let body = r
        .body
        .as_ref()
        .map_or(0, |b| b.len() + "BODY 65536\n".len());
    assert!(body <= MAX_BODY_BYTES + 16);
    assert!(
        encoded.len() - body <= MAX_RESPONSE_BYTES,
        "built response of {} bytes exceeds the {MAX_RESPONSE_BYTES}-byte limit",
        encoded.len()
    );
    let parsed = parse_response(encoded.as_bytes()).expect("a built response always parses");
    if let Outcome::Err { code, .. } = &parsed.outcome {
        assert_ne!(code.len(), 0);
    }
    true
}
