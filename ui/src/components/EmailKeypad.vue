<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "../i18n";

// On-screen keyboard for typing a customer's email address — the
// kiosk has no physical keyboard. Two steps: type (with live
// validation) then confirm (large type, per cert requirement that the
// address is confirmed on screen before it is used).

const emit = defineEmits<{
  confirm: [email: string];
  cancel: [];
}>();

const { t } = useI18n();

const buffer = ref("");
const step = ref<"type" | "confirm">("type");

const LETTER_ROWS = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
const DIGIT_ROW = "1234567890".split("");
const SYMBOL_ROW = ["@", ".", "-", "_", "+"];
const QUICK_KEYS = [".be", ".com", "@gmail.com"];

const MAX_LEN = 60;

function press(ch: string) {
  if (buffer.value.length >= MAX_LEN) return;
  buffer.value += ch;
}

function pressQuick(chunk: string) {
  if (buffer.value.length + chunk.length > MAX_LEN) return;
  buffer.value += chunk;
}

function backspace() {
  buffer.value = buffer.value.slice(0, -1);
}

function clear() {
  buffer.value = "";
}

// Basic local@domain.tld shape, no whitespace. The backend re-validates
// authoritatively before signing — this is just to save the operator a
// round trip on an obvious typo.
const isValid = computed(() => {
  const v = buffer.value;
  if (!v || /\s/.test(v)) return false;
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v);
});

function goConfirm() {
  if (!isValid.value) return;
  step.value = "confirm";
}

function goBackToType() {
  step.value = "type";
}

function confirmEmail() {
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
        <h3>{{ t('delivery_enter_email_title') }}</h3>

        <div class="email-display" :class="{ invalid: buffer.length > 0 && !isValid }">
          <span v-if="buffer">{{ buffer }}</span>
          <span v-else class="email-placeholder">{{ t('delivery_email_placeholder') }}</span>
        </div>
        <p v-if="buffer.length > 0 && !isValid" class="email-error">{{ t('delivery_invalid_email') }}</p>

        <div class="quick-keys">
          <button v-for="q in QUICK_KEYS" :key="q" class="quick-key" @click="pressQuick(q)">{{ q }}</button>
        </div>

        <div class="key-rows">
          <div v-for="(row, i) in LETTER_ROWS" :key="i" class="key-row">
            <button v-for="ch in row" :key="ch" class="key" @click="press(ch)">{{ ch }}</button>
          </div>
          <div class="key-row">
            <button v-for="d in DIGIT_ROW" :key="d" class="key" @click="press(d)">{{ d }}</button>
          </div>
          <div class="key-row">
            <button v-for="s in SYMBOL_ROW" :key="s" class="key key-symbol" @click="press(s)">{{ s }}</button>
            <button class="key key-backspace" @click="backspace" :aria-label="t('delivery_edit')">&#9003;</button>
          </div>
        </div>

        <div class="keypad-actions">
          <button class="act-btn secondary" @click="cancel">{{ t('delivery_cancel') }}</button>
          <button class="act-btn secondary" @click="clear">{{ t('clear') }}</button>
          <button class="act-btn primary" :disabled="!isValid" @click="goConfirm">{{ t('validate') }}</button>
        </div>
      </template>

      <template v-else>
        <h3>{{ t('delivery_confirm_title') }}</h3>
        <div class="email-confirm-display">{{ buffer }}</div>
        <div class="keypad-actions">
          <button class="act-btn secondary" @click="goBackToType">{{ t('delivery_edit') }}</button>
          <button class="act-btn primary" @click="confirmEmail">{{ t('delivery_confirm_btn') }}</button>
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
