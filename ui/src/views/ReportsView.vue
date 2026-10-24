<script setup lang="ts">
import { describeError } from '../utils/errorMessages'
import { formatDate as formatDay, formatTime, formatTimestamp } from '../utils/datetime'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useToastStore } from '../stores/toast'
import { useDeliveryStore } from '../stores/delivery'
import { useInvoiceStore } from '../stores/invoice'
import { formatPrice, round5 } from '../utils/format'
import type { CheckoutResult, OrderLine, UnsettledCardPayment } from '../types'
import type { CopyResult, RefundableEvent, RefundItem } from '../api/ipc'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'
import AppHeader from '../components/AppHeader.vue'
import TicketMediumPicker from '../components/TicketMediumPicker.vue'
import DeliveryOutcomePanel from '../components/DeliveryOutcomePanel.vue'
import PriceDialog from '../components/PriceDialog.vue'
import PaymentCorrectionDialog from '../components/PaymentCorrectionDialog.vue'
import InvoiceDialog from '../components/InvoiceDialog.vue'
import InvoiceList from '../components/InvoiceList.vue'
import SalesReportPanel from '../components/SalesReportPanel.vue'
import GiftVouchersReport from '../components/GiftVouchersReport.vue'
import { useGiftVouchersStore } from '../stores/giftVouchers'

const router = useRouter()
const auth = useAuthStore()
const toast = useToastStore()
const delivery = useDeliveryStore()
const invoice = useInvoiceStore()
const giftVouchers = useGiftVouchersStore()
const { t } = useI18n()

// §2.1: four or five tabs, never more than 6-8 actions visible at once.
type Tab = 'day' | 'cash' | 'tickets' | 'sales' | 'vouchers'
const TABS = computed<{ id: Tab; label: string }[]>(() => [
  { id: 'day', label: 'reports_tab_day' },
  { id: 'cash', label: 'reports_tab_cash' },
  { id: 'tickets', label: 'reports_tab_tickets' },
  { id: 'sales', label: 'reports_tab_sales' },
  ...(giftVouchers.enabled ? [{ id: 'vouchers' as const, label: 'reports_tab_vouchers' }] : []),
])
const tab = ref<Tab>('day')

const loading = ref(false)
const loadingKind = ref<'x' | 'userx' | 'z' | null>(null)
const zHistory = ref<ipc.ZReportSummary[]>([])
const zPending = ref(false)
// Card payments taken on the terminal that no sale records yet: told to
// the operator at the Z, which they never block.
const unsettled = ref<UnsettledCardPayment[]>([])

async function loadHistory() {
  try {
    zHistory.value = await ipc.listZReports()
  } catch {
    /* silently degrade - history is non-critical */
  }
}

async function loadZPending() {
  try {
    zPending.value = await ipc.needsZReport()
  } catch {
    /* the notice is only a reminder */
    zPending.value = false
  }
}

async function loadUnsettled() {
  try {
    unsettled.value = await ipc.listUnsettledCardPayments()
  } catch {
    /* only a warning: the Z does not depend on it */
    unsettled.value = []
  }
}

function unsettledWhere(p: UnsettledCardPayment): string {
  return p.table_number === null ? t('takeaway_badge') : `${t('table')} ${p.table_number}`
}

function unsettledLine(p: UnsettledCardPayment): string {
  return t('reports_z_unsettled_item', {
    where: unsettledWhere(p),
    amount: formatPrice(p.amount_cents),
    brand: p.brand ? ` · ${p.brand}` : '',
  })
}

onMounted(() => {
  void giftVouchers.load()
  if (!auth.currentUser) {
    router.push({ name: 'login' })
    return
  }
  loadHistory()
  loadZPending()
  loadUnsettled()
})

const lastZ = computed(() => zHistory.value[0] ?? null)

async function generateX() {
  if (!auth.currentUser || loading.value) return
  loading.value = true
  loadingKind.value = 'x'
  try {
    const result = await ipc.generateXReport(auth.currentUser.insz)
    if (result.success) {
      toast.show(`${t('report_generated')} (X)`, 'success', 4000)
    } else {
      toast.show(describeError(result.error), 'error', 5000)
    }
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
  } finally {
    loading.value = false
    loadingKind.value = null
  }
}

async function generateUserX() {
  if (!auth.currentUser || loading.value) return
  loading.value = true
  loadingKind.value = 'userx'
  try {
    const result = await ipc.generateUserXReport(auth.currentUser.insz)
    if (result.success) {
      toast.show(`${t('report_generated')} (X util.)`, 'success', 4000)
    } else {
      toast.show(describeError(result.error), 'error', 5000)
    }
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
  } finally {
    loading.value = false
    loadingKind.value = null
  }
}

// A Z closes the day for good: the first tap only arms the button,
// the second (within 5 s) generates it.
const zArmed = ref(false)
let zDisarm: ReturnType<typeof setTimeout> | undefined
onUnmounted(() => clearTimeout(zDisarm))

// Leaving the day tab disarms the Z: it must be armed where it is seen.
watch(tab, () => {
  clearTimeout(zDisarm)
  zArmed.value = false
})

async function generateZ() {
  if (!auth.currentUser || loading.value) return
  if (!zArmed.value) {
    loadUnsettled()
    zArmed.value = true
    zDisarm = setTimeout(() => {
      zArmed.value = false
    }, 5000)
    return
  }
  clearTimeout(zDisarm)
  zArmed.value = false
  loading.value = true
  loadingKind.value = 'z'
  try {
    const result = await ipc.generateZReport(auth.currentUser.insz)
    if (result.success) {
      const label = result.reportNo
        ? `${t('report_generated')} (Z #${result.reportNo})`
        : `${t('report_generated')} (Z)`
      toast.show(label, 'success', 4000)
      await loadHistory()
      await loadZPending()
      await loadUnsettled()
    } else {
      toast.show(describeError(result.error), 'error', 5000)
    }
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
  } finally {
    loading.value = false
    loadingKind.value = null
  }
}

