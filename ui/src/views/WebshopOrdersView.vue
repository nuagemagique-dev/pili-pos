<script setup lang="ts">
// WebshopOrdersView - HubRise-origin orders (own webshop, Deliveroo,
// Uber Eats, Takeaway.com, …) laid out as one column per stage
// (UX Guide §2.1): the column tells the state, so cards carry no
// status chip and at most two or three actions.

import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { usePosStore } from '../stores/pos'
import { useWebshopOrdersStore } from '../stores/webshopOrders'
import { useI18n } from '../i18n'
import { formatPrice } from '../utils/format'
import { formatTime, parseTimestamp } from '../utils/datetime'
import { useBilingualName } from '../utils/names'
import AppHeader from '../components/AppHeader.vue'
import type { WebshopOrderItem, WebshopOrderSummary } from '../api/ipc'
// Bundled through this view only, so builds without HubRise ship none
// of the aggregator logos.
import deliverooLogo from '../assets/webshop/logos/deliveroo.svg'
import uberEatsLogo from '../assets/webshop/logos/uber-eats.svg'
import takeawayLogo from '../assets/webshop/logos/takeaway.svg'

const router = useRouter()
const auth = useAuthStore()
const store = useWebshopOrdersStore()
const { t } = useI18n()
const bilingual = useBilingualName()
const pos = usePosStore()
// The Chinese name gets the Chinese font, whichever line it is on.
const uiIsZh = computed(() => pos.language === 'zh')

// Ticking "now" for the "il y a N min" age of new orders.
const now = ref(Date.now())
let nowTimer: ReturnType<typeof setInterval> | undefined

// ── Stages (one column each) ─────────────────────────────────────
// new      : new / received                 → Accepter / Refuser
// kitchen  : accepted / in_preparation      → Préparer, then Prête
// ready    : awaiting_collection / in_delivery → En livraison / Terminer
// done     : completed / rejected / cancelled (behind "Terminées")
type Stage = 'new' | 'kitchen' | 'ready' | 'done'

function statusOf(order: WebshopOrderSummary): string {
  return order.hubrise_status ?? 'new'
}

/** The order's fiscal state, `pending` before acceptance. */
function fiscalOf(order: WebshopOrderSummary): string {
  return order.webshop?.fiscal_state ?? 'pending'
}

function stageOf(order: WebshopOrderSummary): Stage {
  // A signed order HubRise cancelled waits for staff where they look
  // first; refunded and closed ones are done.
  switch (fiscalOf(order)) {
    case 'refund_pending':
    case 'cancel_pending':
      return 'new'
    case 'refunded':
    case 'closed':
    case 'voided':
    case 'rejected':
      return 'done'
  }
  switch (statusOf(order)) {
    case 'accepted':
    case 'in_preparation':
      return 'kitchen'
    case 'awaiting_collection':
    case 'in_delivery':
      return 'ready'
    case 'completed':
    case 'rejected':
    case 'cancelled':
      return 'done'
    default:
      // new, received, and any status we don't know yet: surface it
      // where the staff looks first rather than hiding it.
      return 'new'
  }
}

const byStage = computed(() => {
  const groups: Record<Stage, WebshopOrderSummary[]> = { new: [], kitchen: [], ready: [], done: [] }
  for (const o of store.orders) groups[stageOf(o)].push(o)
  return groups
})

// The third column shows either the ready orders or, on demand, the
// finished ones (completed / rejected / cancelled).
const showDone = ref(false)

const columns = computed(() => [
  { id: 'new' as const, title: t('webshop_col_new'), orders: byStage.value.new },
  { id: 'kitchen' as const, title: t('webshop_col_kitchen'), orders: byStage.value.kitchen },
  showDone.value
    ? { id: 'done' as const, title: t('webshop_filter_done'), orders: byStage.value.done }
    : { id: 'ready' as const, title: t('webshop_col_ready'), orders: byStage.value.ready },
])

// ── Waiting orders ───────────────────────────────────────────────
// An order nobody answered for 5 minutes raises the alarm: platforms
// cancel orders left unanswered.
const ALARM_MINUTES = 5

