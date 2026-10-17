//! A small, deterministic fuzz runner for Septuvo's parsers (N3-03).
//!
//! **Test code only.** The random numbers come from `SplitMix64`, which is
//! fast and reproducible and **not** suitable for anything security-related.
//! It is never used outside tests.
//!
//! Every case is generated from one 64-bit *case seed*, so any failure can be
//! replayed exactly. Each case is one of:
//! - random bytes;
//! - a string of dictionary tokens;
//! - a mutation of a seed input (bit flips, inserted, deleted, duplicated or
//!   spliced chunks, dictionary tokens);
//! - a structure-aware input from the target's own generator.
//!
//! Environment variables (all optional):
//!
//! | Variable | Meaning | Default |
//! | --- | --- | --- |
//! | `SEPTUVO_FUZZ_ITERS` | cases per target | 500 (keeps `cargo test` fast) |
//! | `SEPTUVO_FUZZ_SECONDS` | run each target for this long instead | unset |
//! | `SEPTUVO_FUZZ_SEED` | base seed (decimal or `0x` hex) | fixed per target |
//! | `SEPTUVO_FUZZ_REPLAY` | run exactly one case seed (`0x…`) | unset |
//!
//! Lives in `septuvo-policy` (Apache-2.0) and is included by path into the
//! other crates' fuzz tests; Apache-2.0 code may be used by the `PolyForm`
//! crates, not the other way round (D-033).

#![allow(dead_code)]

use std::fmt::Write as _;
use std::net::IpAddr;
use std::panic::{self, AssertUnwindSafe};
use std::time::{Duration, Instant};

/// Largest input any target is given, in bytes.
pub const MAX_LEN: usize = 4096;

/// `SplitMix64`. Reproducible test randomness; **not** for security.
pub struct Rng(u64);

impl Rng {
    /// A generator for one case.
    #[must_use]
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n` (0 if `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        let Ok(n64) = u64::try_from(n) else { return 0 };
        if n64 == 0 {
            return 0;
        }
        usize::try_from(self.next_u64() % n64).unwrap_or(0)
    }

    /// True with probability `percent` in 100.
    pub fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    /// One random byte.
    pub fn byte(&mut self) -> u8 {
        self.next_u64().to_le_bytes()[0]
    }

    /// One item of a non-empty slice.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    /// A small number, biased towards limits: 0, 1, around powers of two,
    /// and the edges of `u8`/`u16`/`u32`.
    pub fn interesting_number(&mut self) -> u64 {
        const EDGES: [u64; 14] = [
            0,
            1,
            2,
            9,
            10,
            63,
            64,
            65,
            255,
            256,
            65_535,
            65_536,
            4_294_967_295,
            u64::MAX,
        ];
        if self.chance(50) {
            *self.pick(&EDGES)
        } else {
            self.next_u64() >> self.below(64)
        }
    }
}

/// Seed inputs and dictionary tokens for one target.
pub struct Corpus<'a> {
    /// Valid or nearly valid examples to mutate.
    pub seeds: &'a [&'a str],
    /// Tokens that mean something to the parser.
    pub dict: &'a [&'a str],
}

