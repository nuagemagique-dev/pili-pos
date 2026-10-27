//! `HubRise` Inventory resource.
//!
//! Reference: `docs/hubrise/04-catalogs.md` (Inventory section) and
//! `pos-hubrise-mock/docs/04-catalogs.md` for the mock's own copy.
//!
//! An inventory entry names exactly one of `sku_ref` / `option_ref`
//! (required), carries a `stock` decimal string with 0 to 3 decimals
//! (non-negative, `null` = unlimited), and an `expires_at` that is
//! only meaningful when `stock` is `0`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InventoryEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_ref: Option<String>,
    /// Decimal string, 0 to 3 decimals, non-negative; `null` (missing
    /// here) means unlimited stock.
    #[serde(default)]
    pub stock: Option<String>,
    /// ISO 8601; only allowed when `stock` is exactly `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

impl InventoryEntry {
    /// The ref this entry is about, whichever of the two fields is set.
    pub fn target_ref(&self) -> Option<&str> {
        self.sku_ref.as_deref().or(self.option_ref.as_deref())
    }
}
