<script setup lang="ts">
// Price entry for free-price products ("Prix libre"). Digits are typed
// as cents, like a payment terminal: 2 0 0 0 → 20,00 €.
import { computed, ref } from "vue";
import NumPad from "./NumPad.vue";
import { formatPrice } from "../utils/format";
import { useI18n } from "../i18n";

defineProps<{
  itemName: string;
}>();

const emit = defineEmits<{
  confirm: [priceCents: number];
  cancel: [];
}>();

const { t } = useI18n();

// Up to 9 999,99 € — far above any menu price, and keeps the display short.
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

function confirm() {
  if (cents.value > 0) emit("confirm", cents.value);
}
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div class="dialog" role="dialog" :aria-label="t('open_price_title')">
      <h2 class="title">{{ t('open_price_title') }}</h2>
      <p class="item">{{ itemName }}</p>
      <p class="amount" data-testid="open-price-amount">{{ formatPrice(cents) }} €</p>
      <NumPad :model-value="digits" @update:model-value="onKey" />
      <div class="actions">
        <button class="btn btn-cancel" @click="emit('cancel')">{{ t('back') }}</button>
        <button class="btn btn-confirm" :disabled="cents <= 0" @click="confirm">
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
