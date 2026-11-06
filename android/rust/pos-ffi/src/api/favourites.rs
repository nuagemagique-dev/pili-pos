//! The favourites facade (engine module `favourites`): four commands as async methods of
//! [`PosEngine`].
//!
//! Each method mirrors its Tauri wrapper (`src-tauri/src/ipc/favourites.rs`): the same parameter
//! names and the single engine call. Local data, never fiscal. Any logged-in operator reads; the
//! setting and the star toggle are for managers.

use pos_engine::commands::favourites;

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::favourites::FavouritesSettings;
use crate::records::orders::MenuItem;

#[uniffi::export]
impl PosEngine {
    /// The favourites setting: the mode and the manager's selection.
    #[expect(
        clippy::redundant_closure,
        reason = "check_command_parity.py wants the engine call spelled out"
    )]
    pub async fn get_favourites_settings(&self) -> Result<FavouritesSettings, PosError> {
        let settings = self
            .blocking(|st| favourites::do_get_favourites_settings(st))
            .await?;
        convert::out(&settings)
    }

    /// Store the favourites setting; managers only.
    pub async fn set_favourites_settings(
        &self,
        input: FavouritesSettings,
    ) -> Result<FavouritesSettings, PosError> {
        let input: favourites::FavouritesSettings = convert::inp(&input)?;
        let stored = self
            .blocking(move |st| favourites::do_set_favourites_settings(st, input))
            .await?;
        convert::out(&stored)
    }

    /// Star or unstar a product; managers only.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "UniFFI export: the argument is part of the Kotlin API"
    )]
    pub async fn set_favourite(
        &self,
        menu_item_id: i64,
        favourite: bool,
    ) -> Result<FavouritesSettings, PosError> {
        let stored = self
            .blocking(move |st| favourites::do_set_favourite(st, menu_item_id, favourite))
            .await?;
        convert::out(&stored)
    }

    /// The products of the "Favoris" tab, names in `lang`; empty hides the tab.
    pub async fn list_favourites(&self, lang: Option<String>) -> Result<Vec<MenuItem>, PosError> {
        let items = self
            .blocking(move |st| favourites::do_list_favourites(st, lang))
            .await?;
        convert::out(&items)
    }
}
