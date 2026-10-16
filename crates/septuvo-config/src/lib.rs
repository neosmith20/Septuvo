//! Septuvo configuration: file format, parser and validation (N2-01).
//!
//! - [`toml`]: a strict, bounded subset of TOML (standard library only).
//! - [`device`]: the device file, `septuvo.toml`.
//! - [`server`]: the Septuvo Server file, `septuvo-server.toml`.
//!
//! Configuration is treated as hostile input: sizes are bounded, unknown
//! keys are errors, and a missing security setting falls back to the safe
//! choice or is an error, never to "off".
//!
//! Track N2 (D-029): no networking and no cryptography. Settings that need
//! enrollment or the Septuvo Protocol are placeholders until the G1 review.
//!
//! Licensed under the `PolyForm` Noncommercial License 1.0.0 (D-033); see `LICENSE.md`
//! and `NOTICE` in this crate.

pub mod device;
pub mod error;
mod schema;
pub mod server;
pub mod toml;

pub use device::{
    DeviceConfig, DnsPolicy, ServerIdentity, TunnelSettings, load_device, parse_device,
};
pub use error::{ConfigError, Problem};
pub use schema::{DEFAULT_RETENTION_DAYS, MAX_RETENTION_DAYS};
pub use server::{ServerConfig, load_server, parse_server};

#[cfg(test)]
mod examples {
    #[test]
    fn shipped_examples_are_valid() {
        for text in [
            include_str!("../examples/device-full-tunnel.toml"),
            include_str!("../examples/device-split.toml"),
        ] {
            crate::parse_device(text).expect("device example is valid");
        }
        crate::parse_server(include_str!("../examples/server.toml"))
            .expect("server example is valid");
    }
}
