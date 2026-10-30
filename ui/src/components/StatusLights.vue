<script setup lang="ts">
/**
 * Device lights in the header (UX guide §8.1): the FDM, the ticket
 * printer, the kitchen printer, the linked card terminal (only when one
 * is configured; amber while a payment runs on it) and HubRise.
 * Green = OK, amber = warning, red = error, grey = not configured.
 *
 * The FDM light follows the engine's monitor (store fdm, event
 * "fdm-status"), which also keeps FdmAlerts current.
 */
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useIntegrationsStore } from '../stores/integrations'
import * as ipc from '../api/ipc'
import { useEftStore } from '../stores/eft'
import { useFdmStore } from '../stores/fdm'
import { useI18n } from '../i18n'
import type { PaymentTerminalStatus } from '../types'

type Light = 'ok' | 'warn' | 'err' | 'none'

const fdm = useFdmStore()
const { t } = useI18n()

const fdmLight = computed<Light>(() => {
  if (fdm.severity === 'blocking') return 'err'
  if (fdm.severity === 'warning' || !fdm.connected) return 'warn'
  return 'ok'
})
const ticket = ref<Light>('none')
const kitchen = ref<Light>('none')
const hubrise = ref<Light>('none')
const eft = useEftStore()
const terminalStatus = ref<PaymentTerminalStatus | null>(null)
// Hidden until the engine says a terminal is configured.
const terminalLight = computed(() => {
  const status = terminalStatus.value
  if (!status?.configured) return null
  const state: Light = status.busy ? 'warn' : status.connected ? 'ok' : 'err'
  const why = status.busy
    ? t('check_payment_busy')
    : status.connected
      ? t('check_connected')
      : [t('check_unreachable'), status.detail].filter(Boolean).join(': ')
  return { state, title: why }
})
const scaleStatus = ref<ipc.ScaleStatus | null>(null)
// Hidden until pos.json has a scale. A refused software check is red: no weighing can pass.
const scaleLight = computed(() => {
  const status = scaleStatus.value
  if (!status?.configured) return null
  if (status.check === 'refused' || status.check === 'no_values') {
    return { state: 'err' as Light, title: t('scale_check_refused') }
  }
  if (status.busy) return { state: 'warn' as Light, title: t('scale_busy') }
  return status.connected
    ? { state: 'ok' as Light, title: t('check_connected') }
    : { state: 'err' as Light, title: t('check_unreachable') }
})
// Only while the integration runs.
const integrations = useIntegrationsStore()
const hubriseLights = computed(() =>
  integrations.hubriseEnabled ? [{ label: 'HubRise', state: hubrise.value }] : [],
)

// §4.2: no continuous loops. Pulse once when the FDM degrades.
const fdmPulse = ref(false)
watch(fdmLight, (next, prev) => {
  if (next !== prev && (next === 'warn' || next === 'err')) {
    fdmPulse.value = true
    setTimeout(() => (fdmPulse.value = false), 180)
  }
})

async function checkPrinters() {
  try {
    ticket.value = (await ipc.checkPrinter()) ? 'ok' : 'err'
  } catch {
    ticket.value = 'none'
  }
  try {
    const reachable = await ipc.checkKitchenPrinter()
    kitchen.value = reachable === null ? 'none' : reachable ? 'ok' : 'err'
  } catch {
    kitchen.value = 'none'
  }
}

async function checkTerminal() {
  try {
    terminalStatus.value = await ipc.checkPaymentTerminal()
  } catch {
    terminalStatus.value = null
  }
}

async function checkScale() {
  try {
    scaleStatus.value = await ipc.getScaleStatus()
  } catch {
    scaleStatus.value = null
  }
}

// A payment starting or ending changes what the terminal answers.
watch(
  () => eft.phase,
  () => void checkTerminal(),
)

async function checkHubrise() {
  if (!integrations.hubriseEnabled) return
  try {
    hubrise.value = hubriseLight(await ipc.hubriseStatus())
  } catch {
    hubrise.value = 'none'
  }
}

/**
 * Red: HubRise refuses the token, the last catalog was refused, or
 * something is parked (an event or an update HubRise refused). Amber:
 * not connected, online orders waiting over 5 minutes, a menu to take
 * over, or updates waiting after a failed send.
 */
function hubriseLight(h: ipc.HubriseHealth): Light {
  const catalogRefused =
    typeof h.catalog?.rejected_at === 'string' &&
    h.catalog.rejected_at !== '' &&
    (!h.catalog.applied_at || h.catalog.rejected_at >= h.catalog.applied_at)
  if (h.auth_error_at || catalogRefused || h.parked_events > 0 || h.outbox.parked > 0) {
    return 'err'
  }
  if (
    !h.connected ||
    h.stale_orders > 0 ||
    h.catalog_awaiting_adoption ||
    (h.outbox.pending > 0 && h.outbox_error)
  ) {
    return 'warn'
  }
  return 'ok'
}

let printerTimer: ReturnType<typeof setInterval> | undefined
let hubriseTimer: ReturnType<typeof setInterval> | undefined

onMounted(() => {
  void checkPrinters()
  void checkTerminal()
  void checkScale()
  void checkHubrise()
  printerTimer = setInterval(() => {
    void checkPrinters()
    void checkTerminal()
    void checkScale()
  }, 30_000)
  hubriseTimer = setInterval(() => void checkHubrise(), 60_000)
})
onUnmounted(() => {
  clearInterval(printerTimer)
  clearInterval(hubriseTimer)
})
</script>

<template>
  <div class="lights">
    <div class="light fdm" :class="fdmLight">
      <span class="dot" :class="{ pulse: fdmPulse }" />
      <span>FDM</span>
    </div>
    <div class="light" :class="ticket">
      <span class="dot" />
      <span>{{ t('status_printer') }}</span>
    </div>
    <div class="light" :class="kitchen">
      <span class="dot" />
      <span>{{ t('status_kitchen') }}</span>
    </div>
    <div
      v-if="terminalLight"
      class="light"
      :class="terminalLight.state"
      :title="terminalLight.title"
      data-testid="light-terminal"
    >
      <span class="dot" />
      <span>{{ t('status_terminal') }}</span>
    </div>
    <div
      v-if="scaleLight"
      class="light"
      :class="scaleLight.state"
      :title="scaleLight.title"
      data-testid="light-scale"
    >
      <span class="dot" />
      <span>{{ t('status_scale') }}</span>
    </div>
    <div v-for="light in hubriseLights" :key="light.label" class="light" :class="light.state">
      <span class="dot" />
      <span>{{ light.label }}</span>
    </div>
  </div>
</template>

<style scoped>
.lights {
  display: flex;
  align-items: center;
  gap: 16px;
}

.light {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--text-secondary);
  white-space: nowrap;
}
/* The FDM label turns red with its light: sales are blocked. */
.light.fdm.err {
  color: var(--accent-danger);
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.ok .dot {
  background: var(--accent-success);
}
.warn .dot {
  background: var(--accent-warning);
}
.err .dot {
  background: var(--accent-danger);
}
.none .dot {
  background: var(--text-secondary);
  opacity: 0.4;
}

/* One-shot scale pulse on transition into warn / err (§4.2, ≤ 200 ms). */
.dot.pulse {
  animation: light-pulse 150ms ease-out;
}
@keyframes light-pulse {
  0% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.6);
  }
  100% {
    transform: scale(1);
  }
}
</style>
