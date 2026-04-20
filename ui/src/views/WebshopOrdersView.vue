<script setup lang="ts">
// WebshopOrdersView — dark-theme POS screen for HubRise-origin orders
// (webshop, Deliveroo, Uber Eats, Takeaway.com, …). Matches the
// layout & tokens of OrderView so the staff keeps the same muscle
// memory (UX Guide §2.3) when switching tabs.

import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useOrderStore } from '../stores/order'
import { usePosStore } from '../stores/pos'
import { useWebshopOrdersStore } from '../stores/webshopOrders'
import { useI18n } from '../i18n'
import { formatPrice } from '../utils/format'
import FdmStatus from '../components/FdmStatus.vue'
import PrinterStatus from '../components/PrinterStatus.vue'
import HubriseStatus from '../components/HubriseStatus.vue'
import type { WebshopOrderSummary } from '../api/ipc'

const router = useRouter()
const auth = useAuthStore()
const orderStore = useOrderStore()
const pos = usePosStore()
const store = useWebshopOrdersStore()
const { t } = useI18n()

/**
 * Start a fresh takeaway order without going through the Tables
 * picker and jump to OrderView. Same behaviour as the OrderView
 * nav button so the action is identical from every screen.
 */
async function handleTakeaway() {
  await orderStore.startOrder(null)
  router.push({ name: 'order' })
}

// Pulse the pending-orders badge briefly whenever a new webshop
// order arrives (UX Guide §4.2: informative, ≤ 200 ms).
const pendingPulse = ref(false)
watch(
  () => store.pendingCount,
  (next, prev) => {
    if (next > (prev ?? 0)) {
      pendingPulse.value = true
      setTimeout(() => {
        pendingPulse.value = false
      }, 180)
    }
  },
)

// ── Clock (mirrors OrderView header) ─────────────────────────────
const clock = ref('')
let clockInterval: ReturnType<typeof setInterval>
function updateClock() {
  const now = new Date()
  clock.value = now.toLocaleTimeString('fr-BE', {
    hour: '2-digit',
    minute: '2-digit',
  })
}

// ── Filters ──────────────────────────────────────────────────────
// Active statuses get their own tab so the staff can isolate every
// working stage. Terminal statuses (completed / rejected / cancelled)
// collapse into a single "Terminées" bucket — from the rush-hour
// perspective they all mean "nothing to do, card is dead". The
// distinction is only useful for after-the-fact reporting, not
// live triage.
type Filter =
  | 'all'
  | 'new'
  | 'accepted'
  | 'in_preparation'
  | 'awaiting_collection'
  | 'in_delivery'
  | 'done'

const activeFilter = ref<Filter>('all')

// Computed so labels track locale changes via t().
const filters = computed<{ id: Filter; label: string }[]>(() => [
  { id: 'all', label: t('webshop_filter_all') },
  { id: 'new', label: t('webshop_filter_new') },
  { id: 'accepted', label: t('webshop_filter_accepted') },
  { id: 'in_preparation', label: t('webshop_filter_in_preparation') },
  { id: 'awaiting_collection', label: t('webshop_filter_awaiting_collection') },
  { id: 'in_delivery', label: t('webshop_filter_in_delivery') },
  { id: 'done', label: t('webshop_filter_done') },
])

function matchesFilter(order: WebshopOrderSummary, f: Filter): boolean {
  const s = order.hubrise_status ?? 'new'
  switch (f) {
    case 'all':
      return true
    case 'new':
      return s === 'new' || s === 'received'
    case 'accepted':
      return s === 'accepted'
    case 'in_preparation':
      return s === 'in_preparation'
    case 'awaiting_collection':
      return s === 'awaiting_collection'
    case 'in_delivery':
      return s === 'in_delivery'
    case 'done':
      return s === 'completed' || s === 'rejected' || s === 'cancelled'
  }
}

const visibleOrders = computed(() =>
  store.orders.filter((o) => matchesFilter(o, activeFilter.value)),
)

// ── Display helpers ──────────────────────────────────────────────
function formatEuros(cents: number): string {
  return formatPrice(cents) + ' €'
}

function formatCreatedAt(iso: string): string {
  // SQLite datetime('now') returns "YYYY-MM-DD HH:MM:SS" (UTC).
  const d = new Date(iso.replace(' ', 'T') + 'Z')
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleTimeString('fr-BE', { hour: '2-digit', minute: '2-digit' })
}