// ── Z history (reprint list) ─────────────────────────────────────
const zListOpen = ref(false)

async function openZList() {
  zListOpen.value = true
  await loadHistory()
}

function reprintZ(report: ipc.ZReportSummary) {
  zListOpen.value = false
  openCopy(report, 'z')
}

// ── Copy flow (event C) - picker shown before signing ──────────────
const copyOpen = ref(false)
const copyStep = ref<'picker' | 'processing' | 'result'>('picker')
const copyTarget = ref<{ fiscalEventId: number | null } | null>(null)
const copyKindOf = ref<'z' | 'ticket'>('ticket')
const copyResult = ref<CopyResult | null>(null)

/** Copy of a Z report (from the history) or of a customer ticket. */
function openCopy(report: { fiscalEventId: number | null }, kind: 'z' | 'ticket') {
  if (!report.fiscalEventId) return
  copyTarget.value = report
  copyKindOf.value = kind
  copyResult.value = null
  delivery.reset()
  delivery.refreshMailStatus()
  copyStep.value = 'picker'
  copyOpen.value = true
}

function copyKind(ticket: ipc.CopyableTicket): string {
  if (ticket.eventLabel === 'T') return t('copy_kind_training')
  if (ticket.eventLabel === 'P') return t('copy_kind_pre_bill')
  return ticket.totalCents < 0 ? t('copy_kind_refund') : t('copy_kind_sale')
}

function closeCopy() {
  const signed = copyStep.value === 'result'
  copyOpen.value = false
  copyResult.value = null
  delivery.reset()
  if (signed && copyKindOf.value === 'ticket') loadTickets()
}

async function confirmCopy() {
  const insz = auth.currentUser?.insz
  const target = copyTarget.value
  if (!insz || !target?.fiscalEventId) return
  copyStep.value = 'processing'
  try {
    const req = delivery.request.medium === 'paper' ? undefined : delivery.request
    const result = await ipc.copyTicket(target.fiscalEventId, insz, req)
    copyResult.value = result
    if (result.success) {
      toast.show(t('report_reprint_ok'), 'success')
    } else {
      toast.show(describeError(result.error), 'error', 5000)
    }
    copyStep.value = 'result'
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
    copyOpen.value = false
  }
}

function formatDate(iso: string): string {
  return iso ? formatTimestamp(iso, formatDay) : '-'
}

// ── Cash management (Event F) - direction, reason, amount ────────
type CashDirection = 'in' | 'out'
const cashDirection = ref<CashDirection>('in')
const cashAmountCents = ref(0)
const cashLoading = ref(false)
const cashPresets = [1000, 2000, 5000, 10000] // 10€, 20€, 50€, 100€ - "Autre" opens the keypad

// Only the reasons that make sense for the chosen direction.
const CASH_REASONS: Record<CashDirection, string[]> = {
  in: ['fond_de_caisse', 'pourboire'],
  out: ['retrait', 'pourboire'],
}
const cashReasons = computed(() => CASH_REASONS[cashDirection.value])
const cashReason = ref<string>(CASH_REASONS.in[0])

function setCashDirection(dir: CashDirection) {
  cashDirection.value = dir
  const [first] = CASH_REASONS[dir]
  if (!CASH_REASONS[dir].includes(cashReason.value)) cashReason.value = first
}

function reasonLabel(key: string): string {
  return t(`reports_reason_${key}`)
}

const cashCustom = computed(
  () => cashAmountCents.value > 0 && !cashPresets.includes(cashAmountCents.value),
)
const cashSummary = computed(
  () =>
    `${t(cashDirection.value === 'in' ? 'reports_dir_in' : 'reports_dir_out')} · ${reasonLabel(cashReason.value)}`,
)

const otherAmountOpen = ref(false)
function handleOtherAmount(cents: number) {
  otherAmountOpen.value = false
  cashAmountCents.value = cents
}

async function handleCashInOut() {
  const insz = auth.currentUser?.insz
  if (!insz || cashAmountCents.value <= 0 || !cashReason.value) return
  cashLoading.value = true
  try {
    const result = await ipc.moneyInOut(
      insz,
      cashAmountCents.value,
      cashDirection.value,
      cashReason.value,
    )
    if (result.success) {
      toast.show(t('cash_success'), 'success')
      cashAmountCents.value = 0
    } else {
      toast.show(result.error ? describeError(result.error) : t('cash_failed'), 'error', 5000)
    }
  } catch (e) {
    toast.show(describeError(e), 'error')
  } finally {
    cashLoading.value = false
  }
}

// Cash count (Basic Set M133): any amount, 0 for an empty drawer.
const declarationOpen = ref(false)

async function handleDeclaration(amountCents: number) {
  declarationOpen.value = false
  const insz = auth.currentUser?.insz
  if (!insz) return
  cashLoading.value = true
  try {
    const result = await ipc.moneyInOut(insz, amountCents, 'declaration', 'declaration_tiroir')
    if (result.success) {
      toast.show(t('cash_declared'), 'success')
    } else {
      toast.show(result.error ? describeError(result.error) : t('cash_failed'), 'error', 5000)
    }
  } catch (e) {
    toast.show(describeError(e), 'error')
  } finally {
    cashLoading.value = false
  }
}

async function handleOpenDrawer() {
  const insz = auth.currentUser?.insz
  if (!insz) return
  try {
    const result = await ipc.openDrawerFiscal(insz)
    if (result.success) {
      toast.show(t('cash_drawer_opened'), 'success')
    } else {
      toast.show(result.error ? describeError(result.error) : t('cash_failed'), 'error', 5000)
    }
  } catch (e) {
    toast.show(describeError(e), 'error')
  }
}

