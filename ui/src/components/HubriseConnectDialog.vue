<script setup lang="ts">
// Connecting to HubRise from the kiosk, which has no browser: the
// login link as a QR code to open on a phone, and the field for the
// code HubRise shows once the location is picked.
import { ref } from "vue";
import type { HubriseAuthorizeLink } from "../api/ipc";
import { useI18n } from "../i18n";

defineProps<{
  link: HubriseAuthorizeLink;
  loading: boolean;
  error: string | null;
}>();

const emit = defineEmits<{
  submit: [code: string];
  cancel: [];
}>();

const { t } = useI18n();
const code = ref("");

function submit() {
  const value = code.value.trim();
  if (value) emit("submit", value);
}
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div class="dialog" role="dialog" :aria-label="t('hubrise_open_authorize_url')" data-testid="hubrise-authorize">
      <h2 class="title">{{ t('hubrise_open_authorize_url') }}</h2>
      <div class="body">
        <!-- SVG rendered by the backend from the URL alone. -->
        <div class="qr" v-html="link.qr_svg"></div>
        <div class="steps">
          <p class="help">{{ t('hubrise_authorize_scan') }}</p>
          <p class="url">{{ link.url }}</p>
        </div>
      </div>
      <div class="code-row">
        <input
          v-model="code"
          class="field"
          :placeholder="t('hubrise_code_placeholder')"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          @keyup.enter="submit"
        />
      </div>
      <p v-if="error" class="error">{{ error }}</p>
      <div class="actions">
        <button class="btn btn-cancel" @click="emit('cancel')">{{ t('back') }}</button>
        <button class="btn btn-confirm" :disabled="loading || !code.trim()" @click="submit">
          {{ t('hubrise_complete_connection') }}
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
  gap: var(--space-md);
  width: min(880px, calc(100vw - 48px));
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); }
.body { display: flex; gap: var(--space-lg); align-items: flex-start; }
.qr {
  flex-shrink: 0;
  width: 320px;
  height: 320px;
  padding: 12px;
  border-radius: var(--radius-md);
  background: #ffffff;
}
.qr :deep(svg) { display: block; width: 100%; height: 100%; }
.steps { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: var(--space-md); }
.help { font-size: 20px; line-height: 1.4; text-wrap: pretty; }
.url {
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text-secondary);
  overflow-wrap: anywhere;
  user-select: all;
}
.code-row { display: flex; }
.field {
  flex: 1;
  min-width: 0;
  height: 56px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 18px;
}
.field::placeholder { color: var(--text-secondary); font-family: var(--font-family); }
.error { color: var(--accent-danger); font-size: 16px; }
.actions { display: flex; gap: var(--space-sm); }
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