/// Mutates `data` in place a few times.
pub fn mutate(rng: &mut Rng, data: &mut Vec<u8>, corpus: &Corpus<'_>) {
    for _ in 0..=rng.below(8) {
        let len = data.len();
        match rng.below(9) {
            0 if len > 0 => {
                let i = rng.below(len);
                data[i] ^= 1 << rng.below(8);
            }
            1 if len > 0 => {
                let i = rng.below(len);
                data[i] = rng.byte();
            }
            2 => {
                let i = rng.below(len + 1);
                data.insert(i, rng.byte());
            }
            3 if len > 0 => {
                let i = rng.below(len);
                let n = 1 + rng.below((len - i).min(32));
                data.drain(i..i + n);
            }
            4 if len > 0 => {
                // duplicate a chunk (repeated keys, lines, fields)
                let i = rng.below(len);
                let n = 1 + rng.below((len - i).min(64));
                let chunk: Vec<u8> = data[i..i + n].to_vec();
                let at = rng.below(data.len() + 1);
                data.splice(at..at, chunk);
            }
            5 if !corpus.dict.is_empty() => {
                let token = rng.pick(corpus.dict).as_bytes().to_vec();
                let at = rng.below(len + 1);
                data.splice(at..at, token);
            }
            6 if !corpus.seeds.is_empty() => {
                // splice part of another seed
                let other = rng.pick(corpus.seeds).as_bytes();
                if !other.is_empty() {
                    let i = rng.below(other.len());
                    let n = 1 + rng.below(other.len() - i);
                    let at = rng.below(len + 1);
                    data.splice(at..at, other[i..i + n].iter().copied());
                }
            }
            7 => {
                // an interesting number as text
                let n = rng.interesting_number().to_string().into_bytes();
                let at = rng.below(len + 1);
                data.splice(at..at, n);
            }
            _ if len > 0 => data.truncate(rng.below(len)),
            _ => data.push(rng.byte()),
        }
    }
    data.truncate(MAX_LEN);
}

/// One input for a case.
pub fn input(
    rng: &mut Rng,
    corpus: &Corpus<'_>,
    structured: &mut dyn FnMut(&mut Rng) -> Vec<u8>,
) -> Vec<u8> {
    let mut data = match rng.below(10) {
        0 => {
            let n = rng.below(256);
            (0..n).map(|_| rng.byte()).collect()
        }
        1 | 2 if !corpus.dict.is_empty() => {
            let mut v = Vec::new();
            for _ in 0..rng.below(24) {
                v.extend_from_slice(rng.pick(corpus.dict).as_bytes());
                if rng.chance(30) {
                    v.push(*rng.pick(b" \n=\"[],#\t:"));
                }
            }
            v
        }
        3..=6 if !corpus.seeds.is_empty() => {
            let mut v = rng.pick(corpus.seeds).as_bytes().to_vec();
            mutate(rng, &mut v, corpus);
            v
        }
        _ => {
            let mut v = structured(rng);
            if rng.chance(30) {
                mutate(rng, &mut v, corpus);
            }
            v
        }
    };
    data.truncate(MAX_LEN);
    data
}

fn env_u64(name: &str) -> Option<u64> {
    let v = std::env::var(name).ok()?;
    let v = v.trim();
    match v.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16).ok(),
        None => v.parse().ok(),
    }
}