function statusLabel(order: WebshopOrderSummary): string {
  const s = order.hubrise_status ?? 'new'
  switch (s) {
    case 'new':
      return t('webshop_status_new')
    case 'received':
      return t('webshop_status_received')
    case 'accepted':
      return t('webshop_status_accepted')
    case 'in_preparation':
      return t('webshop_status_in_preparation')
    case 'awaiting_collection':
      return t('webshop_status_ready')
    case 'in_delivery':
      return t('webshop_status_in_delivery')
    case 'completed':
      return t('webshop_status_completed')
    case 'rejected':
      return t('webshop_status_rejected')
    case 'cancelled':
      return t('webshop_status_cancelled')
    default:
      return s
  }
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

/**
 * Color stripe per channel — makes it trivial to spot a Deliveroo
 * vs Uber Eats card across the grid at a glance (UX Guide §6).
 */
function channelColor(order: WebshopOrderSummary): string {
  const src = order.source?.toLowerCase() ?? ''
  if (src.includes('deliveroo')) return 'var(--channel-deliveroo)'
  if (src.includes('uber')) return 'var(--channel-uber-eats)'
  if (src.includes('takeaway') || src.includes('just')) return 'var(--channel-takeaway)'
  if (src.includes('web') || src.includes('webshop')) return 'var(--accent-primary)'
  return 'var(--cat-boissons)'
}

/**
 * Resolve a brand logo SVG for the order's channel. Returns null
 * when the channel has no logo file — the template then falls back
 * to a text label (UX Guide §2.1 "no hidden surprises"). Drop new
 * files in `ui/public/logos/` and extend this mapping to wire them.
 */
function channelLogo(order: WebshopOrderSummary): string | null {
  const src = order.source?.toLowerCase() ?? ''
  if (src.includes('deliveroo')) return '/logos/deliveroo.svg'
  if (src.includes('uber')) return '/logos/uber-eats.svg'
  if (src.includes('takeaway') || src.includes('just')) return '/logos/takeaway.svg'
  return null
}

/**
 * TSB's own webshop (tsb-core) has no brand SVG — we render a
 * "Tokyo" wordmark in the same Channel display font the customer
 * site uses for product category headers, so the staff sees the
 * same visual identity across both surfaces.
 */
function isTsbWebshop(order: WebshopOrderSummary): boolean {
  const src = order.source?.toLowerCase() ?? ''
  return src.includes('web') || src.includes('webshop')
}

/**
 * Show the full delivery address only on TSB webshop orders that
 * are of service_type `delivery`. TSB's own driver decides whether
 * the distance is acceptable — if not, the order is refused before
 * acceptance, so the full address must be visible at a glance.
 *
 * Aggregator channels (Deliveroo/Uber Eats/Takeaway) handle their
 * own delivery, so we deliberately hide the address on those cards
 * to keep them uncluttered.
 */
function shouldShowDeliveryAddress(order: WebshopOrderSummary): boolean {
  return (
    isTsbWebshop(order)
    && order.service_type === 'delivery'
    && !!(order.customer_address_1 || order.customer_postal_code || order.customer_city)
  )
}

/** Two-line "Rue Léopold 12 / 4000 Liège" rendering. */
function addressStreet(order: WebshopOrderSummary): string {
  const a1 = order.customer_address_1 ?? ''
  const a2 = order.customer_address_2 ?? ''
  return [a1, a2].filter(Boolean).join(' ').trim()
}
function addressCity(order: WebshopOrderSummary): string {
  const pc = order.customer_postal_code ?? ''
  const city = order.customer_city ?? ''
  return [pc, city].filter(Boolean).join(' ').trim()
}

// ── Primary action state machine ─────────────────────────────────
//
// Most channels collapse `awaiting_collection → completed` with one
// button because the transit itself is handled by an external driver
// (Deliveroo / Uber Eats / Takeaway) or the customer walks in. TSB's
// own webshop delivery, however, is handled by a TSB driver, so the
// POS needs an explicit "En livraison" transition between Prête and
// Terminer so the staff can mark the bag as having left the counter.
function nextAction(
  order: WebshopOrderSummary,
): { label: string; run: () => Promise<void> } | null {
  const status = order.hubrise_status ?? 'new'
  const id = order.hubrise_order_id
  if (!id) return null
  switch (status) {
    case 'new':
    case 'received':
      return { label: t('webshop_action_accept'), run: () => store.acceptOrder(id) }
    case 'accepted':
      return { label: t('webshop_action_prepare'), run: () => store.startPreparation(id) }
    case 'in_preparation':
      return { label: t('webshop_action_ready'), run: () => store.markReady(id) }
    case 'awaiting_collection': {
      // TSB webshop delivery → insert an "En livraison" step.
      if (isTsbWebshop(order) && order.service_type === 'delivery') {
        return {
          label: t('webshop_action_out_for_delivery'),
          run: () => store.markOutForDelivery(id),
        }
      }
      return { label: t('webshop_action_complete'), run: () => store.completeDelivery(id) }
    }
    case 'in_delivery':
      return { label: t('webshop_action_complete'), run: () => store.completeDelivery(id) }
    default:
      return null
  }
}

function canReject(order: WebshopOrderSummary): boolean {
  const s = order.hubrise_status ?? 'new'
  return s === 'new' || s === 'received'
}

/**
 * Whether the order is currently in a state where delaying the
 * ready-at time is meaningful: the kitchen has taken it but not
 * yet marked it ready/out.
 */
function canDelay(order: WebshopOrderSummary): boolean {
  const s = order.hubrise_status ?? 'new'
  return s === 'accepted' || s === 'in_preparation'
}

/**
 * Whether the card still has an active workflow so the print
 * buttons make sense. Terminal statuses (completed/rejected/
 * cancelled) hide them to reduce clutter on stale cards.
 */
function isActive(order: WebshopOrderSummary): boolean {
  const s = order.hubrise_status ?? 'new'
  return s !== 'completed' && s !== 'rejected' && s !== 'cancelled'
}

function formatReadyAt(iso: string | null): string {
  if (!iso) return ''
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return ''
  return d.toLocaleTimeString('fr-BE', {
    hour: '2-digit',
    minute: '2-digit',
  })
}

// Three chips is enough — +60 is a double tap on +30 when needed,
// and dropping it leaves the row less crowded (UX Guide §2.1).
const delayChoices = [15, 30, 45]

async function handlePrimary(order: WebshopOrderSummary) {
  const action = nextAction(order)
  if (action) await action.run()
}

async function handleReject(order: WebshopOrderSummary) {
  if (!order.hubrise_order_id) return
  await store.rejectOrder(order.hubrise_order_id)
}

async function handleDelay(order: WebshopOrderSummary, minutes: number) {
  if (!order.hubrise_order_id) return
  await store.delayByMinutes(order.hubrise_order_id, minutes)
}

async function handlePrintKitchen(order: WebshopOrderSummary) {
  await store.printKitchen(order.id)
}

async function handlePrintCustomer(order: WebshopOrderSummary) {
  await store.printCustomer(order.id)
}

// ── Lifecycle ────────────────────────────────────────────────────
onMounted(async () => {
  if (!auth.currentUser) {
    router.push({ name: 'login' })
    return
  }
  updateClock()
  clockInterval = setInterval(updateClock, 10_000)
  await store.attachListeners()
  await store.fetchOrders()
})

onUnmounted(() => clearInterval(clockInterval))

function handleLogout() {
  auth.logout()
  router.push({ name: 'login' })
}
</script>

<template>
  <!--
    UX Guide §3: same header as OrderView for muscle-memory parity.
    Filters replace the CategoryTabs row; grid of order cards replaces
    the menu grid; no cart sidebar on this screen.
  -->
  <div class="webshop-view" :class="{ training: pos.trainingMode }">
    <header class="header">
      <FdmStatus />
      <PrinterStatus />
      <HubriseStatus />
      <span class="header-user">{{ auth.currentUser?.name }}</span>
      <span class="header-clock">{{ clock }}</span>
      <nav class="header-nav">
        <button class="nav-btn" @click="router.push({ name: 'tables' })">
          {{ t('tables') }}
        </button>
        <button class="nav-btn" @click="handleTakeaway">
          {{ t('takeaway_new') }}
        </button>
        <button
          class="nav-btn nav-btn-active"
          :class="{ 'nav-btn-raised': store.pendingCount > 0 }"
        >
          {{ t('online_orders') }}
          <span
            v-if="store.pendingCount > 0"
            class="nav-pending-badge"
            :class="{ pulse: pendingPulse }"
          >{{ store.pendingCount }}</span>
        </button>
        <button class="nav-btn" @click="router.push({ name: 'reports' })">
          {{ t('reports') }}
        </button>
        <button class="nav-btn" @click="router.push({ name: 'settings' })">
          {{ t('config') }}
        </button>
        <button class="nav-btn nav-logout" @click="handleLogout">
          {{ t('logout') }}
        </button>
      </nav>
    </header>

    <section class="filter-bar">
      <button
        v-for="f in filters"
        :key="f.id"
        class="filter-chip"
        :class="{ 'filter-chip-active': activeFilter === f.id }"
        @click="activeFilter = f.id"
      >
        <span class="filter-label">{{ f.label }}</span>
        <span class="filter-count">
          {{ store.orders.filter((o) => matchesFilter(o, f.id)).length }}
        </span>
      </button>
    </section>

    <main class="grid-wrap">
      <p v-if="store.loading" class="state-msg">{{ t('webshop_loading') }}</p>
      <p v-else-if="store.error" class="state-msg state-error">
        {{ store.error }}
      </p>
      <p v-else-if="visibleOrders.length === 0" class="state-msg">
        {{ t('webshop_empty') }}
      </p>

      <ul v-else class="grid">
        <li
          v-for="order in visibleOrders"
          :key="order.id"
          class="card"
          :class="`card-${order.hubrise_status ?? 'new'}`"
          :style="{ '--channel-color': channelColor(order) }"
        >
          <header class="card-head">
            <div class="card-channel">
              <img
                v-if="channelLogo(order)"
                :src="channelLogo(order)!"
                :alt="order.source"
                class="channel-logo"
              />
              <span v-else-if="isTsbWebshop(order)" class="tsb-badge">
                Tokyo
              </span>
              <span v-else class="card-source">{{ order.source }}</span>
            </div>
            <span class="card-status">{{ statusLabel(order) }}</span>
          </header>
          <div class="card-body">
            <div class="row">
              <span class="label">{{ t('webshop_label_type') }}</span>
              <span class="value">{{ serviceTypeLabel(order) }}</span>
            </div>
            <div class="row">
              <span class="label">{{ t('webshop_label_time') }}</span>
              <span class="value">{{ formatCreatedAt(order.created_at) }}</span>
            </div>
            <div v-if="order.customer_name" class="row">
              <span class="label">{{ t('webshop_label_customer') }}</span>
              <span class="value">{{ order.customer_name }}</span>
            </div>
            <!-- Full delivery address — TSB webshop + delivery only.
                 Aggregators are muted here because their drivers
                 handle navigation; TSB drivers need this to decide
                 accept/refuse based on distance. -->
            <div v-if="shouldShowDeliveryAddress(order)" class="delivery-address">
              <span class="delivery-address-label">
                {{ t('webshop_label_delivery_address') }}
              </span>
              <span v-if="addressStreet(order)" class="delivery-address-line">
                {{ addressStreet(order) }}
              </span>
              <span v-if="addressCity(order)" class="delivery-address-line">
                {{ addressCity(order) }}
              </span>
            </div>
            <div v-if="order.confirmed_time" class="row">
              <span class="label">{{ t('webshop_label_ready_at') }}</span>
              <span class="value ready-at">
                {{ formatReadyAt(order.confirmed_time) }}
              </span>
            </div>
            <div class="row">
              <span class="label">{{ t('webshop_label_ref') }}</span>
              <span class="value ref">
                {{ order.hubrise_order_id?.slice(0, 8) ?? '—' }}
              </span>
            </div>
          </div>

          <ul v-if="order.items.length > 0" class="items">
            <li v-for="(item, idx) in order.items" :key="idx" class="item">
              <div class="item-main">
                <span class="item-qty">{{ item.quantity }}×</span>
                <span class="item-name">{{ item.product_name }}</span>
                <span class="item-price">
                  {{ formatEuros(item.line_total_cents) }}
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

          <p v-if="order.customer_notes" class="order-notes">
            “{{ order.customer_notes }}”
          </p>

          <div class="card-total">
            <span class="total-label">{{ t('webshop_label_total') }}</span>
            <span class="total-value">{{ formatEuros(order.total_cents) }}</span>
          </div>
          <footer class="card-actions">
            <!-- Delay chips — visible only while the kitchen is
                 working on the order, per UX Guide §2.1 (reduce
                 noise when irrelevant). -->
            <div v-if="canDelay(order)" class="delay-row">
              <button
                v-for="m in delayChoices"
                :key="m"
                class="chip chip-delay"
                @click="handleDelay(order, m)"
              >
                +{{ m }} min
              </button>
            </div>

            <!-- Primary next-status button + reject pair -->
            <div class="primary-row">
              <button
                v-if="nextAction(order)"
                class="btn btn-primary"
                @click="handlePrimary(order)"
              >
                {{ nextAction(order)!.label }}
              </button>
              <button
                v-if="canReject(order)"
                class="btn btn-secondary"
                @click="handleReject(order)"
              >
                {{ t('webshop_action_reject') }}
              </button>
            </div>

            <!-- Print row — available on every non-terminal state. -->
            <div v-if="isActive(order)" class="print-row">
              <button
                class="btn btn-ghost"
                @click="handlePrintKitchen(order)"
              >
                {{ t('webshop_action_print_kitchen') }}
              </button>
              <button
                class="btn btn-ghost"
                @click="handlePrintCustomer(order)"
              >
                {{ t('webshop_action_print_customer') }}
              </button>
            </div>
          </footer>
        </li>
      </ul>
    </main>
  </div>
</template>

<style scoped>
/* UX Guide §3, §5: dark theme, same shell as OrderView */
.webshop-view {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: var(--font-family);
}

.webshop-view.training {
  --bg-primary: #2d1b4e;
}

/* ── Header (48px, mirror OrderView) ──────────────────────────── */
.header {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  padding: 0 var(--space-md);
  background: var(--bg-secondary);
  border-bottom: 1px solid rgba(160, 160, 176, 0.1);
  flex-shrink: 0;
  height: 48px;
}
.header-user {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
.header-clock {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}
.header-nav {
  margin-left: auto;
  display: flex;
  gap: var(--space-xs);
}
.nav-btn {
  position: relative;
  padding: var(--space-xs) var(--space-md);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-secondary);
  min-height: 36px;
  border: none;
  cursor: pointer;
  transition: transform var(--tap-duration) ease;
}
.nav-btn:active {
  transform: scale(var(--tap-scale));
}
.nav-btn-active {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.nav-logout {
  color: var(--accent-danger);
}

/* ── Filter bar (replaces CategoryTabs) ───────────────────────── */
.filter-bar {
  display: flex;
  gap: var(--space-sm);
  padding: var(--space-md);
  background: var(--bg-secondary);
  border-bottom: 1px solid rgba(160, 160, 176, 0.1);
  flex-shrink: 0;
  overflow-x: auto;
}
.filter-chip {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  min-height: 52px;
  padding: 0 var(--space-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  border: none;
  border-radius: var(--radius-md);
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  cursor: pointer;
  transition: transform var(--tap-duration) ease;
}
.filter-chip:active {
  transform: scale(var(--tap-scale));
}
.filter-chip-active {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.filter-label {
  white-space: nowrap;
}
.filter-count {
  min-width: 24px;
  padding: 2px 8px;
  background: rgba(0, 0, 0, 0.25);
  border-radius: 999px;
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;
}

/* ── Card grid ────────────────────────────────────────────────── */
.grid-wrap {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-md);
}
.state-msg {
  text-align: center;
  margin-top: var(--space-xl);
  color: var(--text-secondary);
  font-size: var(--font-size-md);
}
.state-error {
  color: var(--accent-danger);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: var(--space-md);
  list-style: none;
  padding: 0;
  margin: 0;
}

/* Each card uses --channel-color (set inline) for the left stripe
   so Deliveroo/Uber Eats/Takeaway/Web are distinguishable at a
   glance. */
.card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  padding: var(--space-md) var(--space-md) var(--space-md) calc(var(--space-md) + 6px);
  background: var(--bg-secondary);
  border: 1px solid rgba(160, 160, 176, 0.12);
  border-radius: var(--radius-md);
  overflow: hidden;
}
.card::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 6px;
  background: var(--channel-color, var(--accent-primary));
}
.card-completed,
.card-rejected,
.card-cancelled {
  opacity: 0.55;
}

.card-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-sm);
}
.card-channel {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  min-width: 0;
}
.channel-logo {
  height: 28px;
  max-width: 160px;
  object-fit: contain;
  display: block;
}
.card-source {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
}
/* TSB webshop identity — Channel display font, white text for the
   dark POS theme, kept compact so it doesn't inflate the card head. */
