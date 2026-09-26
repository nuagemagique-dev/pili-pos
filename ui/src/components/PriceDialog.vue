<script setup lang="ts">
// Amount entry: the price of a free-price product ("Prix libre"), or
// the cash counted in the drawer. Digits are typed as cents, like a
// payment terminal: 2 0 0 0 → 20,00 €.
import { computed, ref } from "vue";
import NumPad from "./NumPad.vue";
import { formatPrice } from "../utils/format";
import { useI18n } from "../i18n";

const props = defineProps<{
  itemName: string;
  /** Defaults to the free-price title. */
  title?: string;
  /** An empty drawer is counted as 0,00 €; a price never is. */
  allowZero?: boolean;
}>();

const emit = defineEmits<{
  confirm: [priceCents: number];
  cancel: [];
}>();

const { t } = useI18n();

// Up to 9 999,99 € - far above any menu price, and keeps the display short.
const MAX_DIGITS = 6;
const digits = ref("");
const cents = computed(() => Number(digits.value || "0"));

function onKey(key: string) {
  if (key === "BACKSPACE") digits.value = digits.value.slice(0, -1);
  else if (key === "CLEAR") digits.value = "";
  else if (digits.value.length < MAX_DIGITS && !(digits.value === "" && key === "0")) {
    digits.value += key;
  }
}

const valid = computed(() => cents.value > 0 || (props.allowZero === true && cents.value === 0));

function confirm() {
  if (valid.value) emit("confirm", cents.value);
}
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div class="dialog" role="dialog" :aria-label="title ?? t('open_price_title')">
      <h2 class="title">{{ title ?? t('open_price_title') }}</h2>
      <p class="item">{{ itemName }}</p>
      <p class="amount" data-testid="open-price-amount">{{ formatPrice(cents) }} €</p>
      <NumPad :model-value="digits" @update:model-value="onKey" />
      <div class="actions">
        <button class="btn btn-cancel" @click="emit('cancel')">{{ t('back') }}</button>
        <button class="btn btn-confirm" :disabled="!valid" @click="confirm">
          {{ t('validate') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  z-index: 50;
}
.dialog {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); }
.item { font-size: var(--font-size-md); color: var(--text-secondary); }
.amount {
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  color: var(--accent-primary);
  font-variant-numeric: tabular-nums;
}
.actions { display: flex; gap: var(--space-sm); width: 100%; }
.btn {
  flex: 1;
  height: 56px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  transition: transform var(--tap-duration) ease;
}
.btn:active { transform: scale(var(--tap-scale)); }
.btn-cancel { background: var(--bg-surface); color: var(--text-secondary); }
.btn-confirm { background: var(--accent-primary); color: var(--text-on-accent); }
.btn-confirm:disabled { opacity: 0.4; }
</style>
