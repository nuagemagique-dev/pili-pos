<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "../i18n";

// On-screen keyboard for typing a short text - the kiosk has no
// physical keyboard. Used for a customer's email address (EmailKeypad
// wraps it with the email rules and the confirm step) and for the
// optional reason of a till discount. Class names are shared with the
// email keypad so both look and test the same.

const props = withDefaults(
  defineProps<{
    title: string;
    placeholder?: string;
    /** Text the keypad opens with. */
    initial?: string;
    maxLen?: number;
    /** Shortcut chunks above the keys (".com"). */
    quickKeys?: string[];
    /** Extra symbol keys after the digits. */
    symbols?: string[];
    /** A space bar, for free text. */
    space?: boolean;
    /** Whether the buffer may be confirmed; default: any text, empty included. */
    valid?: (value: string) => boolean;
    /** Shown under the display while a non-empty buffer is not valid. */
    invalidText?: string;
    /** Second screen that shows the text large before it is used. */
    confirmStep?: boolean;
    confirmTitle?: string;
    editLabel?: string;
    confirmLabel?: string;
  }>(),
  {
    placeholder: "",
    initial: "",
    maxLen: 60,
    quickKeys: () => [],
    symbols: () => [],
    space: false,
    valid: () => true,
    invalidText: "",
    confirmStep: false,
    confirmTitle: "",
    editLabel: "",
    confirmLabel: "",
  },
);

const emit = defineEmits<{
  confirm: [text: string];
  cancel: [];
}>();

const { t } = useI18n();

const buffer = ref(props.initial.slice(0, props.maxLen));
const step = ref<"type" | "confirm">("type");

const LETTER_ROWS = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
const DIGIT_ROW = "1234567890".split("");

function press(ch: string) {
  if (buffer.value.length >= props.maxLen) return;
  buffer.value += ch;
}

function pressQuick(chunk: string) {
  if (buffer.value.length + chunk.length > props.maxLen) return;
  buffer.value += chunk;
}

function backspace() {
  buffer.value = buffer.value.slice(0, -1);
}

function clear() {
  buffer.value = "";
}

const isValid = computed(() => props.valid(buffer.value));

function goConfirm() {
  if (!isValid.value) return;
  if (props.confirmStep) step.value = "confirm";
  else emit("confirm", buffer.value);
}

function goBackToType() {
  step.value = "type";
}

function confirmText() {
  emit("confirm", buffer.value);
}

function cancel() {
  emit("cancel");
}
</script>

<template>
  <div class="keypad-overlay" @click.self="cancel">
    <div class="keypad-modal">
      <template v-if="step === 'type'">
        <h3>{{ title }}</h3>

        <div class="email-display" :class="{ invalid: buffer.length > 0 && !isValid }" data-testid="keypad-display">
          <span v-if="buffer">{{ buffer }}</span>
          <span v-else class="email-placeholder">{{ placeholder }}</span>
        </div>
        <p v-if="buffer.length > 0 && !isValid && invalidText" class="email-error">{{ invalidText }}</p>

        <div v-if="quickKeys.length" class="quick-keys">
          <button v-for="q in quickKeys" :key="q" class="quick-key" @click="pressQuick(q)">{{ q }}</button>
        </div>

        <div class="key-rows">
          <div v-for="(row, i) in LETTER_ROWS" :key="i" class="key-row">
            <button v-for="ch in row" :key="ch" class="key" @click="press(ch)">{{ ch }}</button>
          </div>
          <div class="key-row">
            <button v-for="d in DIGIT_ROW" :key="d" class="key" @click="press(d)">{{ d }}</button>
          </div>
          <div class="key-row">
            <button v-for="s in symbols" :key="s" class="key key-symbol" @click="press(s)">{{ s }}</button>
            <button v-if="space" class="key key-space" :aria-label="t('keypad_space')" @click="press(' ')">&#9251;</button>
            <button class="key key-backspace" @click="backspace" :aria-label="editLabel || t('delivery_edit')">&#9003;</button>
          </div>
        </div>

        <div class="keypad-actions">
          <button class="act-btn secondary" @click="cancel">{{ t('delivery_cancel') }}</button>
          <button class="act-btn secondary" @click="clear">{{ t('clear') }}</button>
          <button class="act-btn primary" :disabled="!isValid" @click="goConfirm">{{ t('validate') }}</button>
        </div>
      </template>

      <template v-else>
        <h3>{{ confirmTitle }}</h3>
        <div class="email-confirm-display">{{ buffer }}</div>
        <div class="keypad-actions">
          <button class="act-btn secondary" @click="goBackToType">{{ editLabel }}</button>
          <button class="act-btn primary" @click="confirmText">{{ confirmLabel }}</button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.keypad-overlay {
  position: fixed; inset: 0; z-index: 1100;
  background: rgba(0, 0, 0, 0.75);
  display: flex; align-items: center; justify-content: center;
}

.keypad-modal {
  background: var(--bg-primary); border-radius: var(--radius-lg);
  padding: var(--space-lg);
  width: 92vw; max-width: 820px;
  display: flex; flex-direction: column; align-items: center; gap: var(--space-md);
}

.keypad-modal h3 { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); text-align: center; }

.email-display {
  width: 100%;
  min-height: 64px;
  padding: var(--space-md);
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
  border: 2px solid transparent;
  font-size: 28px;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
  text-align: center;
  overflow-wrap: anywhere;
}
.email-display.invalid { border-color: var(--accent-danger); }
.email-placeholder { color: var(--text-secondary); font-weight: var(--font-weight-normal); font-size: var(--font-size-md); }
.email-error { color: var(--accent-danger); font-size: var(--font-size-sm); }

.quick-keys { display: flex; gap: var(--space-sm); flex-wrap: wrap; justify-content: center; }
.quick-key {
  height: 48px; padding: 0 var(--space-md);
  font-size: var(--font-size-sm); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-secondary);
  transition: transform var(--tap-duration) ease;
}
.quick-key:active { transform: scale(var(--tap-scale)); }

.key-rows { display: flex; flex-direction: column; gap: var(--space-sm); align-items: center; }
.key-row { display: flex; gap: var(--space-sm); justify-content: center; flex-wrap: wrap; }

.key {
  width: 64px; height: 56px;
  font-size: var(--font-size-lg); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.key:active { transform: scale(var(--tap-scale)); background: var(--bg-pressed); }
.key-symbol { color: var(--text-secondary); }
.key-backspace { color: var(--accent-danger); }
.key-space { width: 160px; color: var(--text-secondary); }

.keypad-actions { display: flex; gap: var(--space-sm); width: 100%; }

.act-btn {
  flex: 1; height: 64px;
  font-size: var(--font-size-md); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  transition: transform var(--tap-duration) ease;
}
.act-btn:active { transform: scale(var(--tap-scale)); }
.act-btn:disabled { opacity: 0.3; pointer-events: none; }
.act-btn.primary { background: var(--accent-primary); color: var(--text-on-accent); }
.act-btn.secondary { background: var(--bg-surface); color: var(--text-primary); }

.email-confirm-display {
  width: 100%;
  padding: var(--space-xl) var(--space-md);
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
  font-size: 32px;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
  text-align: center;
  overflow-wrap: anywhere;
}
</style>
