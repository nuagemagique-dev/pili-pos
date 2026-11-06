//! Records of the favourites tab (engine module `favourites`, UX audit C12).
//! Mirror the engine types (PD56): same field names, same serde form, `deny_unknown_fields`.

/// `pos_engine::commands::favourites::FavouritesMode`: where the tab takes its products from.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum FavouritesMode {
    /// The best sellers of the last 30 days.
    TopSold,
    /// The manager's own list.
    Selection,
}

/// `pos_engine::commands::favourites::FavouritesSettings`: the mode and the manager's selection
/// (product ids in the order they were added).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct FavouritesSettings {
    pub mode: FavouritesMode,
    pub selection: Vec<i64>,
}
