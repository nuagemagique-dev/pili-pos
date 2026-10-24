<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useOrderStore } from '../stores/order'
import { useAuthStore } from '../stores/auth'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'
import { describeError } from '../utils/errorMessages'
import { useToastStore } from '../stores/toast'
import { formatPrice } from '../utils/format'
import { formatTimestamp, formatTime, minutesSince } from '../utils/datetime'
import AppHeader from '../components/AppHeader.vue'

const router = useRouter()
const { t } = useI18n()
const order = useOrderStore()
const auth = useAuthStore()
const toast = useToastStore()

const tables = ref<ipc.TableInfo[]>([])
// Takeaways left unpaid: each one must still end in a sale (FAQ Q205).
const takeaways = ref<ipc.TakeawayInfo[]>([])
// Orders whose card was charged and not yet settled by a sale.
const cardCharged = ref(new Set<number>())

// Minutes open, refreshed while the screen is shown.
const now = ref(new Date())
let nowTimer: ReturnType<typeof setInterval> | undefined

onMounted(async () => {
  if (!auth.currentUser) {
    router.push({ name: 'login' })
    return
  }
  nowTimer = setInterval(() => (now.value = new Date()), 30_000)
  tables.value = await ipc.listTables()
  takeaways.value = await ipc.listOpenTakeaways()
  try {
    cardCharged.value = new Set((await ipc.listUnsettledCardPayments()).map((p) => p.order_id))
  } catch {
    /* only a badge */
  }
})
onUnmounted(() => clearInterval(nowTimer))

async function resumeTakeaway(takeaway: ipc.TakeawayInfo) {
  try {
    await order.loadOrder(takeaway.orderId)
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
    return
  }
  router.push({ name: 'order' })
}

async function newTakeaway() {
  try {
    await order.startOrder(null)
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
    return
  }
  router.push({ name: 'order' })
}

// "Déplacer une table": tap the occupied table, then a free one, then
// confirm. The engine signs the move (event P, cost center change).
const moving = ref(false)
const source = ref<ipc.TableInfo | null>(null)
const target = ref<ipc.TableInfo | null>(null)
const transferring = ref(false)

const moveHint = computed(() =>
  source.value
    ? t('transfer_pick_free', { from: source.value.tableNumber })
    : t('transfer_pick_source'),
)

function toggleMove() {
  moving.value = !moving.value
  source.value = null
  target.value = null
}

/** In the mode only a source (occupied) or, once chosen, a target
 * (free) can be tapped. */
function selectable(table: ipc.TableInfo): boolean {
  if (!source.value) return table.status === 'occupied' && table.orderId != null
  return table.status === 'free'
}

function pickInMoveMode(table: ipc.TableInfo) {
  if (!selectable(table)) return
  if (!source.value) source.value = table
  else target.value = table
}

async function confirmMove() {
  const from = source.value
  const to = target.value
  if (!from?.orderId || !to || transferring.value) return
  transferring.value = true
  try {
    await order.transferOrder(from.orderId, to.tableNumber)
    toast.show(t('transfer_done', { to: to.tableNumber }), 'success')
  } catch (e) {
    toast.show(describeError(e), 'error', 6000)
  } finally {
    transferring.value = false
    moving.value = false
    source.value = null
    target.value = null
    try {
      tables.value = await ipc.listTables()
    } catch {
      /* the list is read again at the next visit */
    }
  }
}

async function selectTable(table: ipc.TableInfo) {
  if (moving.value) {
    pickInMoveMode(table)
    return
  }
  try {
    if (table.orderId) {
      // Rehydrate the existing order AND its lines so the cart is
      // repopulated (fixes the bug where tapping a busy table after
      // a takeaway switch showed an empty cart).
      await order.loadOrder(table.orderId)
    } else {
      await order.startOrder(table.tableNumber)
    }
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
    return
  }
  router.push({ name: 'order' })
}
</script>

