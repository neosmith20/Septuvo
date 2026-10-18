//! cargo-fuzz target `server_schema`: Server configuration (septuvo-server.toml).
//!
//! Calls the same property check as the standard-library fuzz test
//! `fuzz_server_schema` in `crates/septuvo-config/tests/fuzz.rs` (N3-03).

#![no_main]
#![allow(dead_code, unused_imports)]

#[path = "../../crates/septuvo-config/tests/fuzz.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = props::check_server_schema(data);
});
