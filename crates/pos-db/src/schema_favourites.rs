//! Schema of the favourites cache (migration 92, UX audit C12). Part of the
//! certified binary on every installation.
//!
//! Local data, never fiscal: the cache only remembers which products sold
//! best over the last 30 days so the "Favoris" tab of the order screen does
//! not read the whole archive at every tap. Nothing here reaches an FDM
//! message, a signed ticket, an archived event or an export.

/// v92 - the cached ranking behind "Plus vendus sur 30 jours".
///
/// - `favourites_top_sold`: the ranking, one row per product, best first
///   (`rank` 1 is the best seller); `units` is the net quantity sold in the
///   window in thousandths of a unit (refunds count negative, a weighed line
///   counts as one unit).
/// - `favourites_cache_meta`: one row, when the ranking was computed and the
///   id of the last Z report at that time. The ranking is recomputed when it
///   is an hour old or when a Z was made since.
pub(crate) const MIGRATION_092_FAVOURITES_CACHE: &str = r"
CREATE TABLE IF NOT EXISTS favourites_top_sold (
    rank INTEGER PRIMARY KEY,
    menu_item_id INTEGER NOT NULL,
    units INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS favourites_cache_meta (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    computed_at TEXT NOT NULL,
    last_z_id INTEGER NOT NULL
);
";
