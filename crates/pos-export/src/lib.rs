//! Nightly export of a pili-pos installation (phase X1, decisions X1D4 to X1D9).
//!
//! After each real Z report the POS produces three files and hands them to an
//! outbox that uploads them to a per-installation HTTPS endpoint:
//!
//! - `pos-<id>.db.age`: a full backup of `pos.db`, encrypted with age to the
//!   owner (and, when configured, the producer);
//! - `stats-<id>.sqlite`: a cleaned statistics database without personal data;
//! - `manifest-<id>.json`: format `pili-export/1`, the SHA-256 and size of the
//!   other two files, the recipients and the identity of the installation.
//!
//! `<id>` is the export id `<YYYYMMDD>-z<6 digits>` (booking date and Z report
//! number, see [`naming`]). The export is configured by the optional `export`
//! section of `pos.json` (`pos_core::config::ExportConfig`); an absent section
//! means no export. It never blocks a sale: every failure is kept in the outbox
//! and retried.
//!
//! Modules:
//!
//! - [`naming`], [`settings`], [`snapshot`], [`crypt`], [`manifest`]: the base
//!   (card X1-T2);
//! - [`zref`], [`pseudonym`], [`stats`]: the statistics file (card X1-T3);
//! - [`upload`]: the HTTPS PUT client (card X1-T4);
//! - [`outbox`], [`status`]: the retry queue and the export status (card X1-T5).
//!
//! Public structs use `i64`, never unsigned integers (the FFI forbids them).
//! Errors are plain `String` messages that never contain a token or a secret
//! key.

pub mod crypt;
pub mod manifest;
pub mod naming;
pub mod outbox;
pub mod pseudonym;
pub mod settings;
pub mod snapshot;
pub mod stats;
pub mod status;
pub mod upload;
pub mod zref;

pub use outbox::{ExportEnv, PassReport};
pub use settings::ExportSettings;
pub use status::ExportStatus;
pub use upload::Uploader;
