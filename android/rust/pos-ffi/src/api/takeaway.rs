//! The numbered takeaway facade (engine module `takeaway`): four commands as async methods of
//! [`PosEngine`].
//!
//! Each method mirrors its Tauri wrapper (`src-tauri/src/ipc/takeaway.rs`): the same parameter
//! names and the single engine call. Local data, never fiscal. Any logged-in operator reads the
//! numbering and sets the customer of a takeaway; the first number is for managers.

use pos_engine::commands::takeaway;

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::takeaway::{TakeawayNumbering, TakeawayTag};

#[uniffi::export]
impl PosEngine {
    /// The first takeaway number of each day.
    #[expect(
        clippy::redundant_closure,
        reason = "check_command_parity.py wants the engine call spelled out"
    )]
    pub async fn get_takeaway_numbering(&self) -> Result<TakeawayNumbering, PosError> {
        let numbering = self
            .blocking(|st| takeaway::do_get_takeaway_numbering(st))
            .await?;
        convert::out(&numbering)
    }

    /// Store the first takeaway number; managers only.
    pub async fn set_takeaway_numbering(
        &self,
        input: TakeawayNumbering,
    ) -> Result<TakeawayNumbering, PosError> {
        let input: takeaway::TakeawayNumbering = convert::inp(&input)?;
        let stored = self
            .blocking(move |st| takeaway::do_set_takeaway_numbering(st, input))
            .await?;
        convert::out(&stored)
    }

    /// The number of the day and the customer of a takeaway; `None` for an order without a tag.
    pub async fn get_takeaway_tag(&self, order_id: i64) -> Result<Option<TakeawayTag>, PosError> {
        let tag = self
            .blocking(move |st| takeaway::do_get_takeaway_tag(st, order_id))
            .await?;
        convert::out(&tag)
    }

    /// Set (or clear) the name and the phone of the customer of an open takeaway.
    pub async fn set_takeaway_customer(
        &self,
        order_id: i64,
        name: Option<String>,
        phone: Option<String>,
    ) -> Result<TakeawayTag, PosError> {
        let tag = self
            .blocking(move |st| takeaway::do_set_takeaway_customer(st, order_id, name, phone))
            .await?;
        convert::out(&tag)
    }
}
