import { defineStore } from "pinia";
import { ref } from "vue";
import type { Category, MenuItem } from "../types";
import * as ipc from "../api/ipc";
import { usePosStore } from "./pos";

export const useMenuStore = defineStore("menu", () => {
  const categories = ref<Category[]>([]);
  const items = ref<MenuItem[]>([]);
  const selectedCategoryId = ref<number | null>(null);
  const loading = ref(false);

  // Map category_id → category name for display in cart lines
  const categoryNames = ref<Map<number, string>>(new Map());

  async function loadCategories() {
    const pos = usePosStore();
    categories.value = await ipc.getCategories(pos.language);
    // Build lookup map
    const map = new Map<number, string>();
    for (const cat of categories.value) {
      map.set(cat.id, cat.name);
    }
    categoryNames.value = map;

    if (categories.value.length > 0 && selectedCategoryId.value === null) {
      selectedCategoryId.value = categories.value[0].id;
    }
  }

  async function loadItems(categoryId?: number) {
    const pos = usePosStore();
    loading.value = true;
    try {
      items.value = await ipc.getMenuItems(categoryId, pos.language);
    } finally {
      loading.value = false;
    }
  }

  async function selectCategory(categoryId: number) {
    selectedCategoryId.value = categoryId;
    await loadItems(categoryId);
  }

  async function reloadWithLanguage() {
    await loadCategories();
    if (selectedCategoryId.value) {
      await loadItems(selectedCategoryId.value);
    }
  }

  function getCategoryName(categoryId: number): string {
    return categoryNames.value.get(categoryId) ?? "";
  }

  return {
    categories, items, selectedCategoryId, loading, categoryNames,
    loadCategories, loadItems, selectCategory, reloadWithLanguage, getCategoryName,
  };
});
