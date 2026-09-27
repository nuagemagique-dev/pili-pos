//! The POS menu as a mirror of an external catalog.
//!
//! A source (HubRise today, the owner's catalog service later) maps
//! its own format into the neutral [`Catalog`]; this crate validates
//! it, and applies it with stable product ids. Nothing here knows
//! HubRise.

pub mod apply;
pub mod hash;
pub mod model;
#[doc(hidden)]
pub mod testing;
pub mod validate;

pub use apply::{
    Adoption, ApplyOutcome, Error, SyncState, adopt, adopt_and_apply, adoption_preview, apply,
    apply_checked, managed_by, pending_catalog, release, state,
};
pub use hash::hash;
pub use model::{Catalog, Category, Choice, Group, Item, ItemKind, Names, fiscal_name};
pub use validate::{Issue, IssueCode, validate};
