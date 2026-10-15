//! Septuvo command-line library (N1-04): the four-stage connection status
//! model shared by the CLI now and by the service and apps later, and the
//! `policy check` command.
//!
//! Development preview: no networking, no cryptography, no system changes.

pub mod policy_cmd;
pub mod status;
