//! cargo-fuzz target `nft_render`: nftables kill-switch renderer.
//!
//! Calls the same property check as the standard-library fuzz test
//! `fuzz_nft_render` in `crates/septuvo-policy/tests/fuzz.rs` (N3-03).

#![no_main]
#![allow(dead_code, unused_imports)]

#[path = "../../crates/septuvo-policy/tests/fuzz.rs"]
mod props;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = props::check_nft_render(data);
});