function waitingMinutes(order: WebshopOrderSummary): number | null {
  const created = parseTimestamp(order.created_at)
  if (!created) return null
  return Math.max(0, Math.floor((now.value - created.getTime()) / 60_000))
}

function isLate(order: WebshopOrderSummary): boolean {
  if (fiscalOf(order) !== 'pending') return false
  const s = statusOf(order)
  if (s !== 'new' && s !== 'received') return false
  return (waitingMinutes(order) ?? 0) >= ALARM_MINUTES
}

const lateCount = computed(() => store.orders.filter(isLate).length)

// ── Display helpers ──────────────────────────────────────────────
function formatEuros(cents: number): string {
  return formatPrice(cents) + ' €'
}

function formatClock(iso: string | null): string {
  if (!iso) return ''
  const d = parseTimestamp(iso)
  return d ? formatTime(d) : ''
}

function serviceTypeLabel(order: WebshopOrderSummary): string {
  switch (order.service_type) {
    case 'delivery':
      return t('webshop_service_delivery')
    case 'collection':
      return t('webshop_service_collection')
    case 'eat_in':
      return t('webshop_service_eat_in')
    default:
      return t('webshop_service_unknown')
  }
}

function metaLine(order: WebshopOrderSummary): string {
  return [serviceTypeLabel(order), order.customer_name].filter(Boolean).join(' · ')
}

function doneLabel(order: WebshopOrderSummary): string {
  if (fiscalOf(order) === 'refunded') return t('webshop_status_refunded')
  switch (statusOf(order)) {
    case 'rejected':
      return t('webshop_status_rejected')
    case 'cancelled':
      return t('webshop_status_cancelled')
    default:
      return t('webshop_status_completed')
  }
}

/**
 * The one time that matters for the card's stage, shown top right:
 * age for new orders (amber, they are waiting), target time in the
 * kitchen, ready time once ready.
 */
function keyTime(order: WebshopOrderSummary): { text: string; tone: 'alarm' | 'warn' | 'info' | 'plain' | 'muted' } | null {
  switch (stageOf(order)) {
    case 'new': {
      const minutes = waitingMinutes(order)
      if (minutes === null) return null
      return { text: t('webshop_time_ago', { n: minutes }), tone: isLate(order) ? 'alarm' : 'warn' }
    }
    case 'kitchen': {
      const at = formatClock(order.confirmed_time)
      return at ? { text: t('webshop_time_ready_at', { time: at }), tone: 'plain' } : null
    }
    case 'ready': {
      if (statusOf(order) === 'in_delivery') {
        return { text: t('webshop_status_in_delivery'), tone: 'info' }
      }
      const at = formatClock(order.confirmed_time)
      return at ? { text: t('webshop_time_ready', { time: at }), tone: 'plain' } : null
    }
    case 'done':
      return { text: doneLabel(order), tone: 'muted' }
  }
}

/**
 * Stripe colour per channel so Deliveroo / Uber Eats / Takeaway /
 * own webshop are told apart at a glance (UX Guide §6).
 */
function channelClass(order: WebshopOrderSummary): string {
  const src = order.source?.toLowerCase() ?? ''
  if (src.includes('deliveroo')) return 'ch-deliveroo'
  if (src.includes('uber')) return 'ch-uber'
  if (src.includes('takeaway') || src.includes('just')) return 'ch-takeaway'
  if (src.includes('web')) return 'ch-webshop'
  return 'ch-other'
}

/**
 * Brand logo for the order's channel, or null (the template then
 * falls back to a text label). Drop new files in
 * `ui/src/assets/webshop/logos/` and extend this mapping to wire them.
 */
function channelLogo(order: WebshopOrderSummary): string | null {
  const src = order.source?.toLowerCase() ?? ''
  if (src.includes('deliveroo')) return deliverooLogo
  if (src.includes('uber')) return uberEatsLogo
  if (src.includes('takeaway') || src.includes('just')) return takeawayLogo
  return null
}

/** The restaurant's own webshop (no brand logo: "WEBSHOP" label). */
function isOwnWebshop(order: WebshopOrderSummary): boolean {
  const src = order.source?.toLowerCase() ?? ''
  return src.includes('web')
}

