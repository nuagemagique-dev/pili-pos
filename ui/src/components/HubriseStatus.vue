<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'

const { t } = useI18n()

// Shows the HubRise connection state in the header bar.
// Green  = connected (token stored + poller active)
// Yellow = not connected (no token - needs OAuth setup)
// Grey   = unknown (IPC error)

const status = ref<'ok' | 'warn' | 'none'>('none')
let interval: ReturnType<typeof setInterval>

async function check() {
  try {
    const info = await ipc.hubriseGetConnectionStatus()
    status.value = info.connected ? 'ok' : 'warn'
  } catch {
    status.value = 'none'
  }
}

onMounted(() => {
  check()
  interval = setInterval(check, 60_000) // check every minute
})

onUnmounted(() => clearInterval(interval))
</script>

<template>
  <div class="status-indicator" :class="status">
    <span class="dot"></span>
    <span class="label">{{ t('status_orders') }}</span>
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
.warn .dot { background: var(--accent-warning); }
.none .dot { background: var(--text-secondary); opacity: 0.4; }
</style>
