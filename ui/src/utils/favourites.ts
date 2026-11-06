// The "Favoris" tab of the order screen (UX audit C12). It is not a category: the menu store gives
// it this reserved id, which no category can have (category ids are positive), so every place that
// holds "the selected category" holds the tab the same way.
import type { Category } from '../types'

/** The reserved id of the "Favoris" tab. */
export const FAVOURITES_TAB = -1

/** The tab to open when nothing is selected yet: "Favoris" when it has products, else the first category. */
export function defaultTab(categories: Category[], favouriteCount: number): number | null {
  if (favouriteCount > 0) return FAVOURITES_TAB
  return categories[0]?.id ?? null
}

/**
 * The tab to keep after the categories and the favourites were read again: "Favoris" that became
 * empty (the manager changed the setting, the products sold out) falls back to the first category,
 * a missing selection takes the default tab.
 */
export function keepTab(
  selected: number | null,
  categories: Category[],
  favouriteCount: number,
): number | null {
  if (selected === null) return defaultTab(categories, favouriteCount)
  if (selected === FAVOURITES_TAB && favouriteCount === 0) return categories[0]?.id ?? null
  return selected
}

/** The most products a manager's selection holds (the engine refuses more). */
export const MAX_SELECTION = 30