/**
 * Full delivery address only on own-webshop delivery orders: the
 * restaurant's driver decides from the distance whether to accept.
 * Aggregators deliver themselves, so their cards stay uncluttered.
 */
function deliveryAddress(order: WebshopOrderSummary): string | null {
  if (!isOwnWebshop(order) || order.service_type !== 'delivery') return null
  const street = [order.customer_address_1, order.customer_address_2].filter(Boolean).join(' ').trim()
  const city = [order.customer_postal_code, order.customer_city].filter(Boolean).join(' ').trim()
  const text = [street, city].filter(Boolean).join(', ')
  return text || null
}

function itemName(item: WebshopOrderItem) {
  return bilingual({ name: item.product_name, name_fr: item.product_name, name_zh: item.product_name_zh })
}

// ── Actions ──────────────────────────────────────────────────────
//
// Most channels collapse `awaiting_collection → completed` in one
// button: an external driver or the customer takes the bag. Own-webshop
// delivery is driven by the restaurant, so it gets an explicit
// "En livraison" step between Prête and Terminer.
function nextAction(
  order: WebshopOrderSummary,
): { label: string; run: () => Promise<void> } | null {
  const id = order.hubrise_order_id
  if (!id) return null
  switch (fiscalOf(order)) {
    case 'refund_pending':
      return auth.isManager
        ? { label: t('webshop_action_refund'), run: () => store.refundOrder(order, auth.currentUser!.insz) }
        : null
    case 'cancel_pending':
      return { label: t('webshop_action_close_cancelled'), run: () => store.closeCancelledOrder(order) }
  }
  switch (statusOf(order)) {
    case 'new':
    case 'received':
      // An order to check is never signed: only Refuser or Resynchroniser.
      if (needsReview(order)) return null
      return { label: t('webshop_action_accept'), run: () => store.acceptOrder(order) }
    case 'accepted':
      return { label: t('webshop_action_prepare'), run: () => store.startPreparation(id) }
    case 'in_preparation':
      return { label: t('webshop_action_ready'), run: () => store.markReady(id) }
    case 'awaiting_collection':
      if (isOwnWebshop(order) && order.service_type === 'delivery') {
        return { label: t('webshop_action_out_for_delivery'), run: () => store.markOutForDelivery(id) }
      }
      return finishAction(order)
    case 'in_delivery':
      return finishAction(order)
    default:
      return null
  }
}

/** Handing over: an unpaid order is collected (its sale N), a prepaid one completed. */
function finishAction(order: WebshopOrderSummary): { label: string; run: () => Promise<void> } | null {
  if (isUnpaid(order)) {
    return { label: t('webshop_action_collect_cash'), run: () => store.collectOrder(order, 'cash') }
  }
  return { label: t('webshop_action_complete'), run: () => store.completeOrder(order) }
}

/** Waiting for its money: registered (P), not sold yet. */
function isUnpaid(order: WebshopOrderSummary): boolean {
  return fiscalOf(order) === 'signed_p'
}

/** Card payment, offered next to cash when collecting. */
function canCollectByCard(order: WebshopOrderSummary): boolean {
  return isUnpaid(order) && stageOf(order) === 'ready'
}

function needsReview(order: WebshopOrderSummary): boolean {
  return fiscalOf(order) === 'pending' && order.webshop?.resolution !== 'ok'
}

function reviewReasons(order: WebshopOrderSummary): string[] {
  const codes = new Set((order.webshop?.attention ?? []).map((a) => a.code))
  if (codes.size === 0) return [t('webshop_attention_unknown_product')]
  return [...codes].map((c) => t(`webshop_attention_${c}`))
}

function changedAfterSigning(order: WebshopOrderSummary): boolean {
  return fiscalOf(order) !== 'pending' &&
    (order.webshop?.attention ?? []).some((a) => a.code === 'changed_after_signing')
}

