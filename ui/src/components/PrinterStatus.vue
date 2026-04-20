<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'

const { t } = useI18n()

// Polls the printer every 30s via TCP ping.
// Green = reachable, Red = unreachable, Grey = not configured.

const status = ref<'ok' | 'err' | 'none'>('none')
let interval: ReturnType<typeof setInterval>

async function check() {
  try {
    const reachable = await ipc.checkPrinter()
    status.value = reachable ? 'ok' : 'err'
  } catch {
    status.value = 'none'
  }
}

onMounted(() => {
  check()
  interval = setInterval(check, 30_000)
})

onUnmounted(() => clearInterval(interval))
</script>

<template>
  <div class="status-indicator" :class="status">
    <span class="dot"></span>
    <span class="label">{{ t('status_printer') }}</span>
  </div>
</template>

<style scoped>
.status-indicator {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.ok .dot { background: var(--accent-success); }
.err .dot { background: var(--accent-danger); }
.none .dot { background: var(--text-secondary); opacity: 0.4; }
</style>
