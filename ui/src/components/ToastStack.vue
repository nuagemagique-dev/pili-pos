<script setup lang="ts">
// Non-blocking toasts, top centre just under the 64 px header (C5),
// stacked downwards. Mounted once in App.vue. UX Guide §4.2:
// informative, ≤ 200ms transitions, no hover.
//
// The container never takes a tap: it lets everything through to the
// screen below. Info, success and warning toasts are click-through
// too and go away by themselves. An error toast stays until tapped:
// it is the only thing here that takes a tap, and a tap on it only
// closes it.
import { useToastStore } from '../stores/toast'
import { useI18n } from '../i18n'

const store = useToastStore()
const { t } = useI18n()
</script>

<template>
  <div class="toast-stack" aria-live="polite" data-testid="toast-stack">
    <div
      v-for="toast in store.visible"
      :key="toast.id"
      class="toast"
      :class="[`toast-${toast.variant}`, { 'toast-sticky': toast.variant === 'error' }]"
      :role="toast.variant === 'error' ? 'button' : undefined"
      :tabindex="toast.variant === 'error' ? 0 : undefined"
      @click="toast.variant === 'error' && store.dismiss(toast.id)"
      @keydown.enter="toast.variant === 'error' && store.dismiss(toast.id)"
    >
      <span class="toast-msg">{{ toast.message }}</span>
      <span v-if="toast.variant === 'error'" class="toast-hint">{{ t('toast_tap_to_close') }}</span>
    </div>
  </div>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  top: calc(var(--header-height) + var(--space-sm));
  left: 50%;
  transform: translateX(-50%);
  width: min(560px, calc(100vw - 2 * var(--space-md)));
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  z-index: 10000;
  /* Never a tap target: a tap goes through to what is under it. */
  pointer-events: none;
}

.toast {
  min-height: 56px;
  padding: var(--space-sm) var(--space-md);
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 2px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  border-left: 4px solid var(--accent-primary);
  border-radius: var(--radius-md);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  pointer-events: none;
  animation: toast-in 150ms ease-out;
}

/* The one toast that takes a tap: an error waiting to be closed. */
.toast-sticky {
  pointer-events: auto;
  cursor: pointer;
}

.toast-hint {
  font-size: var(--font-size-xs);
  font-weight: 500;
  color: var(--text-secondary);
}

.toast-success {
  border-left-color: var(--accent-success);
}
.toast-warning {
  border-left-color: var(--accent-warning);
}
.toast-error {
  border-left-color: var(--accent-danger);
}
.toast-info {
  border-left-color: var(--accent-info);
}

@keyframes toast-in {
  from {
    transform: translateY(-8px);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}
</style>