// ── Tickets tab: copy and refund from one list ───────────────────
interface TicketRow {
  fiscalEventId: number
  ticketNumber: number
  tableNumber: number | null
  totalCents: number
  createdAt: string
  kind: string
  /** Present when the ticket can be copied (listCopyableTickets). */
  copyable: ipc.CopyableTicket | null
  /** Present when the ticket can be refunded (listRefundableEvents). */
  refundable: RefundableEvent | null
  /** Present when the sale can go on an invoice (listInvoiceableSales). */
  invoiceable: ipc.InvoiceableSale | null
}

const copyTickets = ref<ipc.CopyableTicket[]>([])
const refundEvents = ref<RefundableEvent[]>([])
const ticketsLoaded = ref(false)

async function loadTickets() {
  const [copyable, refundable] = await Promise.allSettled([
    ipc.listCopyableTickets(),
    ipc.listRefundableEvents(20),
    invoice.loadSales(),
    invoice.loadInvoices(),
  ])
  if (copyable.status === 'fulfilled') {
    copyTickets.value = copyable.value
  } else {
    copyTickets.value = []
    toast.show(describeError(copyable.reason), 'error', 5000)
  }
  refundEvents.value = refundable.status === 'fulfilled' ? refundable.value : []
  ticketsLoaded.value = true
  void loadCorrections()
}

// ── Payment correction (manager): one action per sale ────────────
/** The correction each sale already has, by fiscal event id. */
const corrections = ref<Map<number, ipc.CorrectionDone>>(new Map())
const correctionTarget = ref<TicketRow | null>(null)

/** A sale ticket (N, or T in training) the payment of which may be corrected. */
function isSaleRow(row: TicketRow): boolean {
  if (row.totalCents <= 0) return false
  if (row.copyable) return row.copyable.eventLabel === 'N' || row.copyable.eventLabel === 'T'
  return row.refundable !== null
}

/** Best effort: which sales were corrected, to mark them. */
async function loadCorrections() {
  if (!auth.isManager) return
  const next = new Map<number, ipc.CorrectionDone>()
  await Promise.allSettled(
    ticketRows.value.filter(isSaleRow).map(async (row) => {
      const payments = await ipc.listSalePayments(row.fiscalEventId)
      if (payments.corrected) next.set(row.fiscalEventId, payments.corrected)
    }),
  )
  corrections.value = next
}

function correctionText(c: ipc.CorrectionDone): string {
  const name = (type: string) => {
    if (type === 'CASH') return t('reports_pc_method_cash')
    if (type.startsWith('CARD')) return t('reports_pc_method_card')
    if (type === 'CHEQUE_MEAL') return t('reports_pc_method_meal_voucher')
    return type
  }
  return t('reports_pc_corrected', { from: name(c.fromType), to: name(c.toType) })
}

function onCorrected() {
  correctionTarget.value = null
  toast.show(t('reports_pc_done'), 'success')
  loadTickets()
}

watch(tab, (next) => {
  if (next === 'tickets') loadTickets()
})

const ticketRows = computed<TicketRow[]>(() => {
  const rows = new Map<number, TicketRow>()
  for (const tk of copyTickets.value) {
    rows.set(tk.fiscalEventId, {
      fiscalEventId: tk.fiscalEventId,
      ticketNumber: tk.ticketNumber,
      tableNumber: tk.tableNumber,
      totalCents: tk.totalCents,
      createdAt: tk.createdAt,
      kind: copyKind(tk),
      copyable: tk,
      refundable: null,
      invoiceable: null,
    })
  }
  for (const ev of refundEvents.value) {
    const row = rows.get(ev.fiscalEventId)
    if (row) {
      row.refundable = ev
    } else {
      rows.set(ev.fiscalEventId, {
        fiscalEventId: ev.fiscalEventId,
        ticketNumber: ev.ticketNumber,
        tableNumber: ev.tableNumber,
        totalCents: ev.totalCents,
        createdAt: ev.createdAt,
        kind: t('copy_kind_sale'),
        copyable: null,
        refundable: ev,
        invoiceable: null,
      })
    }
  }
  for (const sale of invoice.sales) {
    const row = rows.get(sale.fiscalEventId)
    if (row) {
      row.invoiceable = sale
    } else {
      rows.set(sale.fiscalEventId, {
        fiscalEventId: sale.fiscalEventId,
        ticketNumber: sale.ticketNumber,
        tableNumber: null,
        totalCents: sale.totalCents,
        createdAt: sale.posDateTime,
        kind: sale.isRefund ? t('copy_kind_refund') : t('copy_kind_sale'),
        copyable: null,
        refundable: null,
        invoiceable: sale,
      })
    }
  }
  // Fiscal event ids grow with time: newest first.
  return [...rows.values()].sort((a, b) => b.fiscalEventId - a.fiscalEventId)
})

// ── Refund flow ──────────────────────────────────────────────────
const refundOpen = ref(false)
const refundStep = ref<'items' | 'delivery' | 'processing' | 'result'>('items')
const refundSelected = ref<RefundableEvent | null>(null)
const refundLines = ref<OrderLine[]>([])
const refundChecked = ref<Map<number, number>>(new Map())
const refundResult = ref<CheckoutResult | null>(null)

async function openRefund(ev: RefundableEvent) {
  refundSelected.value = ev
  refundChecked.value = new Map()
  refundResult.value = null
  try {
    refundLines.value = await ipc.listOrderLines(ev.orderId)
  } catch {
    refundLines.value = []
  }
  refundStep.value = 'items'
  refundOpen.value = true
}

