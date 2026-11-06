package dev.nuagemagique.pilipos.feature.order

import uniffi.pos_ffi.Category

/**
 * The reserved id of the "Favoris" tab (`FAVOURITES_TAB` of `utils/favourites.ts`, UX audit C12). It is not a
 * category: category ids are positive, so every place that holds "the selected category" holds the tab the same way.
 */
internal const val FAVOURITES_TAB: Long = -1L

/** The most products a manager's selection holds (`MAX_SELECTION` of `utils/favourites.ts`, the engine checks it). */
internal const val FAVOURITES_MAX: Int = 30

/** The tab to open when nothing is selected yet: "Favoris" when it has products, else the first category. */
internal fun defaultTab(
    categories: List<Category>,
    favouriteCount: Int,
): Long? = if (favouriteCount > 0) FAVOURITES_TAB else categories.firstOrNull()?.id

/**
 * The tab to keep after the categories and the favourites were read again (`keepTab` of `utils/favourites.ts`):
 * "Favoris" that became empty (the manager changed the setting, the products sold out) falls back to the first
 * category, a missing selection takes the default tab, a category that is gone takes the default tab too.
 */
internal fun keepTab(
    selected: Long?,
    categories: List<Category>,
    favouriteCount: Int,
): Long? =
    when {
        selected == null -> defaultTab(categories, favouriteCount)
        selected == FAVOURITES_TAB -> if (favouriteCount == 0) categories.firstOrNull()?.id else FAVOURITES_TAB
        categories.any { it.id == selected } -> selected
        else -> defaultTab(categories, favouriteCount)
    }
