//! cargo-fuzz target `device_records`: Device records file (devices.db).
//!
//! Calls the same property check as the standard-library fuzz test
//! `fuzz_device_records` in `crates/septuvo-server/tests/fuzz.rs` (N3-03).

#![no_main]
#![allow(dead_code, unused_imports)]

#[path = "../../crates/septuvo-server/tests/fuzz.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = props::check_device_records(data);
});
