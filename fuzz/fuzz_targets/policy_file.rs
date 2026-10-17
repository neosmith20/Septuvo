//! cargo-fuzz target `policy_file`: Policy file format.
//!
//! Calls the same property check as the standard-library fuzz test
//! `fuzz_policy_file` in `crates/septuvo-policy/tests/fuzz.rs` (N3-03).

#![no_main]
#![allow(dead_code, unused_imports)]

#[path = "../../crates/septuvo-policy/tests/fuzz.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = props::check_policy_file(data);
});
