<script setup lang="ts">
import { ref } from "vue";
import type { OrderLine } from "../types";
import { formatPrice } from "../utils/format";

const props = defineProps<{
  line: OrderLine;
  categoryName?: string;
}>();

const emit = defineEmits<{
  increment: [id: number];
  decrement: [id: number];
  remove: [id: number];
}>();

const lineRef = ref<HTMLElement | null>(null);
const justAdded = ref(false);

// Called externally to trigger green flash on new line
defineExpose({
  flashAdded() {
    const el = lineRef.value;
    if (el) {
      el.classList.remove("flash-added");
      void el.offsetWidth;
      el.classList.add("flash-added");
    }
  },
});
</script>

<template>
  <!-- UX Guide §4.1: line height ≥56px (recommended 64px) -->
  <div ref="lineRef" class="order-line">
    <div class="line-top">
      <div class="line-info">
        <span class="line-name">{{ line.name }}</span>
        <span v-if="categoryName" class="line-cat">{{ categoryName }}</span>
      </div>
      <span class="line-price">{{ formatPrice(line.unit_price_cents * line.quantity) }}</span>
    </div>
    <div class="line-bottom">
      <div class="qty-controls">
        <button class="qty-btn" @click="emit('decrement', props.line.id)">-</button>
        <span class="qty">{{ line.quantity }}</span>
        <button class="qty-btn" @click="emit('increment', props.line.id)">+</button>
      </div>
      <button class="remove-btn" @click="emit('remove', props.line.id)">
        <svg width="14" height="14" viewBox="0 0 14 14"><path d="M2 2l10 10M12 2L2 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
.order-line {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  padding: var(--space-sm) 0;
  min-height: 56px;
  border-bottom: 1px solid rgba(160, 160, 176, 0.1);
  transition: box-shadow var(--flash-duration) ease;
}

.line-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-sm);
}

.line-info {
  flex: 1;
  min-width: 0;
}

.line-name {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-normal);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: block;
}

.line-cat {
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
  display: block;
}

.line-price {
  font-weight: var(--font-weight-bold);
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;
}

.line-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.qty-controls {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  background: var(--bg-surface);
  border-radius: var(--radius-md);
  padding: 2px;
}

/* UX Guide §4.1: qty buttons ≥ 48×48 — bumped from 48×40 so
   fingers can hit them reliably under rush-hour stress. */
.qty-btn {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-primary);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.qty-btn:active {
  transform: scale(var(--tap-scale));
  background: var(--bg-pressed);
}

.qty {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  min-width: 32px;
  text-align: center;
  font-variant-numeric: tabular-nums;
}

/* UX Guide §4.1: destructive buttons get at least 48×48 so they
   can't be mashed by accident. */
.remove-btn {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform var(--tap-duration) ease;
}
.remove-btn:active {
  color: var(--accent-danger);
  transform: scale(var(--tap-scale));
}
</style>
