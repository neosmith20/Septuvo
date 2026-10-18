//! Septuvo command-line library (N1-04): the four-stage connection status
//! model shared by the CLI now and by the service and apps later, and the
//! `policy check` command.
//!
//! [`service`] talks to the local Septuvo service (`septuvod`) over its
//! Unix sockets (N2-02); [`watch`] is the live status view (N3-04).
//!
//! Development preview: no VPN networking, no cryptography; the CLI itself
//! never changes the system (only the service does).

pub mod policy_cmd;
pub mod service;
pub mod status;
pub mod watch;