/** "Payée" / "À encaisser", once the payment is known. */
function paymentBadge(order: WebshopOrderSummary): { text: string; tone: 'paid' | 'due' } | null {
  const mode = order.webshop?.payment_mode
  if (fiscalOf(order) === 'signed_n' || mode === 'prepaid') return { text: t('webshop_badge_prepaid'), tone: 'paid' }
  if (mode === 'unpaid') return { text: t('webshop_badge_unpaid'), tone: 'due' }
  return null
}

/** The banner of a signed order HubRise cancelled. */
function cancelledBanner(order: WebshopOrderSummary): string | null {
  switch (fiscalOf(order)) {
    case 'refund_pending':
      return auth.isManager ? t('webshop_cancelled_refund') : t('webshop_manager_needed')
    case 'cancel_pending':
      return t('webshop_cancelled_close')
    default:
      return null
  }
}

function canReject(order: WebshopOrderSummary): boolean {
  const s = statusOf(order)
  return fiscalOf(order) === 'pending' && (s === 'new' || s === 'received')
}

/** Delaying the ready time only makes sense while the kitchen works on it. */
function canDelay(order: WebshopOrderSummary): boolean {
  const s = statusOf(order)
  return s === 'accepted' || s === 'in_preparation'
}

/** Reprints once the order is taken and not finished yet. */
function canReprint(order: WebshopOrderSummary): boolean {
  const stage = stageOf(order)
  return stage === 'kitchen' || stage === 'ready'
}

// +60 is a double tap on +30 when needed (UX Guide §2.1).
const delayChoices = [15, 30, 45]

// "Réimprimer" groups the kitchen and customer tickets: a first tap
// reveals both choices (2 taps, rare action - UX Guide §2.2).
const reprintOpenId = ref<number | null>(null)

function toggleReprint(order: WebshopOrderSummary) {
  reprintOpenId.value = reprintOpenId.value === order.id ? null : order.id
}

async function handlePrimary(order: WebshopOrderSummary) {
  const action = nextAction(order)
  if (action) await guarded(order, action.run)
}

// One action at a time per card: a sale is signed once.
const busyId = ref<number | null>(null)

async function guarded(order: WebshopOrderSummary, action: () => Promise<void>) {
  if (busyId.value !== null) return
  busyId.value = order.id
  try {
    await action()
  } catch {
    // The store showed the error.
  } finally {
    busyId.value = null
  }
}

async function handleReject(order: WebshopOrderSummary) {
  await guarded(order, () => store.rejectOrder(order))
}

async function handleCollectCard(order: WebshopOrderSummary) {
  await guarded(order, () => store.collectOrder(order, 'card'))
}

async function handleResync(order: WebshopOrderSummary) {
  await guarded(order, () => store.resync())
}

async function handleDelay(order: WebshopOrderSummary, minutes: number) {
  if (!order.hubrise_order_id) return
  await store.delayByMinutes(order.hubrise_order_id, minutes)
}

async function handlePrintKitchen(order: WebshopOrderSummary) {
  reprintOpenId.value = null
  await store.printKitchen(order.id)
}

async function handlePrintCustomer(order: WebshopOrderSummary) {
  reprintOpenId.value = null
  await store.printCustomer(order.id)
}

// ── Lifecycle ────────────────────────────────────────────────────
onMounted(async () => {
  if (!auth.currentUser) {
    router.push({ name: 'login' })
    return
  }
  nowTimer = setInterval(() => (now.value = Date.now()), 15_000)
  await store.attachListeners()
  await store.fetchOrders()
})

onUnmounted(() => clearInterval(nowTimer))
</script>

