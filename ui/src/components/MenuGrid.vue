<script setup lang="ts">
import type { MenuItem } from "../types";
import { useI18n } from "../i18n";
import MenuItemButton from "./MenuItemButton.vue";

const { t } = useI18n();

const props = defineProps<{
  items: MenuItem[];
  categoryColor?: string | null;
}>();

const emit = defineEmits<{
  addItem: [item: MenuItem];
}>();
</script>

<template>
  <!-- UX Guide §3: 4 colonnes, scrollable vertical, inertie tactile -->
  <div class="menu-grid scroll-touch">
    <MenuItemButton
      v-for="item in items"
      :key="item.id"
      :item="item"
      :category-color="props.categoryColor ?? undefined"
      @add="emit('addItem', $event)"
    />
    <div v-if="items.length === 0" class="empty">
      {{ t('no_items') }}
    </div>
  </div>
</template>

<style scoped>
/* UX Guide §3: 4 colonnes, gap 8px, minimum 140px per cell */
.menu-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--space-sm);
  padding: var(--space-md);
  padding-bottom: var(--space-md);
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
  overscroll-behavior-y: contain;
  flex: 1;
  align-content: start;
}

.empty {
  grid-column: 1 / -1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  font-size: var(--font-size-lg);
  padding: var(--space-xl);
}
</style>
