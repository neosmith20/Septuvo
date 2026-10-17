//! cargo-fuzz target `status_socket_changes_nothing`: Change commands on the status socket.
//!
//! Calls the same property check as the standard-library fuzz test
//! `fuzz_status_socket_changes_nothing` in `crates/septuvo-daemon/tests/fuzz.rs` (N3-03).

#![no_main]
#![allow(dead_code, unused_imports)]

#[path = "../../crates/septuvo-daemon/tests/fuzz.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = props::check_status_socket_changes_nothing(data);
});