function closeRefund() {
  const signed = refundStep.value === 'result'
  refundOpen.value = false
  refundResult.value = null
  delivery.reset()
  if (signed) loadTickets()
}

function toggleRefundLine(line: OrderLine) {
  const map = new Map(refundChecked.value)
  if (map.has(line.id)) {
    map.delete(line.id)
  } else {
    map.set(line.id, line.quantity)
  }
  refundChecked.value = map
}

function adjustRefundQty(lineId: number, delta: number) {
  const map = new Map(refundChecked.value)
  const current = map.get(lineId) ?? 0
  const line = refundLines.value.find((l) => l.id === lineId)
  if (!line) return
  const next = Math.max(1, Math.min(line.quantity, current + delta))
  map.set(lineId, next)
  refundChecked.value = map
}

function refundTotalCents(): number {
  let total = 0
  for (const [lineId, qty] of refundChecked.value) {
    const line = refundLines.value.find((l) => l.id === lineId)
    if (line) total += line.unit_price_cents * qty
  }
  // Paid back in cash: rounded to 5 cents, like the signed refund.
  return round5(total)
}

function goToRefundDelivery() {
  if (refundChecked.value.size === 0) return
  delivery.reset()
  delivery.refreshMailStatus()
  refundStep.value = 'delivery'
}

async function confirmRefund() {
  const insz = auth.currentUser?.insz
  const ev = refundSelected.value
  if (!insz || !ev || refundChecked.value.size === 0) return

  const items: RefundItem[] = []
  for (const [lineId, qty] of refundChecked.value) {
    const line = refundLines.value.find((l) => l.id === lineId)
    if (line) {
      items.push({
        menuItemId: line.menu_item_id,
        quantity: qty,
        lineId: line.id,
      })
    }
  }

  refundStep.value = 'processing'
  try {
    const req = delivery.request.medium === 'paper' ? undefined : delivery.request
    const result = await ipc.refundCheckout(ev.fiscalEventId, items, insz, req)
    refundResult.value = result
    if (result.success) {
      toast.show(t('refund_success'), 'success')
    } else {
      toast.show(describeError(result.error), 'error')
    }
    refundStep.value = 'result'
  } catch (e) {
    toast.show(describeError(e), 'error')
    refundStep.value = 'items'
  }
}

function formatRefundTime(iso: string): string {
  return formatTimestamp(iso, formatTime)
}
</script>

