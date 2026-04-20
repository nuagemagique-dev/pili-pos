<script setup lang="ts">
import { ref } from "vue";
import type { MenuItem } from "../types";
import { formatPrice } from "../utils/format";

const props = defineProps<{
  item: MenuItem;
  categoryColor?: string;
}>();

const emit = defineEmits<{
  add: [item: MenuItem];
}>();

const btnRef = ref<HTMLElement | null>(null);

function handleTap() {
  emit("add", props.item);
  // UX Guide §4.2: flash couleur accent 200ms
  const el = btnRef.value;
  if (el) {
    el.classList.remove("flash-added");
    void el.offsetWidth;
    el.classList.add("flash-added");
  }
}
</script>

<template>
  <!--
    UX Guide §6: 3 infos par bouton
    1. Nom produit (ZH principal 16px bold, FR secondaire 12px)
    2. Prix (18px, accent)
    3. Couleur catégorie (bordure gauche 4px)
  -->
  <button
    ref="btnRef"
    class="item-btn"
    :style="categoryColor ? `border-left: 4px solid ${categoryColor}` : ''"
    @click="handleTap"
  >
    <span class="item-name">{{ item.name }}</span>
    <span class="item-price">{{ formatPrice(item.price_cents) }}</span>
  </button>
</template>

<style scoped>
/* UX Guide §4.1: minimum 140×100px, recommended */
.item-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-xs);
  padding: var(--space-sm) var(--space-sm);
  min-height: 100px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  text-align: center;
  border: none;
  border-left: 4px solid var(--bg-surface);
  transition:
    transform var(--tap-duration) ease,
    background-color var(--flash-duration) ease;
}

/* UX Guide §4.2: NO hover. scale(0.95) + bg change on :active */
.item-btn:active {
  transform: scale(var(--tap-scale));
  background-color: var(--bg-pressed);
}

/* UX Guide §10: nom ZH 16px bold */
.item-name {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  line-height: 1.25;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

/* UX Guide §6: prix 18px accent */
.item-price {
  font-size: var(--font-size-price);
  font-weight: var(--font-weight-bold);
  color: var(--accent-primary);
}
</style>
