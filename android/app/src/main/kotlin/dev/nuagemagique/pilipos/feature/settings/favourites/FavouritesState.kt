package dev.nuagemagique.pilipos.feature.settings.favourites

import uniffi.pos_ffi.FavouritesMode

/** Everything the manager can do in the favourites panel. */
internal data class FavouritesCallbacks(
    val onMode: (FavouritesMode) -> Unit = {},
    val onRemove: (productId: Long) -> Unit = {},
)

/**
 * What the favourites panel shows (`FavouritesSettings.vue`). [settings] is what the engine holds (null until read);
 * [names] gives the product names to show for the selection (a product that left the menu shows `#id`).
 */
internal data class FavouritesUiState(
    val manager: Boolean = false,
    val mode: FavouritesMode? = null,
    val selection: List<Long> = emptyList(),
    val names: Map<Long, String> = emptyMap(),
    val busy: Boolean = false,
    val error: String? = null,
) {
    /** True once the engine's setting was read. */
    val loaded: Boolean get() = mode != null

    /** The name to show for a selected product. */
    fun nameOf(id: Long): String = names[id] ?: "#$id"
}
