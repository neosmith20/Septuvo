//! cargo-fuzz target `ipc_framed`: Control protocol requests with bodies (N4-03).
//!
//! Calls the same property check as the standard-library fuzz test
//! `fuzz_ipc_framed` in `crates/septuvo-ipc/tests/fuzz.rs` (N3-03).

#![no_main]
#![allow(dead_code, unused_imports)]

#[path = "../../crates/septuvo-ipc/tests/fuzz.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = props::check_ipc_framed(data);
});
