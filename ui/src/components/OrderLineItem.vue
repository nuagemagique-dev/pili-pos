<script setup lang="ts">
import { computed, ref } from "vue";
import type { CartLine } from "../types";
import { formatPrice } from "../utils/format";
import { useBilingualName } from "../utils/names";
import { useI18n } from "../i18n";

const props = defineProps<{
  line: CartLine;
  /** Not yet sent to the bar / kitchen: amber mark on the left. */
  pending?: boolean;
}>();

const emit = defineEmits<{
  increment: [id: number];
  decrement: [id: number];
  remove: [id: number];
}>();

const { t } = useI18n();
const bilingual = useBilingualName();
const names = computed(() => bilingual(props.line));

/** Notes on the line ("Sans sésame"), when the line carries any:
 * `modifiers_json` holds a string or a list of strings. */
const notes = computed(() => {
  const raw = props.line.modifiers_json;
  if (!raw) return "";
  try {
    const value: unknown = JSON.parse(raw);
    if (Array.isArray(value)) return value.filter((v) => typeof v === "string").join(" · ");
    return typeof value === "string" ? value : "";
  } catch {
    return raw;
  }
});

const lineRef = ref<HTMLElement | null>(null);

// Called externally to trigger the green flash on a new line.
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
  <!-- One 64 px row: quantity, names, price, delete (UX guide §4.1). -->
  <div ref="lineRef" class="order-line" :class="{ pending }">
    <div class="qty-controls">
      <!-- §4.3: "−" stops at 1; removing a line is its own button. -->
      <button
        class="qty-btn"
        :disabled="line.quantity <= 1"
        :aria-label="t('qty_less')"
        @click="emit('decrement', props.line.id)"
      >−</button>
      <span class="qty">{{ line.quantity }}</span>
      <button class="qty-btn" :aria-label="t('qty_more')" @click="emit('increment', props.line.id)">+</button>
    </div>
    <div class="line-info">
      <span class="line-name" :class="{ code: names.isCode }">{{ names.primary }}</span>
      <span v-if="names.secondary" class="line-sub">{{ names.secondary }}</span>
      <span v-if="notes" class="line-notes">{{ notes }}</span>
    </div>
    <span class="line-price">{{ formatPrice(line.unit_price_cents * line.quantity) }}</span>
    <button class="remove-btn" :aria-label="t('line_remove')" @click="emit('remove', props.line.id)">
      <svg width="14" height="14" viewBox="0 0 14 14"><path d="M2 2l10 10M12 2L2 12" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" /></svg>
    </button>
  </div>
</template>

<style scoped>
.order-line {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto auto;
  align-items: center;
  gap: 16px;
  min-height: 64px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
  transition: box-shadow var(--flash-duration) ease;
}
.order-line.pending {
  padding-left: 12px;
  border-left: 3px solid var(--accent-warning);
}

.qty-controls {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 2px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
}

/* UX guide §4.1: 48×48 so fingers hit them under rush-hour stress. */
.qty-btn {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-primary);
  font-size: 24px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.qty-btn:active {
  transform: scale(var(--tap-scale));
  background: var(--bg-pressed);
}
.qty-btn:disabled {
  color: var(--border);
  pointer-events: none;
}

.qty {
  min-width: 28px;
  text-align: center;
  font-family: var(--font-mono);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  font-variant-numeric: tabular-nums;
}

.line-info {
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.line-name,
.line-sub {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.3;
}
.line-name {
  font-size: 17px;
  font-weight: var(--font-weight-bold);
}
.line-name.code {
  font-family: var(--font-mono);
}
.line-sub {
  font-size: 13px;
  color: var(--text-secondary);
}
.line-notes {
  font-size: 13px;
  color: var(--accent-warning);
}

.line-price {
  font-family: var(--font-mono);
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  font-variant-numeric: tabular-nums;
}

/* Destructive: set apart by a rule, never where "−" was (§4.3). */
.remove-btn {
  width: 48px;
  height: 48px;
  margin-left: 8px;
  border-left: 1px solid var(--border);
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
