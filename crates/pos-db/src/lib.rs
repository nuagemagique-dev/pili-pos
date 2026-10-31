//! SQLite persistence: schema, migrations and typed queries, with the fiscal archive append-only.

pub mod migrate;
pub mod queries;
pub mod schema;
mod schema_badges;
#[cfg(feature = "catalog-sync")]
mod schema_catalog;
mod schema_eft;
#[cfg(feature = "hubrise")]
mod schema_hubrise;
mod schema_invoicing;
mod schema_owner;
mod schema_till;
mod schema_weight;