<template>
  <div class="webshop-view">
    <AppHeader />

    <p v-if="store.loading && store.orders.length === 0" class="state-msg">
      {{ t('webshop_loading') }}
    </p>
    <p v-else-if="store.error" class="state-msg state-error">{{ store.error }}</p>

    <p v-if="lateCount > 0" class="alarm" role="alert" data-testid="stale-alarm">
      {{ t('webshop_stale_alarm', { count: lateCount }) }}
    </p>

    <main class="board">
      <section
        v-for="col in columns"
        :key="col.id"
        class="column"
        :class="`column-${col.id}`"
      >
        <header class="column-head">
          <span class="column-dot" />
          <span class="column-title">{{ col.title }}</span>
          <span class="column-count">{{ col.orders.length }}</span>
          <button
            v-if="col.id === 'ready' || col.id === 'done'"
            class="done-toggle"
            :class="{ 'done-toggle-active': showDone }"
            @click="showDone = !showDone"
          >
            {{ t('webshop_filter_done') }} · <span class="mono">{{ byStage.done.length }}</span>
          </button>
        </header>

        <ul class="column-list">
          <li v-if="col.orders.length === 0" class="column-empty">
            {{ t('webshop_empty') }}
          </li>
          <li
            v-for="order in col.orders"
            :key="order.id"
            class="card"
            :class="[`card-${statusOf(order)}`, channelClass(order)]"
          >
            <header class="card-head">
              <img
                v-if="channelLogo(order)"
                :src="channelLogo(order)!"
                :alt="order.source"
                class="channel-logo"
              />
              <span v-else-if="isOwnWebshop(order)" class="webshop-badge">
                {{ t('webshop_own_label') }}
              </span>
              <span v-else class="card-source">{{ order.source }}</span>
              <span
                v-if="keyTime(order)"
                class="key-time"
                :class="`key-time-${keyTime(order)!.tone}`"
              >{{ keyTime(order)!.text }}</span>
            </header>

            <div v-if="metaLine(order) || paymentBadge(order)" class="card-meta">
              <span>{{ metaLine(order) }}</span>
              <span
                v-if="paymentBadge(order)"
                class="pay-badge"
                :class="`pay-badge-${paymentBadge(order)!.tone}`"
                data-testid="pay-badge"
              >{{ paymentBadge(order)!.text }}</span>
            </div>

            <div v-if="cancelledBanner(order)" class="fiscal-banner" data-testid="cancelled-banner">
              {{ cancelledBanner(order) }}
            </div>
            <div v-if="changedAfterSigning(order)" class="fiscal-banner" data-testid="changed-banner">
              {{ t('webshop_changed_after_signing') }}
            </div>

            <div v-if="needsReview(order)" class="review" data-testid="review">
              <span class="review-title">{{ t('webshop_review_title') }}</span>
              <ul class="review-reasons">
                <li v-for="reason in reviewReasons(order)" :key="reason">{{ reason }}</li>
              </ul>
            </div>

            <div v-if="deliveryAddress(order)" class="delivery-address">
              {{ deliveryAddress(order) }}
            </div>

            <ul v-if="order.items.length > 0" class="items">
              <li v-for="(item, idx) in order.items" :key="idx" class="item">
                <div class="item-main">
                  <span class="item-qty">{{ item.quantity }}×</span>
                  <span class="item-names">
                    <span class="item-name" :class="{ zh: uiIsZh }">
                      {{ itemName(item).primary }}
                    </span>
                    <span v-if="itemName(item).secondary" class="item-name-alt" :class="{ zh: !uiIsZh }">
                      {{ itemName(item).secondary }}
                    </span>
                  </span>
                </div>
                <div v-if="item.options.length > 0" class="item-options">
                  {{ item.options.join(' · ') }}
                </div>
                <div v-if="item.customer_notes" class="item-notes">
                  “{{ item.customer_notes }}”
                </div>
              </li>
            </ul>

            <p v-if="order.customer_notes" class="order-notes">“{{ order.customer_notes }}”</p>

            <div v-if="canDelay(order)" class="delay-row">
              <span class="delay-label">{{ t('webshop_delay_label') }}</span>
              <button
                v-for="m in delayChoices"
                :key="m"
                class="chip-delay"
                @click="handleDelay(order, m)"
              >+{{ m }}</button>
            </div>

            <footer class="card-actions">
              <span class="total-value">{{ formatEuros(order.total_cents) }}</span>
              <button
                v-if="canReject(order)"
                class="btn btn-secondary"
                :disabled="busyId === order.id"
                @click="handleReject(order)"
              >
                {{ t('webshop_action_reject') }}
              </button>
              <button
                v-if="needsReview(order)"
                class="btn btn-secondary"
                :disabled="busyId === order.id"
                @click="handleResync(order)"
              >
                {{ t('webshop_action_resync') }}
              </button>
              <button
                v-if="canCollectByCard(order)"
                class="btn btn-secondary"
                :disabled="busyId === order.id"
                @click="handleCollectCard(order)"
              >
                {{ t('webshop_action_collect_card') }}
              </button>
              <button
                v-if="canReprint(order)"
                class="btn btn-reprint"
                :class="{ 'btn-reprint-open': reprintOpenId === order.id }"
                @click="toggleReprint(order)"
              >
                {{ t('webshop_action_reprint') }}
              </button>
              <button
                v-if="nextAction(order)"
                class="btn btn-primary"
                :disabled="busyId === order.id"
                @click="handlePrimary(order)"
              >
                {{ nextAction(order)!.label }}
              </button>
            </footer>

            <div v-if="reprintOpenId === order.id && canReprint(order)" class="print-row">
              <button class="btn btn-ghost" @click="handlePrintKitchen(order)">
                {{ t('webshop_action_print_kitchen') }}
              </button>
              <button class="btn btn-ghost" @click="handlePrintCustomer(order)">
                {{ t('webshop_action_print_customer') }}
              </button>
            </div>
          </li>
        </ul>
      </section>
    </main>
  </div>
