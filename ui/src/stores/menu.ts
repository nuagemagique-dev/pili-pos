import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Category, MenuItem, SoldOutItem } from '../types'
import type { Availability } from '../utils/soldOut'
import { useToastStore } from './toast'
import { describeError } from '../utils/errorMessages'
import * as ipc from '../api/ipc'
import { usePosStore } from './pos'

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

    if (categories.value.length > 0 && selectedCategoryId.value === null) {
      selectedCategoryId.value = categories.value[0].id
    }
  }

  async function loadItems(categoryId?: number) {
    const pos = usePosStore()
    loading.value = true
    try {
      items.value = await ipc.getMenuItems(categoryId, pos.language)
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
    if (selectedCategoryId.value) {
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
    if (selectedCategoryId.value) await loadItems(selectedCategoryId.value)
    return true
  }

  function getCategoryName(categoryId: number): string {
    return categoryNames.value.get(categoryId) ?? ''
  }

  return {
    categories,
    items,
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