<template>
  <div class="reports">
    <AppHeader />
    <div class="content">
      <div class="tabs" role="tablist">
        <button
          v-for="item in TABS"
          :key="item.id"
          class="tab"
          :class="{ active: tab === item.id }"
          role="tab"
          :aria-selected="tab === item.id"
          @click="tab = item.id"
        >
          {{ t(item.label) }}
        </button>
      </div>

      <!-- ── Day: X, user X, Z reprint, Z close ─────────────────── -->
      <section v-if="tab === 'day'" class="tab-body day">
        <div v-if="zPending" class="z-notice" role="status">{{ t('reports_z_pending') }}</div>

        <div class="report-cards">
          <button class="report-card x-btn" :disabled="loading" @click="generateX">
            <span v-if="loadingKind === 'x'" class="spinner card-spinner" aria-hidden="true"></span>
            <span class="card-title">{{ t('report_x') }}</span>
            <span class="card-sub">{{ t('report_x_desc') }}</span>
          </button>
          <button class="report-card x-btn user-x-btn" :disabled="loading" @click="generateUserX">
            <span
              v-if="loadingKind === 'userx'"
              class="spinner card-spinner"
              aria-hidden="true"
            ></span>
            <span class="card-title">{{ t('report_user_x') }}</span>
            <span class="card-sub">{{ t('report_user_x_desc') }}</span>
          </button>
          <button class="report-card z-history-btn" @click="openZList">
            <span class="card-title">{{ t('reports_reprint_z') }}</span>
            <span class="card-sub">
              <template v-if="lastZ">{{
                t('reports_last_z', { no: lastZ.sequenceNumber, date: formatDate(lastZ.createdAt) })
              }}</template>
              <template v-else>{{ t('report_no_history') }}</template>
            </span>
          </button>
        </div>

        <div class="spacer"></div>

        <div class="z-bar">
          <div v-if="unsettled.length" class="z-unsettled" role="status" data-testid="z-unsettled">
            <p class="z-unsettled-title">{{ t('reports_z_unsettled') }}</p>
            <ul>
              <li v-for="p in unsettled" :key="p.payment_id">{{ unsettledLine(p) }}</li>
            </ul>
            <p class="z-unsettled-note">{{ t('reports_z_unsettled_note') }}</p>
          </div>
          <p class="z-explain">{{ t('reports_z_explain') }}</p>
          <button class="z-btn" :class="{ armed: zArmed }" :disabled="loading" @click="generateZ">
            <span v-if="loadingKind === 'z'" class="spinner" aria-hidden="true"></span>
            <span v-else>{{ zArmed ? t('confirm_z') : t('reports_z_close') }}</span>
          </button>
        </div>
      </section>

      <!-- ── Cash: direction, reason, amount, summary ───────────── -->
      <section v-else-if="tab === 'cash'" class="tab-body cash">
        <div class="panel cash-form">
          <div class="step-label">{{ t('reports_step_direction') }}</div>
          <div class="segmented two">
            <button
              class="seg cash-dir cash-dir-in"
              :class="{ active: cashDirection === 'in' }"
              @click="setCashDirection('in')"
            >
              {{ t('reports_dir_in') }}
            </button>
            <button
              class="seg cash-dir cash-dir-out"
              :class="{ active: cashDirection === 'out' }"
              @click="setCashDirection('out')"
            >
              {{ t('reports_dir_out') }}
            </button>
          </div>

          <div class="step-label">{{ t('reports_step_reason') }}</div>
          <div class="reason-grid">
            <button
              v-for="key in cashReasons"
              :key="key"
              class="choice cash-reason"
              :class="{ active: cashReason === key }"
              :data-reason="key"
              @click="cashReason = key"
            >
              {{ reasonLabel(key) }}
            </button>
          </div>

          <div class="step-label">{{ t('reports_step_amount') }}</div>
          <div class="amount-grid">
            <button
              v-for="cents in cashPresets"
              :key="cents"
              class="choice mono cash-preset"
              :class="{ active: cashAmountCents === cents }"
              @click="cashAmountCents = cents"
            >
              {{ formatPrice(cents).replace(/,00$/, '') }} €
            </button>
            <button
              class="choice cash-other"
              :class="{ active: cashCustom }"
              @click="otherAmountOpen = true"
            >
              {{ t('other') }}
            </button>
          </div>
        </div>

        <div class="panel cash-summary">
          <h2>{{ t('reports_summary') }}</h2>
          <div class="summary-line">{{ cashSummary }}</div>
          <div class="summary-amount" :class="{ empty: cashAmountCents <= 0 }">
            {{ formatPrice(cashAmountCents) }} €
          </div>
          <div class="spacer"></div>
          <div class="drawer-actions">
            <button class="secondary-btn drawer-btn" @click="handleOpenDrawer">
              {{ t('reports_open_drawer') }}
            </button>
            <button
              class="secondary-btn declaration-btn"
              :disabled="cashLoading"
              @click="declarationOpen = true"
            >
              {{ t('reports_declare_drawer') }}
            </button>
          </div>
          <button
            class="cash-submit"
            :disabled="cashLoading || cashAmountCents <= 0 || !cashReason"
            @click="handleCashInOut"
          >
            {{ t(cashDirection === 'in' ? 'reports_save_in' : 'reports_save_out') }}
          </button>
        </div>
      </section>

      <!-- ── Tickets: copy and refund from one list ─────────────── -->
      <SalesReportPanel v-else-if="tab === 'sales'" />
      <GiftVouchersReport v-else-if="tab === 'vouchers'" />

      <section v-else class="tab-body tickets">
        <div class="panel tickets-panel">
          <div class="tickets-head">
            <h2>{{ t('reports_tickets_title') }}</h2>
            <p class="tickets-hint">
              {{ t('reports_tickets_hint') }} {{ t('invoice_select_hint') }}
            </p>
            <button
              class="invoice-btn"
              data-testid="invoice-action"
              :disabled="invoice.selected.length === 0"
              @click="invoice.openDialog()"
            >
              {{
                invoice.selected.length > 0
                  ? t('invoice_action_count', { n: invoice.selected.length })
                  : t('invoice_action')
              }}
            </button>
          </div>
          <p v-if="ticketsLoaded && ticketRows.length === 0" class="tickets-empty">
            {{ t('copy_none') }}
          </p>
          <ul v-else class="ticket-list">
            <li
              v-for="row in ticketRows"
              :key="row.fiscalEventId"
              class="ticket-row"
              :data-testid="`ticket-row-${row.fiscalEventId}`"
            >
              <span class="rt-check">
                <button
                  v-if="row.invoiceable && !row.invoiceable.invoicedBy"
                  class="check-box"
                  :class="{ on: invoice.selected.includes(row.fiscalEventId) }"
                  role="checkbox"
                  :aria-checked="invoice.selected.includes(row.fiscalEventId)"
                  :aria-label="t('invoice_select_row', { no: row.ticketNumber })"
                  :data-testid="`ticket-check-${row.fiscalEventId}`"
                  @click="invoice.toggleSale(row.fiscalEventId)"
                >
                  {{ invoice.selected.includes(row.fiscalEventId) ? '&#10003;' : '' }}
                </button>
              </span>
              <span class="rt-num">#{{ row.ticketNumber }}</span>
              <span class="rt-kind">
                {{ row.kind }}
                <span
                  v-if="row.invoiceable?.invoicedBy"
                  class="rt-invoiced"
                  :data-testid="`ticket-invoiced-${row.fiscalEventId}`"
                  >{{ t('invoice_invoiced_by', { no: row.invoiceable.invoicedBy }) }}</span
                >
              </span>
              <span class="rt-table">{{ row.tableNumber ? `T${row.tableNumber}` : '' }}</span>
              <span class="rt-time">{{ formatRefundTime(row.createdAt) }}</span>
              <span class="rt-total" :class="{ negative: row.totalCents < 0 }"
                >{{ formatPrice(row.totalCents) }} €</span
              >
              <span class="row-actions">
                <button
                  v-if="row.copyable"
                  class="copy-ticket-btn"
                  @click="openCopy(row.copyable, 'ticket')"
                >
                  {{ t('reports_copy') }}
                </button>
                <button
                  v-if="row.refundable"
                  class="refund-btn"
                  @click="openRefund(row.refundable)"
                >
                  {{ t('refund_confirm') }}
                </button>
                <span
                  v-if="auth.isManager && corrections.get(row.fiscalEventId)"
                  class="corrected-mark"
                  :data-testid="`corrected-${row.fiscalEventId}`"
                  >{{ correctionText(corrections.get(row.fiscalEventId)!) }}</span
                >
                <button
                  v-else-if="auth.isManager && isSaleRow(row)"
                  class="correct-btn"
                  :data-testid="`correct-payment-${row.fiscalEventId}`"
                  @click="correctionTarget = row"
                >
                  {{ t('reports_pc_action') }}
                </button>
              </span>
            </li>
          </ul>
        </div>
        <InvoiceList v-if="invoice.invoices.length > 0" />
      </section>
    </div>

    <!-- Payment correction (event F): lines, new method, PIN -->
    <Teleport to="body">
      <PaymentCorrectionDialog
        v-if="correctionTarget"
        :fiscal-event-id="correctionTarget.fiscalEventId"
        :ticket-number="correctionTarget.ticketNumber"
        @done="onCorrected"
        @cancel="correctionTarget = null"
      />
    </Teleport>
    <InvoiceDialog />

    <!-- Z history: pick the report to reprint -->
    <Teleport to="body">
      <div v-if="zListOpen" class="refund-overlay" @click.self="zListOpen = false">
        <div class="refund-modal z-list">
          <h3>{{ t('report_history_title') }}</h3>
          <p v-if="zHistory.length === 0" class="empty">{{ t('report_no_history') }}</p>
          <ul v-else class="history-list">
            <li v-for="z in zHistory" :key="z.id" class="history-row">
              <span class="history-label"
                >Z #{{ z.sequenceNumber }} · {{ formatDate(z.createdAt) }}</span
              >
              <button v-if="z.fiscalEventId" class="reprint-btn" @click="reprintZ(z)">
                {{ t('report_reprint') }}
              </button>
            </li>
          </ul>
          <button class="refund-cancel" @click="zListOpen = false">{{ t('back') }}</button>
        </div>
      </div>
    </Teleport>

    <!-- Refund modal overlay (§8.2) -->
    <Teleport to="body">
      <div v-if="refundOpen" class="refund-overlay" @click.self="closeRefund">
        <div class="refund-modal">
          <template v-if="refundStep === 'items'">
            <h3>
              {{ t('refund_select_items')
              }}<span v-if="refundSelected" class="modal-ticket">
                · #{{ refundSelected.ticketNumber }}</span
              >
            </h3>
            <div class="refund-lines">
              <div
                v-for="line in refundLines"
                :key="line.id"
                class="refund-line"
                :class="{ selected: refundChecked.has(line.id) }"
                @click="toggleRefundLine(line)"
              >
                <div class="rl-info">
                  <span class="rl-name">{{ line.name }}</span>
                  <span class="rl-price">{{
                    formatPrice(line.unit_price_cents * line.quantity)
                  }}</span>
                </div>
                <div v-if="refundChecked.has(line.id)" class="rl-qty">
                  <button class="rl-qty-btn" @click.stop="adjustRefundQty(line.id, -1)">-</button>
                  <span>{{ refundChecked.get(line.id) }}</span>
                  <button class="rl-qty-btn" @click.stop="adjustRefundQty(line.id, 1)">+</button>
                  <span class="rl-max">/ {{ line.quantity }}</span>
                </div>
                <div v-else class="rl-orig-qty">{{ line.quantity }}x</div>
              </div>
            </div>
            <button
              class="refund-confirm-btn"
              :disabled="refundChecked.size === 0"
              @click="goToRefundDelivery"
            >
              {{ t('refund_confirm') }} {{ formatPrice(refundTotalCents()) }}
            </button>
            <button class="refund-cancel" @click="closeRefund">{{ t('back') }}</button>
          </template>

          <template v-else-if="refundStep === 'delivery'">
            <h3>{{ t('refund_confirm') }}</h3>
            <TicketMediumPicker />
            <button class="refund-confirm-btn" @click="confirmRefund">
              {{ t('refund_confirm') }} {{ formatPrice(refundTotalCents()) }}
            </button>
            <button class="refund-cancel" @click="refundStep = 'items'">{{ t('back') }}</button>
          </template>

          <template v-else-if="refundStep === 'processing'">
            <div class="refund-processing">
              <div class="spinner"></div>
              <p>{{ t('processing_fdm') }}</p>
            </div>
          </template>

          <template v-else>
            <DeliveryOutcomePanel
              :outcome="refundResult?.delivery ?? null"
              :fiscal-event-id="refundResult?.fiscal_event_id ?? null"
            />
            <p v-if="refundResult && !refundResult.success" class="refund-error">
              {{ describeError(refundResult.error) }}
            </p>
            <button class="refund-cancel" @click="closeRefund">{{ t('finish') }}</button>
          </template>
        </div>
      </div>
    </Teleport>

    <!-- Copy modal (event C) - picker shown before signing (§8.2) -->
    <Teleport to="body">
      <div v-if="copyOpen" class="refund-overlay" @click.self="closeCopy">
        <div class="refund-modal">
          <template v-if="copyStep === 'picker'">
            <h3>{{ copyKindOf === 'ticket' ? t('copy_ticket') : t('report_reprint') }}</h3>
            <TicketMediumPicker />
            <button class="refund-confirm-btn copy-confirm-btn" @click="confirmCopy">
              {{ t('validate') }}
            </button>
            <button class="refund-cancel" @click="closeCopy">{{ t('back') }}</button>
          </template>

          <template v-else-if="copyStep === 'processing'">
            <div class="refund-processing">
              <div class="spinner"></div>
              <p>{{ t('processing_fdm') }}</p>
            </div>
          </template>

          <template v-else>
            <DeliveryOutcomePanel
              :outcome="copyResult?.delivery ?? null"
              :fiscal-event-id="copyResult?.fiscalEventId ?? null"
            />
            <p v-if="copyResult && !copyResult.success" class="refund-error">
              {{ describeError(copyResult.error) }}
            </p>
            <button class="refund-cancel" @click="closeCopy">{{ t('finish') }}</button>
          </template>
        </div>
      </div>
    </Teleport>

    <PriceDialog
      v-if="declarationOpen"
      :title="t('cash_declaration')"
      :item-name="t('cash_declaration_hint')"
      allow-zero
      @confirm="handleDeclaration"
      @cancel="declarationOpen = false"
    />

    <PriceDialog
      v-if="otherAmountOpen"
      :title="t('reports_other_amount')"
      :item-name="cashSummary"
      @confirm="handleOtherAmount"
      @cancel="otherAmountOpen = false"
    />
  </div>
