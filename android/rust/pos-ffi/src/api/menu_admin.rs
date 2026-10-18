//! Facade of the menu editor commands (engine module `menu_admin`): one method per Tauri
//! command, same parameter names, one engine call each.

use pos_engine::commands::menu_admin;

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::menu_admin::{
    MenuAdmin, MenuChange, MenuSaved, SaveCategoryInput, SaveModifierGroupInput, SaveModifierInput,
    SaveProductInput,
};

// The closures name the engine function at the call, which `check_command_parity.py` looks for.
#[allow(clippy::redundant_closure)]
#[uniffi::export]
impl PosEngine {
    /// The whole menu with its names per language, hidden entries included. Manager only.
    pub async fn menu_admin_get(&self) -> Result<MenuAdmin, PosError> {
        let menu = self
            .blocking(move |st| menu_admin::do_menu_admin_get(st))
            .await?;
        convert::out(&menu)
    }

    /// Create (no id) or update a category.
    pub async fn menu_save_category(
        &self,
        input: SaveCategoryInput,
    ) -> Result<MenuSaved, PosError> {
        let input: menu_admin::SaveCategoryInput = convert::inp(&input)?;
        let saved = self
            .blocking(move |st| menu_admin::do_menu_save_category(st, input))
            .await?;
        convert::out(&saved)
    }

    /// Create (no id) or update a product.
    pub async fn menu_save_product(&self, input: SaveProductInput) -> Result<MenuSaved, PosError> {
        let input: menu_admin::SaveProductInput = convert::inp(&input)?;
        let saved = self
            .blocking(move |st| menu_admin::do_menu_save_product(st, input))
            .await?;
        convert::out(&saved)
    }

    /// Create (no id) or update an option group of a product.
    pub async fn menu_save_modifier_group(
        &self,
        input: SaveModifierGroupInput,
    ) -> Result<MenuSaved, PosError> {
        let input: menu_admin::SaveModifierGroupInput = convert::inp(&input)?;
        let saved = self
            .blocking(move |st| menu_admin::do_menu_save_modifier_group(st, input))
            .await?;
        convert::out(&saved)
    }

    /// Create (no id) or update an option of a group.
    pub async fn menu_save_modifier(
        &self,
        input: SaveModifierInput,
    ) -> Result<MenuSaved, PosError> {
        let input: menu_admin::SaveModifierInput = convert::inp(&input)?;
        let saved = self
            .blocking(move |st| menu_admin::do_menu_save_modifier(st, input))
            .await?;
        convert::out(&saved)
    }

    /// The change log of the menu, newest first.
    pub async fn list_menu_changes(&self, limit: Option<i64>) -> Result<Vec<MenuChange>, PosError> {
        let changes = self
            .blocking(move |st| menu_admin::do_list_menu_changes(st, limit))
            .await?;
        convert::out(&changes)
    }
}
