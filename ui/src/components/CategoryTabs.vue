<script setup lang="ts">
import type { Category } from '../types'
import { useI18n } from '../i18n'
import { FAVOURITES_TAB } from '../utils/favourites'
import { useBilingualName } from '../utils/names'
import UiIcon from './UiIcon.vue'

defineProps<{
  categories: Category[]
  selectedId: number | null
  /** The "Favoris" tab, first before the categories; hidden while it has no product (UX audit C12). */
  showFavourites?: boolean
}>()

const emit = defineEmits<{
  select: [id: number]
}>()

const bilingual = useBilingualName()
const { t } = useI18n()
</script>

<template>
  <!-- UX guide §3: categories always above the menu grid. Two fixed
       rows instead of scrolling tabs, so all 17 categories are one tap
       away (deliberate deviation from §2.1, noted in the design). -->
  <div class="category-tabs">
    <button
      v-if="showFavourites"
      class="tab favourites"
      :class="{ active: selectedId === FAVOURITES_TAB }"
      data-testid="tab-favourites"
      @click="emit('select', FAVOURITES_TAB)"
    >
      <span class="stripe" :style="{ background: 'var(--border)' }" />
      <span class="tab-name"><UiIcon name="star" :size="20" /> {{ t('favourites_tab') }}</span>
    </button>
    <button
      v-for="cat in categories"
      :key="cat.id"
      class="tab"
      :class="{ active: cat.id === selectedId }"
      data-testid="category-tab"
      @click="emit('select', cat.id)"
    >
      <span class="stripe" :style="{ background: cat.color ?? 'var(--border)' }" />
      <span class="tab-name">{{ bilingual(cat).primary }}</span>
      <span v-if="bilingual(cat).secondary" class="tab-sub">{{ bilingual(cat).secondary }}</span>
    </button>
  </div>
</template>

<style scoped>
.category-tabs {
  display: grid;
  grid-template-columns: repeat(9, minmax(0, 1fr));
  gap: 8px;
  padding: 16px 16px 8px;
  flex-shrink: 0;
}

.tab {
  position: relative;
  height: 60px;
  padding: 7px 6px 2px;
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  color: var(--text-primary);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  line-height: 1.2;
  text-align: center;
  transition: transform var(--tap-duration) ease;
}

/* UX guide §4.2: scale(0.95) for 100 ms, no hover */
.tab:active {
  transform: scale(var(--tap-scale));
}

.tab.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}

/* The category colour, as on the menu buttons. */
.stripe {
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  height: 5px;
}

.tab-name {
  max-width: 100%;
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.favourites .tab-name {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}

.tab-sub {
  max-width: 100%;
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tab.active .tab-sub {
  color: var(--selected-fg);
}
</style>