</template>

<style scoped>
.reports {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  background: var(--bg-primary);
}

.content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 24px;
  padding: 24px 32px;
  overflow: hidden;
}

/* ── Tabs (segmented control) ─────────────────────────────── */
.tabs {
  width: 900px;
  max-width: 100%;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 4px;
  padding: 4px;
  background: var(--bg-surface);
  border-radius: 12px;
  flex-shrink: 0;
}
.tab {
  height: 64px;
  border-radius: 8px;
  font-size: 20px;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
  background: transparent;
  transition: transform var(--tap-duration) ease;
}
.tab.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.tab:active {
  transform: scale(var(--tap-scale));
}

.tab-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 24px;
}
.spacer {
  flex: 1;
}

.panel {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 0;
}
.panel h2 {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}

.mono {
  font-family: var(--font-mono);
}

/* ── Day ──────────────────────────────────────────────────── */
.z-notice {
  padding: 14px 20px;
  border-radius: var(--radius-md);
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
}

/* Charged cards waiting for their sale: told, never blocking. */
.z-unsettled {
  padding: 14px 20px;
  border-radius: var(--radius-md);
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-size: 18px;
}
.z-unsettled-title {
  font-weight: var(--font-weight-bold);
}
.z-unsettled ul {
  margin: 6px 0;
  padding-left: 22px;
  font-family: var(--font-mono);
}
.z-unsettled-note {
  font-size: 16px;
}

