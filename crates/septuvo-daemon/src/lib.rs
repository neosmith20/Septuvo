//! `septuvod`, the privileged Septuvo VPN service (N2-02).
//!
//! The only Septuvo component allowed to change network state. The
//! unprivileged `septuvo` tool talks to it over two Unix sockets
//! ([`sockets`]), using the open protocol in `septuvo-ipc`.
//!
//! **Before the G1 security review it is a lab tool:**
//! - it runs only as root, only with `mode = "lab"`, and only inside the
//!   named network namespace its settings name ([`guard`]);
//! - "connect" turns on the N1 kill switch, with no tunnel; peer
//!   authentication always reports *failed*, so status is never
//!   "connected" ([`engine`]);
//! - with tunnel DNS, connect also locks DNS to the tunnel resolvers and
//!   points the lab namespace's resolver file at them ([`resolver`]);
//! - nothing here encrypts, handshakes, handles keys or carries traffic.
//!
//! Licensed under the `PolyForm` Noncommercial License 1.0.0 (D-033); see
//! `LICENSE.md` and `NOTICE` in this crate.

pub mod engine;
pub mod guard;
pub mod resolver;
pub mod settings;
pub mod sockets;
