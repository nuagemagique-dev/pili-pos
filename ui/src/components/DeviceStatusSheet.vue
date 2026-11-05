<script setup lang="ts">
// The sheet a status light opens (UX audit L2): the device, its state in
// words, the reason (the `title` tooltip stays, but it is no longer the
// only place), the time of the last check, and "Réessayer maintenant",
// which runs that device's check again. Used by the header lights and by
// the start-up rows of the login screen.
import { computed } from 'vue'
import { useI18n } from '../i18n'
import { formatTimeSeconds } from '../utils/datetime'
import { stateWordKey, type DeviceStatus } from '../utils/deviceStatus'

const { device, busy } = defineProps<{
  device: DeviceStatus
  /** The check is running: the retry button waits. */
  busy: boolean
}>()

const emit = defineEmits<{
  retry: []
  close: []
}>()

const { t } = useI18n()

const checkedText = computed(() =>
  device.checkedAt === null
    ? t('status_never_checked')
    : t('status_last_check', { time: formatTimeSeconds(new Date(device.checkedAt)) }),
)
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <div class="sheet" role="dialog" :aria-label="device.name" data-testid="device-sheet">
      <div class="head">
        <span class="dot" :class="device.state" />
        <h2 class="name" data-testid="device-sheet-name">{{ device.name }}</h2>
      </div>
      <div class="state" :class="device.state" data-testid="device-sheet-state">
        {{ t(stateWordKey(device.state)) }}
      </div>
      <p class="reason" data-testid="device-sheet-reason">{{ device.reason }}</p>
      <p class="checked" data-testid="device-sheet-checked">{{ checkedText }}</p>
      <div class="actions">
        <button
          class="btn"
          :disabled="busy"
          data-testid="device-sheet-retry"
          @click="emit('retry')"
        >
          {{ t('fdm_retry_now') }}
        </button>
        <button class="btn" data-testid="device-sheet-close" @click="emit('close')">
          {{ t('status_sheet_close') }}
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
  z-index: 60;
}
.sheet {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  width: min(560px, 92vw);
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.head {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}
.name {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.dot {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  flex-shrink: 0;
}
.dot.ok {
  background: var(--accent-success);
}
.dot.warn {
  background: var(--accent-warning);
}
.dot.err {
  background: var(--accent-danger);
}
.dot.none {
  background: var(--text-secondary);
  opacity: 0.4;
}
.state {
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-heavy);
}
.state.ok {
  color: var(--accent-success);
}
.state.warn {
  color: var(--accent-warning);
}
.state.err {
  color: var(--accent-danger);
}
.state.none {
  color: var(--text-secondary);
}
.reason {
  font-size: var(--font-size-md);
  line-height: 1.35;
  text-wrap: pretty;
}
.reason::first-letter {
  text-transform: uppercase;
}
.checked {
  font-family: var(--font-mono);
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
.actions {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}
.btn {
  height: 56px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.btn:active:not(:disabled) {
  transform: scale(var(--tap-scale));
}
.btn:disabled {
  opacity: 0.5;
}
</style>
