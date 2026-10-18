//! The allergens facade (engine module allergens): four commands as async methods of
//! [`PosEngine`].
//!
//! Each method mirrors its Tauri wrapper (`src-tauri/src/ipc/allergens.rs`): the same parameter
//! names and the single engine call. Reading and printing are open to any logged-in operator;
//! `set_allergens` is for managers and refused while a catalog source manages the menu.

use pos_engine::commands::allergens;

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::allergens::{AllergenEntry, AllergenSheetView, SetAllergensInput};
use crate::records::common::SlipOutcome;

#[uniffi::export]
impl PosEngine {
    /// The 14 EU allergens in Annex II order, for the legend.
    #[allow(clippy::redundant_closure)] // check_command_parity.py wants the engine call spelled out
    pub async fn list_allergens(&self) -> Result<Vec<AllergenEntry>, PosError> {
        let entries = self.blocking(|st| allergens::do_list_allergens(st)).await?;
        convert::out(&entries)
    }

    /// The allergen sheet of one product, or of the whole active menu.
    pub async fn get_allergen_sheet(
        &self,
        menu_item_id: Option<i64>,
        lang: Option<String>,
    ) -> Result<AllergenSheetView, PosError> {
        let sheet = self
            .blocking(move |st| allergens::do_get_allergen_sheet(st, menu_item_id, lang))
            .await?;
        convert::out(&sheet)
    }

    /// Print the allergen sheet (one product or the whole menu).
    pub async fn print_allergen_sheet(
        &self,
        menu_item_id: Option<i64>,
        lang: Option<String>,
    ) -> Result<SlipOutcome, PosError> {
        let outcome = self
            .spawn(move |e| async move {
                allergens::do_print_allergen_sheet(&e, menu_item_id, lang).await
            })
            .await?;
        convert::out(&outcome)
    }

    /// Set the allergens of a product or an option by hand; managers only, refused while a
    /// catalog source manages the menu.
    pub async fn set_allergens(&self, input: SetAllergensInput) -> Result<bool, PosError> {
        let input: allergens::SetAllergensInput = convert::inp(&input)?;
        self.blocking(move |st| allergens::do_set_allergens(st, input))
            .await
    }
}
