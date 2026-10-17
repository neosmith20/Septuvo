//! Septuvo Server administration foundations (N2-04).
//!
//! The local `septuvo-server` tool keeps the server's device records: who is
//! enrolled, what each device may reach, and its tunnel addresses. It runs
//! on the server host with administrator rights, and there is no network
//! administration interface (AD-1).
//!
//! **Before the G1 security review no device can connect:**
//! - there are no invitations, no enrollment, no keys and no network
//!   listener;
//! - each record holds a [`records::PlaceholderKey`], which is not a key.
//!
//! The records are built and stored now, so that reviewed enrollment can
//! fill in real keys later without changing the administration commands.
//!
//! Licensed under the `PolyForm` Noncommercial License 1.0.0 (D-033); see
//! `LICENSE.md` and `NOTICE` in this crate.

pub mod cli;
pub mod records;
pub mod store;