.tsb-badge {
  font-family: 'Channel', var(--font-family);
  font-size: 18px;
  line-height: 18px;
  color: var(--text-primary);
  letter-spacing: 0.5px;
}
.card-status {
  padding: var(--space-xs) var(--space-sm);
  background: var(--bg-surface);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  text-transform: uppercase;
  white-space: nowrap;
}
.card-new .card-status,
.card-received .card-status {
  background: var(--accent-warning);
  color: var(--bg-primary);
}
.card-accepted .card-status,
.card-in_preparation .card-status {
  background: var(--cat-sashimi);
  color: var(--text-on-accent);
}
.card-awaiting_collection .card-status,
.card-in_delivery .card-status {
  background: var(--accent-success);
  color: var(--bg-primary);
}
.card-rejected .card-status,
.card-cancelled .card-status {
  background: var(--accent-danger);
  color: var(--text-on-accent);
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}
.row {
  display: flex;
  justify-content: space-between;
  font-size: var(--font-size-sm);
}
.label {
  color: var(--text-secondary);
}
.value {
  color: var(--text-primary);
  font-weight: var(--font-weight-bold);
}
.value.ref {
  font-family: 'SFMono-Regular', Menlo, monospace;
  font-size: var(--font-size-xs);
}

/* ── Items list ───────────────────────────────────────────────── */
.items {
  list-style: none;
  margin: 0;
  padding: var(--space-sm) 0;
  border-top: 1px dashed rgba(160, 160, 176, 0.2);
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}
.item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.item-main {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: var(--space-sm);
  align-items: baseline;
  font-size: var(--font-size-sm);
  color: var(--text-primary);
}
.item-qty {
  min-width: 28px;
  font-weight: var(--font-weight-bold);
  color: var(--accent-primary);
  font-variant-numeric: tabular-nums;
}
.item-name {
  font-weight: var(--font-weight-normal);
}
.item-price {
  font-variant-numeric: tabular-nums;
  color: var(--text-secondary);
}
.item-options,
.item-notes {
  margin-left: calc(28px + var(--space-sm));
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
}
.item-notes {
  font-style: italic;
}

