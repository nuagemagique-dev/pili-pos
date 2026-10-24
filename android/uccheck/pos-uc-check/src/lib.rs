//! `pos-uc-check`: the host side of the P7 end-to-end parity proof.
//!
//! - `record` plays scenario groups with the engine compiled for the host
//!   (the reference run) and writes one journal per scenario;
//! - `compare` checks the journals of the Android run against the
//!   reference ones.
//!
//! The scenario vocabulary and the journal format are the spec of
//! `docs/android/phases/P7.md` section 4.

pub mod cli;
pub mod compare;
pub mod host;
pub mod journal;
pub mod record;
pub mod scenario;
pub mod ticket;
pub mod volatile;
