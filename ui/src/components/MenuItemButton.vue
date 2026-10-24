<script setup lang="ts">
import { computed, ref } from "vue";
import type { MenuItem } from "../types";
import { formatPrice } from "../utils/format";
import { useI18n } from "../i18n";
import { useBilingualName } from "../utils/names";
import AllergenBadges from "./AllergenBadges.vue";

const props = defineProps<{
  item: MenuItem;
  categoryColor?: string;
  /** Quantity of this product already in the cart (badge when > 0). */
  inCart?: number;
}>();

const emit = defineEmits<{
  add: [item: MenuItem];
}>();

const { t } = useI18n();
const bilingual = useBilingualName();
const names = computed(() => bilingual(props.item));
const btnRef = ref<HTMLElement | null>(null);

function handleTap() {
  emit("add", props.item);
  // UX guide §4.2: 200 ms confirmation flash
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
    UX guide §6: three things per button, centred:
    1. the name (UI language first, the other language underneath)
    2. the price (neutral: colour is for actions only)
    3. the category colour (4 px left border)
  -->
  <button
    ref="btnRef"
    class="item-btn"
    :class="{ 'sold-out': item.sold_out }"
    :style="categoryColor ? { borderLeftColor: categoryColor } : undefined"
    @click="handleTap"
  >
    <span class="item-name" :class="{ code: names.isCode }">{{ names.primary }}</span>
    <span v-if="names.secondary" class="item-sub">{{ names.secondary }}</span>
    <span v-if="item.open_price" class="item-open-price">{{ t('open_price_badge') }}</span>
    <span v-else class="item-price">{{ formatPrice(item.price_cents) }}</span>
    <AllergenBadges v-if="item.allergens?.length" class="item-allergens" :codes="item.allergens" />
    <span v-if="inCart" class="item-qty">{{ inCart }}</span>
    <span v-if="item.sold_out" class="item-sold-out" data-testid="soldout-badge">
      {{ t('soldout_badge') }}
    </span>
  </button>
</template>

<style scoped>
.item-btn {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 12px 12px 10px;
  min-height: 120px;
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  color: var(--text-primary);
  text-align: center;
  border-left: 4px solid var(--border);
  transition:
    transform var(--tap-duration) ease,
    background-color var(--flash-duration) ease;
}

/* UX guide §4.2: no hover. scale(0.95) + darker on :active */
.item-btn:active {
  transform: scale(var(--tap-scale));
  background-color: var(--bg-pressed);
}

/* Sold out: greyed, with a badge. A tap says why it cannot be added. */
.item-btn.sold-out { opacity: 0.55; background: var(--bg-surface); }
.item-sold-out {
  position: absolute;
  top: 8px;
  right: 8px;
  padding: 2px 8px;
  border-radius: 6px;
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-size: 13px;
  font-weight: var(--font-weight-bold);
}

.item-name {
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  line-height: 1.3;
  padding: 0 4px;
  text-wrap: balance;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

/* A menu code leads: bigger, in mono like every figure. */
.item-name.code {
  font-family: var(--font-mono);
  font-size: 24px;
}

.item-sub {
  font-size: 13px;
  line-height: 1.3;
  color: var(--text-secondary);
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.item-price {
  margin-top: 2px;
  font-family: var(--font-mono);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.item-open-price {
  margin-top: 2px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 13px;
  font-weight: var(--font-weight-bold);
}

/* The allergens the product contains: bottom left, on the price line,
   clear of the quantity badge. */
.item-allergens {
  position: absolute;
  bottom: 8px;
  left: 8px;
  max-width: calc(100% - 56px);
}

/* Already in the cart: bottom right, on the price line, clear of the
   names. */
.item-qty {
  position: absolute;
  bottom: 8px;
  right: 8px;
  min-width: 30px;
  height: 30px;
  padding: 0 8px;
  border-radius: 15px;
  background: var(--selected-bg);
  color: var(--selected-fg);
  font-family: var(--font-mono);
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
