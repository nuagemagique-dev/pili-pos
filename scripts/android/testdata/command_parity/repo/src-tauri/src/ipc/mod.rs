//! Fixture: two plain modules and one gated by the peppol feature.

pub mod a;
#[cfg(feature = "peppol")]
pub mod inv;
