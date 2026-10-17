//! Fuzz targets for `septuvo-ipc` (N3-03): requests, responses and the
//! response builder of the local control protocol.
//! Longer runs: `SEPTUVO_FUZZ_SECONDS=60 cargo test -p septuvo-ipc --test fuzz -- --nocapture`.

#[path = "../../septuvo-policy/tests/support/fuzzer.rs"]
mod fuzzer;

use fuzzer::{Corpus, Rng, run, word};
use septuvo_ipc::{
    Command, MAX_FIELDS, MAX_KEY_BYTES, MAX_RESPONSE_BYTES, MAX_VALUE_BYTES, Outcome, Response,
    parse_request, parse_response,
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
    run(
        "fuzz_ipc_request",
        &corpus,
        structured,
        |data| match parse_request(data) {
            // Only the canonical encoding of a command is ever accepted.
            Ok(command) => {
                assert_eq!(command.encode().as_bytes(), data, "canonical form");
                if let Command::Captive { minutes, .. } = command {
                    assert!((1..=septuvo_ipc::MAX_CAPTIVE_MINUTES).contains(&minutes));
                }
                true
            }
            Err(_) => false,
        },
    );
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
    run("fuzz_ipc_response", &corpus, structured_response, |data| {
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
    });
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
    run("fuzz_ipc_builder", &corpus, structured, |data| {
        let mut seed = [0u8; 8];
        for (i, b) in data.iter().take(8).enumerate() {
            seed[i] = *b;
        }
        let mut rng = Rng::new(u64::from_le_bytes(seed));
        let any = |rng: &mut Rng, max: usize| -> String {
            let n = rng.below(max + 1);
            (0..n)
                .map(|_| {
                    char::from_u32(u32::try_from(rng.next_u64() % 0x2_0000).unwrap_or(0))
                        .unwrap_or('?')
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
        let encoded = r.encode();
        assert!(
            encoded.len() <= MAX_RESPONSE_BYTES,
            "built response of {} bytes exceeds the {MAX_RESPONSE_BYTES}-byte limit",
            encoded.len()
        );
        let parsed = parse_response(encoded.as_bytes()).expect("a built response always parses");
        if let Outcome::Err { code, .. } = &parsed.outcome {
            assert!(!code.is_empty());
        }
        true
    });
}
