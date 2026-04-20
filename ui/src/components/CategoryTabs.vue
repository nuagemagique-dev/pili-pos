<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import type { Category } from "../types";

const props = defineProps<{
  categories: Category[];
  selectedId: number | null;
}>();

const emit = defineEmits<{
  select: [id: number];
}>();

const tabsRef = ref<HTMLElement | null>(null);

watch(() => props.selectedId, async () => {
  await nextTick();
  const el = tabsRef.value?.querySelector(".tab.active") as HTMLElement | null;
  el?.scrollIntoView({ behavior: "smooth", block: "nearest", inline: "center" });
});
</script>

<template>
  <!-- UX Guide §3: categories TOUJOURS en haut de la grille menu -->
  <div ref="tabsRef" class="category-tabs">
    <button
      v-for="cat in categories"
      :key="cat.id"
      class="tab"
      :class="{ active: cat.id === selectedId }"
      @click="emit('select', cat.id)"
    >
      {{ cat.name }}
    </button>
  </div>
</template>

<style scoped>
/* UX Guide §4.1: category tab 120×52px recommended */
.category-tabs {
  display: flex;
  gap: var(--space-xs);
  padding: var(--space-sm) var(--space-md);
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
  scrollbar-width: none;
  flex-shrink: 0;
}
.category-tabs::-webkit-scrollbar { display: none; }

.tab {
  padding: var(--space-sm) var(--space-lg);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  white-space: nowrap;
  border-radius: var(--radius-lg);
  background: var(--bg-surface);
  color: var(--text-secondary);
  min-height: 52px;
  flex-shrink: 0;
  transition: transform var(--tap-duration) ease;
}

/* UX Guide §4.2: scale(0.95) pendant 100ms, NO hover */
.tab:active { transform: scale(var(--tap-scale)); }

.tab.active {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
</style>
