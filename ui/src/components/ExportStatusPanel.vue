<script setup lang="ts">
// Nightly export status for Settings (manager only: the engine refuses
// the commands to anyone else, and SettingsView renders the panel for
// an admin session only). Polls the status every 30 s while mounted.
import { computed, onMounted, onUnmounted, ref } from 'vue'
import * as ipc from '../api/ipc'
import type { ExportStatus } from '../types'
import { useExportText } from '../i18n-export'
import { formatDayTime, formatTimestamp } from '../utils/datetime'
import { describeError } from '../utils/errorMessages'

const POLL_MS = 30_000

const { t } = useExportText()

const status = ref<ExportStatus | null>(null)
const callError = ref<string | null>(null)
const running = ref(false)

let timer: ReturnType<typeof setInterval> | undefined
let loading = false
let gone = false

async function load(keepError = false) {
  // A slow answer never stacks a second call on top of it.
  if (loading) return
  loading = true
  try {
    const next = await ipc.exportStatus()
    if (gone) return
    status.value = next
    if (!keepError) callError.value = null
  } catch (e) {
    // The last status stays on screen; the error shows above it.
    if (!gone) callError.value = describeError(e)
  } finally {
    loading = false
  }
}

/** Pending exports become due now; the engine answers at once, the
 * panel reads the new status after it. */
async function retryNow() {
  if (running.value) return
  running.value = true
  let refused = false
  try {
    await ipc.exportRunNow()
    callError.value = null
  } catch (e) {
    refused = true
    callError.value = describeError(e)
  }
  try {
    // A refused retry keeps its error on screen over the fresh status.
    await load(refused)
  } finally {
    running.value = false
  }
}

onMounted(() => {
  void load()
  timer = setInterval(() => void load(), POLL_MS)
})

onUnmounted(() => {
  gone = true
  if (timer !== undefined) clearInterval(timer)
  timer = undefined
})

const time = (iso: string | null) => (iso ? formatTimestamp(iso, formatDayTime) : t('never'))

const invalid = computed(() => Boolean(status.value?.problem))
const disabled = computed(() => status.value !== null && !status.value.enabled && !invalid.value)
const canRetry = computed(() => status.value !== null && status.value.enabled && !invalid.value)

const stateText = computed(() => {
  const s = status.value
  if (!s) return ''
  if (s.problem) return t('state_invalid')
  if (!s.enabled) return t('state_disabled')
  if (s.pending > 0) return t('state_pending', { n: s.pending })
  return t('state_ok')
})

const stateClass = computed(() => {
  const s = status.value
  if (!s) return ''
  if (s.problem) return 'state-err'
  if (!s.enabled) return 'state-off'
  return s.pending > 0 ? 'state-warn' : 'state-ok'
})
</script>

<template>
  <section class="export-panel" data-testid="export-panel">
    <div class="panel-head">
      <h2>{{ t('title') }}</h2>
      <button v-if="canRetry" class="retry-btn" :disabled="running" @click="retryNow">
        {{ running ? t('retrying') : t('retry_now') }}
      </button>
    </div>

    <p v-if="callError" class="call-error" role="alert">{{ callError }}</p>

    <template v-if="status">
      <div class="state-row">
        <span class="pill" :class="stateClass"><span class="dot"></span>{{ stateText }}</span>
      </div>
      <p v-if="status.problem" class="problem">{{ status.problem }}</p>

      <div v-if="!disabled && !invalid" class="rows">
        <div v-if="status.endpoint" class="row">
          <span>{{ t('endpoint') }}</span
          ><span class="value">{{ status.endpoint }}</span>
        </div>
        <div class="row">
          <span>{{ t('last_export') }}</span>
          <span class="value">
            <template v-if="status.lastExportId">{{ status.lastExportId }} · </template
            >{{ time(status.lastPreparedAt) }}
          </span>
        </div>
        <div class="row">
          <span>{{ t('last_upload') }}</span
          ><span class="value">{{ time(status.lastUploadedAt) }}</span>
        </div>
        <div v-if="status.pending > 0 && status.nextAttemptAt" class="row">
          <span>{{ t('next_attempt') }}</span
          ><span class="value">{{ time(status.nextAttemptAt) }}</span>
        </div>
        <div v-if="status.lastError" class="row error-row">
          <span>{{ t('last_error') }}</span>
          <span class="value">
            {{ status.lastError
            }}<template v-if="status.lastErrorAt"> · {{ time(status.lastErrorAt) }}</template>
          </span>
        </div>
        <div class="row">
          <span>{{ t('kept', { n: status.kept, days: status.keepDays }) }}</span>
        </div>
        <div v-if="status.dropped > 0" class="row">
          <span>{{ t('dropped') }}</span
          ><span class="value">{{ status.dropped }}</span>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.export-panel {
  flex-shrink: 0;
}
.export-panel h2 {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}
.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
}

.retry-btn {
  height: 48px;
  padding: 0 16px;
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.retry-btn:active {
  transform: scale(var(--tap-scale));
}
.retry-btn:disabled {
  opacity: 0.5;
}

.state-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.pill {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: 0 14px;
  border-radius: 18px;
  border: 1px solid var(--border);
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
.pill .dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: currentColor;
}
.state-ok {
  color: var(--accent-success);
}
.state-warn {
  color: var(--accent-warning);
}
.state-err {
  color: var(--accent-danger);
}
.state-off {
  color: var(--text-secondary);
}

.problem,
.call-error {
  font-size: 15px;
  color: var(--accent-danger);
  overflow-wrap: anywhere;
}

.row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  min-height: 40px;
  border-bottom: 1px solid var(--border);
  font-size: 16px;
}
.row span:first-child {
  color: var(--text-secondary);
}
.row .value {
  font-family: var(--font-mono);
  text-align: right;
  overflow-wrap: anywhere;
}
.error-row span,
.error-row .value {
  color: var(--accent-danger);
}
</style>
