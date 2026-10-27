//! Schema of the menu mirrored from an external catalog (feature
//! `catalog-sync`, part of `hubrise`), migrations 21 and 26.

/// v21 - catalog sync: charges and online-only items stay out of the
/// POS grid, option groups and options can be taken off sale, and the
/// state of the last sync per source is kept (hash, issues, raw
/// catalog). `manages_menu` locks local menu editing.
pub(crate) const MIGRATION_021_CATALOG_SYNC: &str = r"
ALTER TABLE categories ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0;
ALTER TABLE menu_items ADD COLUMN kind TEXT NOT NULL DEFAULT 'product'
    CHECK(kind IN ('product','charge'));
ALTER TABLE menu_items ADD COLUMN on_pos INTEGER NOT NULL DEFAULT 1;
ALTER TABLE modifier_groups ADD COLUMN active INTEGER NOT NULL DEFAULT 1;
ALTER TABLE modifier_groups ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
ALTER TABLE modifiers ADD COLUMN active INTEGER NOT NULL DEFAULT 1;

CREATE TABLE IF NOT EXISTS catalog_sync_state (
    source TEXT PRIMARY KEY,
    manages_menu INTEGER NOT NULL DEFAULT 0,
    hash TEXT,
    applied_at TEXT,
    rejected_at TEXT,
    issues_json TEXT,
    catalog_json TEXT
);
";

/// v26 - the menu code of a product ("A2"), from the catalog. Display
/// only: the till can show it in place of the name.
pub(crate) const MIGRATION_026_PRODUCT_CODES: &str = r"
ALTER TABLE menu_items ADD COLUMN code TEXT;
";
