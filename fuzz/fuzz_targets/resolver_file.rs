//! cargo-fuzz target `resolver_file`: Resolver file (network resolvers).
//!
//! Calls the same property check as the standard-library fuzz test
//! `fuzz_resolver_file` in `crates/septuvo-daemon/tests/fuzz.rs` (N3-03).

#![no_main]
#![allow(dead_code, unused_imports)]

#[path = "../../crates/septuvo-daemon/tests/fuzz.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = props::check_resolver_file(data);
});
