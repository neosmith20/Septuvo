//! Fuzz targets for `septuvo-server` (N3-03): the device records file and
//! the record operations (add, revoke, remove).
//! Longer runs: `SEPTUVO_FUZZ_SECONDS=60 cargo test -p septuvo-server --test fuzz -- --nocapture`.

#[path = "../../septuvo-policy/tests/support/fuzzer.rs"]
mod fuzzer;

use fuzzer::{Corpus, Rng, run, word};
use septuvo_policy::Prefix;
use septuvo_server::records::{HEADER, MAX_DEVICES, Records, Scope, valid_name};
use std::fmt::Write as _;

fn ranges() -> (Prefix, Prefix) {
    (
        "10.77.0.0/24".parse().unwrap(),
        "fd77:77:77::/64".parse().unwrap(),
    )
}

const SEED: &str = "# Septuvo Server device records. Managed by septuvo-server; do not edit.\n\
septuvo-devices 1\n\
device laptop both 10.77.0.2 fd77:77:77::2 1760000000 - placeholder-not-a-key\n\
device phone home 10.77.0.3 fd77:77:77::3 1760000001 1760000100 placeholder-not-a-key\n";

const DICT: &[&str] = &[
    "device ",
    "septuvo-devices 1",
    "septuvo-devices 2",
    "home",
    "exit",
    "both",
    "Both",
    "-",
    "placeholder-not-a-key",
    "10.77.0.",
    "10.77.1.2",
    "fd77:77:77::",
    "fd77:77:78::1",
    " ",
    "\n",
    "#",
    "18446744073709551615",
    "18446744073709551616",
    "-1",
    "laptop",
    "\t",
];

/// Any error text must be safe to print to a terminal or log.
fn printable(text: &str) -> bool {
    text.bytes().all(|b| (0x20..=0x7e).contains(&b))
}

fn structured_records(rng: &mut Rng) -> Vec<u8> {
    let mut s = format!("{HEADER}\n");
    for i in 0..rng.below(12) {
        let name = if rng.chance(85) {
            format!("dev{i}{}", word(rng, b"abc-_.", 4))
        } else {
            word(rng, b"abcABC -_.:/", 40)
        };
        let scope = *rng.pick(&["home", "exit", "both", "all"]);
        let span = if rng.chance(90) { 250 } else { 300 };
        let host = 2 + rng.below(span);
        let revoked = if rng.chance(30) {
            rng.interesting_number().to_string()
        } else {
            "-".into()
        };
        let _ = writeln!(
            s,
            "device {name} {scope} 10.77.0.{host} fd77:77:77::{host:x} {} {revoked} placeholder-not-a-key",
            rng.interesting_number()
        );
    }
    s.into_bytes()
}

#[test]
fn fuzz_device_records() {
    let corpus = Corpus {
        seeds: &[SEED, "septuvo-devices 1\n"],
        dict: DICT,
    };
    let (v4, v6) = ranges();
    run("fuzz_device_records", &corpus, structured_records, |data| {
        let Ok(text) = std::str::from_utf8(data) else {
            return false;
        };
        match Records::decode(text, v4, v6) {
            Ok(r) => {
                assert!(r.devices().len() <= MAX_DEVICES);
                for d in r.devices() {
                    assert!(valid_name(&d.name), "accepted name {:?}", d.name);
                    assert!(v4.contains(d.ipv4.into()) && v6.contains(d.ipv6.into()));
                }
                // Round trip: encoded and decoded again, the same records; and
                // encoding is stable.
                let encoded = r.encode();
                let again = Records::decode(&encoded, v4, v6).expect("encoded records decode");
                assert_eq!(again, r, "round trip");
                assert_eq!(again.encode(), encoded, "stable encoding");
                true
            }
            Err(e) => {
                let msg = e.to_string();
                assert!(printable(&msg), "error text is printable: {msg:?}");
                assert!(msg.len() <= 300, "bounded error message: {msg}");
                false
            }
        }
    });
}

/// A random sequence of administrator operations. A refused operation never
/// changes the records; every state reached still round-trips.
#[test]
fn fuzz_record_operations() {
    let corpus = Corpus {
        seeds: &[],
        dict: &[],
    };
    let (v4, v6) = ranges();
    let structured = |rng: &mut Rng| rng.next_u64().to_le_bytes().to_vec();
    run("fuzz_record_operations", &corpus, structured, |data| {
        let mut seed = [0u8; 8];
        for (i, b) in data.iter().take(8).enumerate() {
            seed[i] = *b;
        }
        let mut rng = Rng::new(u64::from_le_bytes(seed));
        let mut records = Records::default();
        let mut refused = 0;
        for _ in 0..rng.below(40) {
            let name = if rng.chance(70) {
                format!("d{}", rng.below(8))
            } else {
                let any: String = (0..rng.below(40))
                    .map(|_| {
                        char::from_u32(u32::try_from(rng.next_u64() % 0x250).unwrap_or(0))
                            .unwrap_or('?')
                    })
                    .collect();
                any
            };
            let before = records.clone();
            let now = rng.interesting_number();
            let result = match rng.below(3) {
                0 => records
                    .add(
                        &name,
                        *rng.pick(&[Scope::Home, Scope::Exit, Scope::Both]),
                        now,
                        v4,
                        v6,
                    )
                    .map(|_| ()),
                1 => records.revoke(&name, now),
                _ => records.remove(&name),
            };
            if let Err(e) = result {
                refused += 1;
                assert_eq!(records, before, "a refused operation changes nothing");
                assert!(printable(&e), "error text is printable: {e:?}");
            }
            let again = Records::decode(&records.encode(), v4, v6).expect("always decodable");
            assert_eq!(again, records, "round trip after every operation");
        }
        refused == 0
    });
}