</template>

<style scoped>
.webshop-view {
  /* Delivery aggregator brand colours (their logos' colours); the own
     webshop uses the Volt accent. */
  --channel-deliveroo: #00ccbc;
  --channel-uber-eats: #06c167;
  --channel-takeaway: #ff8000;

  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: var(--font-family);
}

.mono {
  font-family: var(--font-mono);
}

.pay-badge {
  margin-left: 8px;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  font-weight: 600;
}
.pay-badge-paid {
  background: color-mix(in srgb, var(--accent-success) 15%, transparent);
  color: var(--accent-success);
}
.pay-badge-due {
  background: color-mix(in srgb, var(--accent-warning) 18%, transparent);
  color: var(--text-primary);
}

.fiscal-banner {
  margin-top: 8px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  background: color-mix(in srgb, var(--accent-warning) 18%, transparent);
  color: var(--text-primary);
  font-size: 14px;
}

.review {
  margin-top: 8px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--accent-danger);
  font-size: 14px;
}
.review-title {
  font-weight: 700;
  color: var(--accent-danger);
}
.review-reasons {
  margin: 4px 0 0;
  padding-left: 18px;
}

.state-msg {
  margin: 16px 16px 0;
  color: var(--text-secondary);
  font-size: 16px;
}
.state-error {
  color: var(--accent-danger);
}

.alarm {
  margin: 16px 16px 0;
  padding: 12px 16px;
  border-radius: var(--radius-sm);
  background: var(--accent-danger);
  color: var(--text-on-accent);
  font-size: 18px;
  font-weight: 700;
}

