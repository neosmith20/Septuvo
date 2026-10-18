//! cargo-fuzz target `ipc_response`: Control protocol responses.
//!
//! Calls the same property check as the standard-library fuzz test
//! `fuzz_ipc_response` in `crates/septuvo-ipc/tests/fuzz.rs` (N3-03).

#![no_main]
#![allow(dead_code, unused_imports)]

#[path = "../../crates/septuvo-ipc/tests/fuzz.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = props::check_ipc_response(data);
});
