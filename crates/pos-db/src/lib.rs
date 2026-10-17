//! SQLite persistence: schema, migrations and typed queries, with the fiscal archive append-only.

pub mod migrate;
pub mod queries;
pub mod schema;
#[cfg(feature = "catalog-sync")]
mod schema_catalog;
mod schema_eft;
#[cfg(feature = "hubrise")]
mod schema_hubrise;
mod schema_v12;