/// FNV-1a, to give each target its own default seed.
fn name_hash(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Shows an input for a failure report: printable ASCII as is, everything
/// else escaped, at most 600 bytes.
#[must_use]
pub fn show(data: &[u8]) -> String {
    let mut s: String = data
        .iter()
        .take(600)
        .map(|&b| match b {
            b'\n' => "\\n".to_string(),
            0x20..=0x7e if b != b'\\' => char::from(b).to_string(),
            _ => format!("\\x{b:02x}"),
        })
        .collect();
    if data.len() > 600 {
        let _ = write!(s, "... ({} bytes)", data.len());
    }
    s
}

/// Runs `check` on generated inputs. `check` panics when a property fails,
/// and returns true if the parser accepted the input (reported as a ratio,
/// so a target that never reaches its deeper code paths is visible).
/// The failing case seed and input are printed so the case can be replayed
/// (`SEPTUVO_FUZZ_REPLAY`) and turned into a regression test.
pub fn run(
    target: &str,
    corpus: &Corpus<'_>,
    mut structured: impl FnMut(&mut Rng) -> Vec<u8>,
    mut check: impl FnMut(&[u8]) -> bool,
) {
    // Seeds alone first: they must always pass.
    for seed in corpus.seeds {
        check(seed.as_bytes());
    }
    let mut accepted: u64 = 0;
    let base = env_u64("SEPTUVO_FUZZ_SEED").unwrap_or(0x5E97_0F00_0000_0000) ^ name_hash(target);
    let replay = env_u64("SEPTUVO_FUZZ_REPLAY");
    let seconds = env_u64("SEPTUVO_FUZZ_SECONDS");
    let iters = env_u64("SEPTUVO_FUZZ_ITERS").unwrap_or(500);
    let deadline = seconds.map(|s| Instant::now() + Duration::from_secs(s));
    let started = Instant::now();
    let mut cases: u64 = 0;
    loop {
        let case_seed = match replay {
            Some(s) => s,
            None => Rng::new(base.wrapping_add(cases)).next_u64(),
        };
        let mut rng = Rng::new(case_seed);
        let data = input(&mut rng, corpus, &mut structured);
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| check(&data)));
        cases += 1;
        if matches!(outcome, Ok(true)) {
            accepted += 1;
        }
        if let Err(cause) = outcome {
            eprintln!(
                "FUZZ FAILURE target={target} case_seed=0x{case_seed:016x}\n  input: {}\n  \
                 replay: SEPTUVO_FUZZ_REPLAY=0x{case_seed:016x} cargo test --test fuzz {target} -- --nocapture",
                show(&data)
            );
            panic::resume_unwind(cause);
        }
        let done = match (replay, deadline) {
            (Some(_), _) => true,
            (None, Some(d)) => Instant::now() >= d,
            (None, None) => cases >= iters,
        };
        if done {
            break;
        }
    }
    eprintln!(
        "fuzz {target}: {cases} cases ({accepted} accepted) in {:.1}s, base seed 0x{base:016x}: \
         all properties held",
        started.elapsed().as_secs_f64()
    );
}

/// A random string made of `alphabet` characters, up to `max` long.
pub fn word(rng: &mut Rng, alphabet: &[u8], max: usize) -> String {
    let n = rng.below(max + 1);
    (0..n).map(|_| char::from(*rng.pick(alphabet))).collect()
}

/// A random IP address as text: IPv4 or IPv6, often a well-known one.
pub fn ip_text(rng: &mut Rng) -> String {
    const KNOWN: [&str; 10] = [
        "0.0.0.0",
        "127.0.0.1",
        "10.77.0.1",
        "192.168.1.1",
        "255.255.255.255",
        "224.0.0.251",
        "::",
        "::1",
        "fd77::1",
        "ff02::fb",
    ];
    if rng.chance(30) {
        return (*rng.pick(&KNOWN)).to_string();
    }
    if rng.chance(60) {
        format!(
            "{}.{}.{}.{}",
            rng.byte(),
            rng.byte(),
            rng.byte(),
            rng.byte()
        )
    } else {
        let parts: Vec<String> = (0..8)
            .map(|_| format!("{:x}", rng.next_u64() & 0xffff))
            .collect();
        parts.join(":")
    }
}

/// A random, valid IP address.
pub fn ip(rng: &mut Rng) -> IpAddr {
    loop {
        if let Ok(a) = ip_text(rng).parse() {
            return a;
        }
    }
}

/// A prefix with its host bits cleared, so it is usually valid; sometimes
/// left as generated, so the "host bits set" path is exercised too.
pub fn prefix_text(rng: &mut Rng) -> String {
    let addr = ip(rng);
    let len = match addr {
        IpAddr::V4(_) => rng.below(33),
        IpAddr::V6(_) => rng.below(129),
    };
    if rng.chance(15) {
        return format!("{addr}/{}", rng.below(140));
    }
    let masked: IpAddr = match addr {
        IpAddr::V4(a) => {
            let bits = u32::from(a);
            let keep = if len == 0 { 0 } else { u32::MAX << (32 - len) };
            IpAddr::from((bits & keep).to_be_bytes())
        }
        IpAddr::V6(a) => {
            let bits = u128::from(a);
            let keep = if len == 0 {
                0
            } else {
                u128::MAX << (128 - len)
            };
            IpAddr::from((bits & keep).to_be_bytes())
        }
    };
    format!("{masked}/{len}")
}