<template>
  <div class="tables-view">
    <AppHeader />

    <div class="main">
      <!-- §5.3: occupied tables filled, total in bold; free ones dashed. -->
      <div class="plan">
        <div class="plan-bar">
          <button
            class="move-toggle"
            :class="{ active: moving }"
            data-testid="move-toggle"
            @click="toggleMove"
          >
            {{ t('transfer_mode') }}
          </button>
          <span v-if="moving" class="move-hint" data-testid="move-hint">{{ moveHint }}</span>
        </div>
        <div class="grid">
          <button
            v-for="tbl in tables"
            :key="tbl.tableNumber"
            class="table-card"
            :class="{
              occupied: tbl.status === 'occupied',
              dimmed: moving && !selectable(tbl) && source?.tableNumber !== tbl.tableNumber,
              chosen: moving && source?.tableNumber === tbl.tableNumber,
            }"
            @click="selectTable(tbl)"
          >
            <span class="table-top">
              <span class="table-num">{{ tbl.tableNumber }}</span>
              <span v-if="tbl.preBillPrinted" class="bill-tag">{{ t('pre_bill') }}</span>
              <span
                v-if="tbl.heldCourse"
                class="bill-tag"
                :data-testid="`held-course-${tbl.tableNumber}`"
                >{{ t('table_held_course', { course: t(`course_${tbl.heldCourse}`) }) }}</span
              >
              <span
                v-if="tbl.orderId && cardCharged.has(tbl.orderId)"
                class="bill-tag charged-tag"
                data-testid="card-charged"
                >{{ t('card_charged_badge') }}</span
              >
            </span>
            <span v-if="tbl.status === 'occupied'" class="table-info">
              <span class="table-total">{{ formatPrice(tbl.totalCents) }} €</span>
              <span class="table-meta">
                {{ tbl.lineCount }} {{ t('items_short')
                }}<template v-if="tbl.openedAt">
                  · {{ minutesSince(tbl.openedAt, now) }} min</template
                >
              </span>
            </span>
            <span v-else class="table-free">{{ t('free') }}</span>
          </button>
        </div>
      </div>

      <!-- Open takeaways where the cart usually is; the main action at
           the bottom right (§2.3). -->
      <aside class="takeaways">
        <div class="takeaways-header">
          <h2>{{ t('takeaways_open') }}</h2>
          <span class="takeaways-count">{{ takeaways.length }}</span>
        </div>
        <div class="takeaway-list scroll-touch">
          <button
            v-for="tk in takeaways"
            :key="tk.orderId"
            class="takeaway-card"
            @click="resumeTakeaway(tk)"
          >
            <span class="takeaway-text">
              <span class="takeaway-time">{{
                t('takeaway_started_at', { time: formatTimestamp(tk.createdAt, formatTime) })
              }}</span>
              <span class="takeaway-meta">{{ tk.lineCount }} {{ t('items_short') }}</span>
              <span
                v-if="cardCharged.has(tk.orderId)"
                class="bill-tag charged-tag"
                data-testid="card-charged"
                >{{ t('card_charged_badge') }}</span
              >
            </span>
            <span class="takeaway-total">{{ formatPrice(tk.totalCents) }} €</span>
          </button>
        </div>
        <div class="takeaways-footer">
          <button class="new-takeaway-btn" @click="newTakeaway">{{ t('takeaway_start') }}</button>
        </div>
      </aside>
    </div>
    <div v-if="source && target" class="backdrop" @click.self="target = null">
      <div
        class="dialog"
        role="dialog"
        :aria-label="t('transfer_table')"
        data-testid="move-confirm"
      >
        <p class="dialog-text">
          {{ t('transfer_confirm', { from: source.tableNumber, to: target.tableNumber }) }}
        </p>
        <div class="dialog-actions">
          <button class="btn btn-cancel" @click="target = null">{{ t('back') }}</button>
          <button
            class="btn btn-confirm"
            :disabled="transferring"
            data-testid="move-validate"
            @click="confirmMove"
          >
            {{ t('validate') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tables-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  background: var(--bg-primary);
}

.main {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.plan {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}
.plan-bar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px 24px 0;
}
.move-toggle {
  height: 56px;
  padding: 0 20px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.move-toggle:active {
  transform: scale(var(--tap-scale));
}
.move-toggle.active {
  background: var(--accent-warning);
  color: var(--text-on-accent);
}
.move-hint {
  font-size: 18px;
  color: var(--text-secondary);
}
.table-card.dimmed {
  opacity: 0.3;
  pointer-events: none;
}
.table-card.chosen {
  outline: 4px solid var(--accent-warning);
}

.backdrop {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  z-index: 50;
}
.dialog {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  min-width: 480px;
}
.dialog-text {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.dialog-actions {
  display: flex;
  gap: var(--space-sm);
}
.btn {
  flex: 1;
  height: 56px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  transition: transform var(--tap-duration) ease;
}
.btn:active {
  transform: scale(var(--tap-scale));
}
.btn-cancel {
  background: var(--bg-surface);
  color: var(--text-secondary);
}
.btn-confirm {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.btn-confirm:disabled {
  opacity: 0.4;
}

.grid {
  flex: 1;
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  grid-template-rows: repeat(4, minmax(0, 1fr));
  gap: 16px;
  padding: 24px;
}

.table-card {
  position: relative;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  align-items: flex-start;
  padding: 18px 20px;
  border-radius: var(--radius-lg);
  border: 2px dashed var(--border);
  color: var(--text-secondary);
  text-align: left;
  transition: transform var(--tap-duration) ease;
}
.table-card:active {
  transform: scale(var(--tap-scale));
}

.table-card.occupied {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  color: var(--text-primary);
}
/* Brume band on top: taken, at a glance. */
.table-card.occupied::before {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  top: 0;
  height: 6px;
  background: var(--text-primary);
}

.table-top {
  display: flex;
  width: 100%;
  justify-content: space-between;
  align-items: flex-start;
}
.table-num {
  font-family: var(--font-mono);
  font-size: 44px;
  font-weight: var(--font-weight-bold);
  line-height: 1;
}
/* A provisional bill is on the table (signPreBill, P). */
.charged-tag {
  align-self: flex-start;
}
.bill-tag {
  padding: 2px 8px;
  border-radius: 6px;
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
}

.table-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.table-total {
  font-family: var(--font-mono);
  font-size: 26px;
  font-weight: var(--font-weight-bold);
}
.table-meta {
  font-family: var(--font-mono);
  font-size: 15px;
  color: var(--text-secondary);
}
.table-free {
  font-size: 16px;
}

.takeaways {
  width: var(--cart-width);
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border);
}
.takeaways-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  padding: 16px 16px 12px;
  border-bottom: 1px solid var(--border);
}
.takeaways-header h2 {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}
.takeaways-count {
  font-family: var(--font-mono);
  font-size: 16px;
  color: var(--text-secondary);
}
.takeaway-list {
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* Amber: a takeaway still waiting for its payment. */
.takeaway-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: 16px;
  min-height: 80px;
  padding: 12px 20px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  border-left: 4px solid var(--accent-warning);
  color: var(--text-primary);
  text-align: left;
  transition: transform var(--tap-duration) ease;
}
.takeaway-card:active {
  transform: scale(var(--tap-scale));
}
.takeaway-text {
  display: flex;
  flex-direction: column;
}
.takeaway-time {
  font-size: 20px;
  font-weight: var(--font-weight-bold);
}
.takeaway-meta {
  font-family: var(--font-mono);
  font-size: 15px;
  color: var(--text-secondary);
}
.takeaway-total {
  font-family: var(--font-mono);
  font-size: 22px;
  font-weight: var(--font-weight-bold);
}

.takeaways-footer {
  padding: 16px;
  border-top: 1px solid var(--border);
}
.new-takeaway-btn {
  width: 100%;
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
.new-takeaway-btn:active {
  transform: scale(var(--tap-scale));
}
</style>
