<script setup lang="ts">
/**
 * Device lights in the header (UX guide §8.1): the FDM, the ticket
 * printer, the kitchen printer, the linked card terminal (only when one
 * is configured; amber while a payment runs on it) and HubRise.
 * Green = OK, amber = warning, red = error, grey = not configured.
 *
 * The FDM light follows the engine's monitor (store fdm, event
 * "fdm-status"), which also keeps FdmAlerts current.
 *
 * Each light is a real button with a 48 px hit area (UX audit L2): a tap
 * opens a sheet with the device, its state in words, the reason, the time
 * of the last check and "Réessayer maintenant", which runs that device's
 * check again. The `title` tooltip stays on some lights but is no longer
 * the only place for the reason.
 */
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useIntegrationsStore } from '../stores/integrations'
import * as ipc from '../api/ipc'
import { useEftStore } from '../stores/eft'
import { useFdmStore } from '../stores/fdm'
import { useI18n } from '../i18n'
import { translateFdmError } from '../utils/fdmMessages'
import { stateWordKey, type DeviceStatus, type Light } from '../utils/deviceStatus'
import DeviceStatusSheet from './DeviceStatusSheet.vue'
import type { PaymentTerminalStatus } from '../types'

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

/** When each device was last checked (ms since epoch), for the sheet. */
const checkedAt = ref<Record<string, number>>({})
function stamp(key: string) {
  checkedAt.value = { ...checkedAt.value, [key]: Date.now() }
}

async function checkTicket() {
  try {
    ticket.value = (await ipc.checkPrinter()) ? 'ok' : 'err'
  } catch {
    ticket.value = 'none'
  }
  stamp('ticket')
}

async function checkKitchen() {
  try {
    const reachable = await ipc.checkKitchenPrinter()
    kitchen.value = reachable === null ? 'none' : reachable ? 'ok' : 'err'
  } catch {
    kitchen.value = 'none'
  }
  stamp('kitchen')
}

async function checkPrinters() {
  await checkTicket()
  await checkKitchen()
}

async function checkTerminal() {
  try {
    terminalStatus.value = await ipc.checkPaymentTerminal()
  } catch {
    terminalStatus.value = null
  }
  stamp('terminal')
}

async function checkScale() {
  try {
    scaleStatus.value = await ipc.getScaleStatus()
  } catch {
    scaleStatus.value = null
  }
  stamp('scale')
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
  stamp('hubrise')
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

// ── The status sheet (L2) ─────────────────────────────────────────

/** One light of the row, with what the sheet shows about it. */
interface LightRow {
  key: string
  /** Short label of the light. */
  label: string
  /** Full device name for the sheet. */
  name: string
  state: Light
  /** The tooltip (the reason, kept for the lights that had one). */
  title?: string
  reason: string
}

/** The reason in words for a printer or an other device that only answers yes or no. */
function plainReason(state: Light, errKey: string): string {
  if (state === 'ok') return t('status_reason_ok')
  if (state === 'none') return t('status_reason_none')
  return t(errKey)
}

const fdmReason = computed(() => {
  const said = translateFdmError(fdm.lastError)
  if (said !== null) return said
  if (!fdm.connected) return t('status_reason_fdm_offline')
  return t('status_reason_ok')
})

const rows = computed<LightRow[]>(() => {
  const list: LightRow[] = [
    {
      key: 'fdm',
      label: 'FDM',
      name: 'FDM',
      state: fdmLight.value,
      reason: fdmReason.value,
    },
    {
      key: 'ticket',
      label: t('status_printer'),
      name: t('check_ticket_printer'),
      state: ticket.value,
      reason: plainReason(ticket.value, 'status_reason_printer_err'),
    },
    {
      key: 'kitchen',
      label: t('status_kitchen'),
      name: t('check_kitchen_printer'),
      state: kitchen.value,
      reason: plainReason(kitchen.value, 'status_reason_printer_err'),
    },
  ]
  if (terminalLight.value) {
    list.push({
      key: 'terminal',
      label: t('status_terminal'),
      name: t('check_payment_terminal'),
      state: terminalLight.value.state,
      title: terminalLight.value.title,
      reason: terminalLight.value.title,
    })
  }
  if (scaleLight.value) {
    list.push({
      key: 'scale',
      label: t('status_scale'),
      name: t('status_scale'),
      state: scaleLight.value.state,
      title: scaleLight.value.title,
      reason: scaleLight.value.title,
    })
  }
  for (const light of hubriseLights.value) {
    list.push({
      key: 'hubrise',
      label: light.label,
      name: light.label,
      state: light.state,
      reason:
        light.state === 'warn'
          ? t('status_reason_hubrise_warn')
          : plainReason(light.state, 'status_reason_hubrise_err'),
    })
  }
  return list
})

/** The check that "Réessayer maintenant" runs again, per device. */
const retries: Record<string, () => Promise<unknown>> = {
  fdm: () => fdm.checkNow(),
  ticket: checkTicket,
  kitchen: checkKitchen,
  terminal: checkTerminal,
  scale: checkScale,
  hubrise: checkHubrise,
}

const openKey = ref<string | null>(null)
const retrying = ref(false)

// The sheet shows the live row: after a retry it shows the new answer.
const sheetDevice = computed<DeviceStatus | null>(() => {
  const row = rows.value.find((r) => r.key === openKey.value)
  if (!row) return null
  return {
    key: row.key,
    name: row.name,
    state: row.state,
    reason: row.reason,
    checkedAt: row.key === 'fdm' ? fdm.checkedAt : (checkedAt.value[row.key] ?? null),
  }
})

async function retryOpen() {
  const key = openKey.value
  if (key === null || retrying.value) return
  retrying.value = true
  try {
    await retries[key]?.()
  } finally {
    retrying.value = false
  }
}

function ariaLabel(row: LightRow): string {
  return t('status_light_aria', { name: row.name, state: t(stateWordKey(row.state)) })
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
    <button
      v-for="row in rows"
      :key="row.key"
      type="button"
      class="light"
      :class="[row.state, { fdm: row.key === 'fdm' }]"
      :title="row.title"
      :aria-label="ariaLabel(row)"
      :data-testid="`light-${row.key}`"
      @click="openKey = row.key"
    >
      <span class="dot" :class="{ pulse: row.key === 'fdm' && fdmPulse }" />
      <span>{{ row.label }}</span>
    </button>

    <Teleport to="body">
      <DeviceStatusSheet
        v-if="sheetDevice"
        :device="sheetDevice"
        :busy="retrying"
        @retry="retryOpen"
        @close="openKey = null"
      />
    </Teleport>
  </div>
</template>

<style scoped>
.lights {
  display: flex;
  align-items: center;
  gap: 4px;
}

/* A real button: the dot is small but the hit area is 48 px high. */
.light {
  min-height: 48px;
  min-width: 48px;
  padding: 0 6px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--text-secondary);
  white-space: nowrap;
}
.light:active {
  background: var(--bg-surface);
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
