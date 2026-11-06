import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Category, MenuItem, SoldOutItem } from '../types'
import type { Availability } from '../utils/soldOut'
import { useToastStore } from './toast'
import { describeError } from '../utils/errorMessages'
import * as ipc from '../api/ipc'
import { usePosStore } from './pos'
import { FAVOURITES_TAB, keepTab } from '../utils/favourites'

export const useMenuStore = defineStore('menu', () => {
  const categories = ref<Category[]>([])
  const items = ref<MenuItem[]>([])
  const selectedCategoryId = ref<number | null>(null)
  const loading = ref(false)

  // Map category_id → category name for display in cart lines
  const categoryNames = ref<Map<number, string>>(new Map())

  async function loadCategories() {
    const pos = usePosStore()
    categories.value = await ipc.getCategories(pos.language)
    // Build lookup map
    const map = new Map<number, string>()
    for (const cat of categories.value) {
      map.set(cat.id, cat.name)
    }
    categoryNames.value = map

    await refreshFavourites()
    selectedCategoryId.value = keepTab(
      selectedCategoryId.value,
      categories.value,
      favourites.value.length,
    )
  }

  // The products of the "Favoris" tab (UX audit C12); the tab is hidden while there are none.
  const favourites = ref<MenuItem[]>([])

  /** Read the tab's products again. A failure hides the tab: it is a shortcut, never a blocker. */
  async function refreshFavourites() {
    const pos = usePosStore()
    try {
      favourites.value = await ipc.listFavourites(pos.language)
    } catch {
      favourites.value = []
    }
  }

  async function loadItems(categoryId?: number) {
    const pos = usePosStore()
    loading.value = true
    try {
      if (categoryId === FAVOURITES_TAB) {
        await refreshFavourites()
        const [first] = categories.value
        if (favourites.value.length === 0 && first !== undefined) {
          // The tab emptied (the setting changed, the products sold out): back to the first category.
          selectedCategoryId.value = first.id
          items.value = await ipc.getMenuItems(first.id, pos.language)
        } else {
          items.value = favourites.value
        }
      } else {
        items.value = await ipc.getMenuItems(categoryId, pos.language)
      }
    } finally {
      loading.value = false
    }
  }

  async function selectCategory(categoryId: number) {
    selectedCategoryId.value = categoryId
    await loadItems(categoryId)
  }

  async function reloadWithLanguage() {
    await loadCategories()
    if (selectedCategoryId.value !== null) {
      await loadItems(selectedCategoryId.value)
    }
  }

  // The products sold out now ("Épuisé"); read by cashiers and managers only.
  const soldOut = ref<SoldOutItem[]>([])

  async function loadSoldOut() {
    const pos = usePosStore()
    try {
      soldOut.value = await ipc.listSoldOut(pos.language)
    } catch (e) {
      useToastStore().show(describeError(e), 'error', 5000)
    }
  }

  /** Change a product's availability and refresh the grid. False when the till refused. */
  async function setAvailability(menuItemId: number, choice: Availability): Promise<boolean> {
    const pos = usePosStore()
    try {
      soldOut.value =
        choice === 'available'
          ? await ipc.clearSoldOut(menuItemId, pos.language)
          : await ipc.setSoldOut(menuItemId, choice === 'day', pos.language)
    } catch (e) {
      useToastStore().show(describeError(e), 'error', 5000)
      return false
    }
    await refreshFavourites()
    if (selectedCategoryId.value !== null) await loadItems(selectedCategoryId.value)
    return true
  }

  function getCategoryName(categoryId: number): string {
    return categoryNames.value.get(categoryId) ?? ''
  }

  return {
    categories,
    items,
    favourites,
    refreshFavourites,
    selectedCategoryId,
    loading,
    categoryNames,
    soldOut,
    loadSoldOut,
    setAvailability,
    loadCategories,
    loadItems,
    selectCategory,
    reloadWithLanguage,
    getCategoryName,
  }
})