/* ── Board: one column per stage (§2.1) ───────────────────────── */
.board {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
  padding: 16px;
  overflow: hidden;
}
.column {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}
.column-head {
  display: flex;
  align-items: center;
  gap: 12px;
  height: 56px;
  padding: 0 4px;
  flex-shrink: 0;
}
.column-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--text-secondary);
}
.column-new .column-dot { background: var(--accent-warning); }
.column-kitchen .column-dot { background: var(--accent-info); }
.column-ready .column-dot { background: var(--accent-success); }
.column-title {
  font-size: 24px;
  font-weight: 800;
}
.column-count {
  min-width: 32px;
  height: 32px;
  padding: 0 10px;
  border-radius: 16px;
  background: var(--bg-surface);
  font-family: var(--font-mono);
  font-size: 16px;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.done-toggle {
  margin-left: auto;
  height: 48px;
  padding: 0 18px;
  border-radius: 10px;
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: 15px;
  font-weight: 700;
  transition: transform var(--tap-duration) ease;
}
.done-toggle-active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.column-list {
  list-style: none;
  margin: 0;
  padding: 0;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.column-empty {
  padding: 24px 4px;
  color: var(--text-secondary);
  font-size: 16px;
}

/* ── Card ─────────────────────────────────────────────────────── */
.card {
  position: relative;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 18px 18px 18px 24px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}
.card::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 6px;
  background: var(--channel-color, var(--text-secondary));
}
.ch-deliveroo { --channel-color: var(--channel-deliveroo); }
.ch-uber { --channel-color: var(--channel-uber-eats); }
.ch-takeaway { --channel-color: var(--channel-takeaway); }
.ch-webshop { --channel-color: var(--accent-primary); }
.card-completed,
.card-rejected,
.card-cancelled {
  opacity: 0.6;
}

.card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 36px;
}
.channel-logo {
  height: 28px;
  max-width: 160px;
  object-fit: contain;
  display: block;
}
.webshop-badge {
  font-family: var(--font-mono);
  font-size: 17px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.card-source {
  font-size: 17px;
  font-weight: 700;
}
.key-time {
  margin-left: auto;
  font-family: var(--font-mono);
  font-size: 24px;
  font-weight: 700;
  white-space: nowrap;
}
.key-time-warn { color: var(--accent-warning); }
.key-time-alarm { color: var(--accent-danger); font-weight: 800; }
.key-time-info { color: var(--accent-info); }
.key-time-plain { color: var(--text-primary); }
.key-time-muted { color: var(--text-secondary); font-size: 18px; }

.card-meta {
  font-size: 16px;
  color: var(--text-secondary);
}

.delivery-address {
  padding: 8px 12px;
  border-radius: 8px;
  background: var(--bg-surface);
  font-size: 16px;
  font-weight: 700;
}

/* ── Items ────────────────────────────────────────────────────── */
.items {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.item-main {
  display: grid;
  grid-template-columns: 36px minmax(0, 1fr);
  gap: 8px;
  align-items: center;
}
.item-qty {
  font-family: var(--font-mono);
  font-size: 18px;
  font-weight: 700;
}
.item-names {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.item-name,
.item-name-alt {
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.item-name {
  font-size: 17px;
  font-weight: 700;
}
.item-name.zh,
.item-name-alt.zh {
  font-family: var(--font-zh);
}
.item-name-alt {
  font-size: 13px;
  color: var(--text-secondary);
}
.item-options,
.item-notes {
  margin-left: 44px;
  font-size: 14px;
  color: var(--text-secondary);
}
.item-notes {
  font-style: italic;
}

.order-notes {
  margin: 0;
  padding: 8px 12px;
  border-left: 3px solid var(--accent-warning);
  font-size: 16px;
}

/* ── Delay (kitchen only) ─────────────────────────────────────── */
.delay-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.delay-label {
  font-size: 15px;
  color: var(--text-secondary);
  margin-right: 4px;
}
.chip-delay {
  flex: 1;
  height: 48px;
  background: var(--bg-surface);
  color: var(--text-primary);
  border-radius: 8px;
  font-family: var(--font-mono);
  font-size: 15px;
  font-weight: 700;
  transition: transform var(--tap-duration) ease;
}

/* ── Footer actions ───────────────────────────────────────────── */
.card-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-top: 12px;
  border-top: 1px solid var(--border);
}
.total-value {
  margin-right: auto;
  font-family: var(--font-mono);
  font-size: 24px;
  font-weight: 700;
  white-space: nowrap;
}
.btn {
  height: 64px;
  padding: 0 18px;
  border-radius: 10px;
  font-family: var(--font-family);
  font-size: 16px;
  font-weight: 700;
  transition: transform var(--tap-duration) ease;
}
.btn-secondary {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border);
}
.btn-reprint {
  padding: 0 16px;
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: 15px;
}
.btn-reprint-open {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
/* Next step in Brume: Volt stays reserved for PAYER / VALIDER /
   Enregistrer (note 03). */
.btn-primary {
  min-width: 150px;
  padding: 0 22px;
  margin-left: 8px;
  background: var(--selected-bg);
  color: var(--selected-fg);
  font-size: 18px;
  font-weight: 800;
}

.print-row {
  display: flex;
  gap: 8px;
}
.btn-ghost {
  flex: 1;
  height: 56px;
  background: var(--bg-surface);
  color: var(--text-primary);
}

.btn:active,
.chip-delay:active,
.done-toggle:active {
  transform: scale(var(--tap-scale));
}
</style>
