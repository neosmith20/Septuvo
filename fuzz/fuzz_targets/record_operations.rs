//! cargo-fuzz target `record_operations`: Device record operations.
//!
//! Calls the same property check as the standard-library fuzz test
//! `fuzz_record_operations` in `crates/septuvo-server/tests/fuzz.rs` (N3-03).

#![no_main]
#![allow(dead_code, unused_imports)]

#[path = "../../crates/septuvo-server/tests/fuzz.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = props::check_record_operations(data);
});