.report-cards {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
}
.report-card {
  position: relative;
  min-height: 180px;
  padding: 28px;
  border-radius: var(--radius-lg);
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  color: var(--text-primary);
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: flex-end;
  gap: 6px;
  text-align: left;
  transition: transform var(--tap-duration) ease;
}
.report-card:active {
  transform: scale(var(--tap-scale));
}
.report-card:disabled {
  opacity: 0.6;
}
.card-title {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}
.card-sub {
  font-size: 17px;
  color: var(--text-secondary);
}
.card-spinner {
  position: absolute;
  top: 24px;
  right: 24px;
}

.z-bar {
  display: flex;
  align-items: center;
  gap: 32px;
  padding-top: 24px;
  border-top: 1px solid var(--border);
}
.z-explain {
  flex: 1;
  font-size: 18px;
  color: var(--text-secondary);
  text-wrap: pretty;
}
.z-btn {
  width: 560px;
  height: 96px;
  flex-shrink: 0;
  border-radius: 12px;
  border: 2px solid var(--accent-danger);
  background: transparent;
  color: var(--text-primary);
  font-size: 24px;
  font-weight: var(--font-weight-heavy);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: transform var(--tap-duration) ease;
}
.z-btn:active {
  transform: scale(var(--tap-scale));
}
.z-btn:disabled {
  opacity: 0.6;
}
/* Armed: filled red, the next tap closes the day. */
.z-btn.armed {
  background: var(--accent-danger);
  color: var(--text-on-accent);
}

.spinner {
  width: 28px;
  height: 28px;
  border: 3px solid var(--border);
  border-top-color: currentColor;
  border-radius: 50%;
  animation: report-spin 800ms linear infinite;
}
@keyframes report-spin {
  to {
    transform: rotate(360deg);
  }
}

/* ── Cash ─────────────────────────────────────────────────── */
.cash {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 670px;
  gap: 24px;
}

