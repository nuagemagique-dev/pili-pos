//! Crate-wide error type composing the per-module errors.

use crate::render::RenderError;
use crate::tem::TemError;

/// Umbrella error for the outbox orchestration layer, which touches
/// SQLite, the render step and the TEM HTTP client.
#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error(transparent)]
    Tem(#[from] TemError),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Render(#[from] RenderError),
}
