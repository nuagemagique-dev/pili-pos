<script setup lang="ts">
// Fixed bottom-right stack of non-blocking toasts. Mounted once in
// App.vue. UX Guide §4.2: informative, ≤ 200ms transitions, no hover.
import { useToastStore } from '../stores/toast'

const store = useToastStore()
</script>

<template>
  <div class="toast-stack" aria-live="polite">
    <div
      v-for="t in store.toasts"
      :key="t.id"
      class="toast"
      :class="`toast-${t.variant}`"
      @click="store.dismiss(t.id)"
    >
      {{ t.message }}
    </div>
  </div>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  right: var(--space-md);
  bottom: var(--space-md);
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  z-index: 10000;
  pointer-events: none;
  max-width: 360px;
}

.toast {
  min-height: 56px;
  padding: var(--space-sm) var(--space-md);
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  background: var(--bg-secondary);
  color: var(--text-primary);
  border-left: 4px solid var(--accent-primary);
  border-radius: var(--radius-md);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  pointer-events: auto;
  cursor: pointer;
  animation: toast-in 150ms ease-out;
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
    transform: translateX(12px);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}
</style>