.step-label {
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.segmented {
  display: grid;
  gap: 4px;
  padding: 4px;
  background: var(--bg-surface);
  border-radius: 12px;
}
.segmented.two {
  grid-template-columns: repeat(2, 1fr);
}
.seg {
  height: 64px;
  border-radius: 8px;
  font-size: 20px;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
  background: transparent;
  transition: transform var(--tap-duration) ease;
}
.seg.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.seg:active {
  transform: scale(var(--tap-scale));
}

.reason-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}
.amount-grid {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 8px;
}
.choice {
  height: 72px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 20px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.choice.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.choice:active {
  transform: scale(var(--tap-scale));
}

.summary-line {
  font-size: 22px;
  line-height: 1.5;
}
.summary-amount {
  font-family: var(--font-mono);
  font-size: 56px;
  font-weight: var(--font-weight-bold);
  line-height: 1;
}
.summary-amount.empty {
  color: var(--text-secondary);
}

.drawer-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.secondary-btn {
  height: 64px;
  padding: 0 20px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.secondary-btn:active {
  transform: scale(var(--tap-scale));
}
.secondary-btn:disabled {
  opacity: 0.4;
}

.cash-submit {
  height: 80px;
  padding: 0 28px;
  border-radius: var(--radius-md);
  background: var(--accent-primary);
  color: var(--text-on-accent);
  font-size: 22px;
  font-weight: var(--font-weight-heavy);
  letter-spacing: 0.02em;
  transition: transform var(--tap-duration) ease;
}
.cash-submit:active {
  transform: scale(var(--tap-scale));
}
.cash-submit:disabled {
  opacity: 0.4;
}

/* ── Tickets ──────────────────────────────────────────────── */
.tickets-panel {
  flex: 3;
}
.tickets-head {
  display: flex;
  align-items: baseline;
  gap: 24px;
}
.tickets-hint {
  font-size: 17px;
  color: var(--text-secondary);
}
.tickets-empty {
  font-size: 18px;
  color: var(--text-secondary);
  padding: 32px 0;
  text-align: center;
}

.ticket-list {
  list-style: none;
  margin: 0;
  padding: 0;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
  display: flex;
  flex-direction: column;
}
.ticket-row {
  display: grid;
  grid-template-columns: 56px 120px 200px 80px 100px minmax(0, 1fr) auto;
  align-items: center;
  gap: 16px;
  min-height: 80px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}
.ticket-row:last-child {
  border-bottom: none;
}
.rt-num {
  font-family: var(--font-mono);
  font-size: 22px;
  font-weight: var(--font-weight-heavy);
}
.rt-kind {
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
.rt-table {
  font-family: var(--font-mono);
  font-size: 17px;
  font-weight: var(--font-weight-bold);
}
.rt-time {
  font-family: var(--font-mono);
  font-size: 17px;
  color: var(--text-secondary);
}
.rt-total {
  font-family: var(--font-mono);
  font-size: 22px;
  font-weight: var(--font-weight-bold);
  text-align: right;
}
.rt-total.negative {
  color: var(--accent-danger);
}
.rt-check {
  display: flex;
  align-items: center;
  justify-content: center;
}
.check-box {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--selected-fg);
  border: 2px solid var(--text-secondary);
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform var(--tap-duration) ease;
}
.check-box.on {
  background: var(--selected-bg);
  border-color: var(--selected-bg);
}
.check-box:active {
  transform: scale(var(--tap-scale));
}
.rt-invoiced {
  display: block;
  font-family: var(--font-mono);
  font-size: 14px;
  color: var(--accent-info);
}
.tickets-head {
  align-items: center;
}
.invoice-btn {
  margin-left: auto;
  height: 64px;
  min-width: 200px;
  padding: 0 24px;
  border-radius: var(--radius-md);
  background: var(--accent-primary);
  color: var(--text-on-accent);
  font-size: 20px;
  font-weight: var(--font-weight-heavy);
  transition: transform var(--tap-duration) ease;
}
.invoice-btn:active {
  transform: scale(var(--tap-scale));
}
.invoice-btn:disabled {
  opacity: 0.3;
  pointer-events: none;
}
.row-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  min-width: 368px;
  flex-wrap: wrap;
}
.copy-ticket-btn,
.correct-btn,
.refund-btn {
  height: 64px;
  min-width: 180px;
  padding: 0 20px;
  border-radius: var(--radius-md);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.copy-ticket-btn {
  background: var(--bg-surface);
  color: var(--text-primary);
}
.refund-btn {
  background: transparent;
  color: var(--text-primary);
  border: 2px solid var(--accent-danger);
}
.correct-btn {
  background: transparent;
  color: var(--text-primary);
  border: 2px solid var(--border);
}
.corrected-mark {
  align-self: center;
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
  max-width: 260px;
  text-align: right;
}
.copy-ticket-btn:active,
.correct-btn:active,
.refund-btn:active {
  transform: scale(var(--tap-scale));
}

/* ── Modals (Z history, refund, copy) ─────────────────────── */
.refund-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
}
.refund-modal {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 24px;
  width: 90vw;
  max-width: 640px;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
}
.refund-modal h3 {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}
.modal-ticket {
  font-family: var(--font-mono);
  color: var(--text-secondary);
}
.refund-error {
  color: var(--accent-danger);
  font-size: var(--font-size-md);
  text-align: center;
}

.empty {
  font-size: 18px;
  color: var(--text-secondary);
  padding: 16px 0;
}
.history-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.history-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 8px 8px 8px 20px;
  background: var(--bg-surface);
  border-radius: var(--radius-md);
}
.history-label {
  font-family: var(--font-mono);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
}
.reprint-btn {
  height: 56px;
  padding: 0 24px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.reprint-btn:active {
  transform: scale(var(--tap-scale));
}

.refund-lines {
  display: flex;
  flex-direction: column;
}
.refund-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 64px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
.refund-line.selected {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.refund-line.selected .rl-price,
.refund-line.selected .rl-max {
  color: var(--selected-fg);
}
.rl-info {
  flex: 1;
}
.rl-name {
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  display: block;
}
.rl-price {
  font-family: var(--font-mono);
  font-size: 15px;
  color: var(--text-secondary);
}
.rl-orig-qty {
  font-family: var(--font-mono);
  font-size: 17px;
  color: var(--text-secondary);
  min-width: 40px;
  text-align: right;
}
.rl-qty {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: var(--font-mono);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
}
.rl-qty-btn {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform var(--tap-duration) ease;
}
.rl-qty-btn:active {
  transform: scale(var(--tap-scale));
}
.rl-max {
  font-size: 14px;
  color: var(--text-secondary);
}

/* The confirming action of each modal. */
.refund-confirm-btn {
  width: 100%;
  height: 72px;
  flex-shrink: 0;
  font-size: 22px;
  font-weight: var(--font-weight-heavy);
  border-radius: var(--radius-md);
  background: var(--accent-primary);
  color: var(--text-on-accent);
  transition: transform var(--tap-duration) ease;
  font-variant-numeric: tabular-nums;
}
.refund-confirm-btn:active {
  transform: scale(var(--tap-scale));
}
.refund-confirm-btn:disabled {
  opacity: 0.3;
  pointer-events: none;
}

.refund-cancel {
  width: 100%;
  height: 56px;
  flex-shrink: 0;
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-secondary);
  transition: transform var(--tap-duration) ease;
}
.refund-cancel:active {
  transform: scale(var(--tap-scale));
}

.refund-processing {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
  padding: 32px 0;
}
.refund-processing .spinner {
  width: 48px;
  height: 48px;
  border: 4px solid var(--bg-surface);
  border-top: 4px solid var(--accent-primary);
}
.refund-processing p {
  color: var(--text-secondary);
  font-size: var(--font-size-md);
}
</style>
