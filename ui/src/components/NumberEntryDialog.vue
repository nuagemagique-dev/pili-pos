<script setup lang="ts">
// Whole-number entry on the shared NumPad: a Telegram chat id (optional
// minus sign, up to 20 digits) or a small setting (hours, minutes).
// The value is kept as text so a long chat id never loses a digit.
import { computed, ref } from "vue";
import NumPad from "./NumPad.vue";
import { useI18n } from "../i18n";

const props = defineProps<{
  title: string;
  /** The value being edited ("-123" or "45"). */
  initial: string;
  maxDigits: number;
  /** Shows the minus toggle (chat ids of groups are negative). */
  allowMinus?: boolean;
  /** Smallest and largest accepted number (checked when no minus sign is allowed). */
  min?: number;
  max?: number;
}>();

const emit = defineEmits<{
  confirm: [value: string];
  cancel: [];
}>();

const { t } = useI18n();

const minus = ref(props.allowMinus === true && props.initial.startsWith("-"));
const digits = ref(props.initial.replace(/^-/, ""));

function onKey(key: string) {
  if (key === "BACKSPACE") digits.value = digits.value.slice(0, -1);
  else if (key === "CLEAR") digits.value = "";
  else if (digits.value.length < props.maxDigits) digits.value += key;
}

const text = computed(() => (digits.value === "" ? "" : (minus.value ? "-" : "") + digits.value));
const valid = computed(() => {
  if (digits.value === "") return false;
  if (props.allowMinus) return true;
  const n = Number(digits.value);
  return n >= (props.min ?? 0) && n <= (props.max ?? Number.MAX_SAFE_INTEGER);
});

function confirm() {
  if (valid.value) emit("confirm", props.allowMinus ? text.value : String(Number(digits.value)));
}
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div class="dialog" role="dialog" :aria-label="title">
      <h2 class="title">{{ title }}</h2>
      <p class="amount" data-testid="number-entry-value">{{ text || "0" }}</p>
      <button
        v-if="allowMinus"
        class="minus"
        :class="{ on: minus }"
        data-testid="number-entry-minus"
        @click="minus = !minus"
      >{{ t('alerts_minus') }}</button>
      <NumPad :model-value="digits" @update:model-value="onKey" />
      <div class="actions">
        <button class="btn btn-cancel" @click="emit('cancel')">{{ t('back') }}</button>
        <button class="btn btn-confirm" :disabled="!valid" data-testid="number-entry-ok" @click="confirm">
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
.amount {
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  color: var(--accent-primary);
  font-variant-numeric: tabular-nums;
  font-family: var(--font-mono);
  min-height: 1.2em;
  max-width: 320px;
  overflow-wrap: anywhere;
}
.minus {
  height: 48px; padding: 0 16px; border-radius: var(--radius-sm);
  background: var(--bg-surface); color: var(--text-primary);
  font-size: 16px; font-weight: var(--font-weight-bold);
}
.minus.on { background: var(--selected-bg); color: var(--selected-fg); }
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
