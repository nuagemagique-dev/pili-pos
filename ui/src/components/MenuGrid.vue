<script setup lang="ts">
import type { MenuItem } from '../types'
import { useI18n } from '../i18n'
import MenuItemButton from './MenuItemButton.vue'

const { t } = useI18n()

const {
  items,
  categoryColor = undefined,
  cartQuantities = undefined,
} = defineProps<{
  items: MenuItem[]
  categoryColor?: string | null
  /** Quantity already in the cart, per product id. */
  cartQuantities?: Map<number, number>
}>()

const emit = defineEmits<{
  addItem: [item: MenuItem]
}>()
</script>

<template>
  <!-- 5 columns (buttons ~230 × 120 px, above the §4.1 minimum of
       140 × 100; deliberate deviation from §3's 4 columns), vertical
       scroll with touch inertia. -->
  <div class="menu-grid scroll-touch">
    <MenuItemButton
      v-for="item in items"
      :key="item.id"
      :item="item"
      :category-color="categoryColor ?? undefined"
      :in-cart="cartQuantities?.get(item.id) ?? 0"
      @add="emit('addItem', $event)"
    />
    <div v-if="items.length === 0" class="empty">
      {{ t('no_items') }}
    </div>
  </div>
</template>

<style scoped>
.menu-grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 8px;
  padding: 16px;
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