.order-notes {
  margin: 0;
  padding: var(--space-sm);
  background: rgba(243, 156, 18, 0.12);
  border-left: 3px solid var(--accent-warning);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  font-style: italic;
  border-radius: var(--radius-sm);
}

.card-total {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: var(--space-sm) 0;
  border-top: 1px dashed rgba(160, 160, 176, 0.2);
}
.total-label {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.total-value {
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  color: var(--accent-primary);
  font-variant-numeric: tabular-nums;
}

/* ── Card actions (UX Guide §4.1) ─────────────────────────────── */
.card-actions {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}
.primary-row,
.print-row,
.delay-row {
  display: flex;
  gap: var(--space-sm);
}
.btn {
  flex: 1;
  min-height: 64px;
  padding: 0 var(--space-md);
  border: none;
  border-radius: var(--radius-md);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  cursor: pointer;
  transition: transform var(--tap-duration) ease;
}
.btn:active {
  transform: scale(var(--tap-scale));
}
.btn-primary {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.btn-primary:active {
  background: var(--bg-pressed);
}
.btn-secondary {
  flex: 0 0 auto;
  min-width: 96px;
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid rgba(160, 160, 176, 0.25);
}
/* Print row is lower-priority than the status transition, so the
   buttons are slightly smaller and use a ghost style. */
.print-row .btn-ghost {
  min-height: 48px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  border: none;
}
.print-row .btn-ghost:active {
  background: var(--bg-pressed);
}

/* Delay chips — 4 equal-width chips above the primary action. */
.chip-delay {
  flex: 1;
  min-height: 52px;
  background: var(--bg-surface);
  color: var(--text-primary);
  border: 1px solid rgba(160, 160, 176, 0.18);
  border-radius: var(--radius-sm);
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  font-variant-numeric: tabular-nums;
  cursor: pointer;
  transition: transform var(--tap-duration) ease;
}
.chip-delay:active {
  transform: scale(var(--tap-scale));
  background: var(--accent-warning);
  color: var(--bg-primary);
}

/* "Prêt à HH:MM" is the currently targeted ready time — shown
   in amber so it stands out from the neutral metadata rows. */
.value.ready-at {
  color: var(--accent-warning);
  font-variant-numeric: tabular-nums;
}

/* Full delivery address block — only rendered on TSB webshop
   delivery cards. Two lines max (street + postal+city) so the
   driver can decide accept/refuse based on distance at a glance. */
.delivery-address {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: var(--space-sm);
  background: rgba(15, 52, 96, 0.35);
  border-left: 3px solid var(--cat-sashimi);
  border-radius: var(--radius-sm);
}
.delivery-address-label {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
}
.delivery-address-line {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
  font-weight: var(--font-weight-bold);
}
</style>
