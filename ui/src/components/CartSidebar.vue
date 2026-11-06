<script setup lang="ts">
import { describeError } from '../utils/errorMessages'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useOrderStore } from '../stores/order'
import { useAuthStore } from '../stores/auth'
import { useToastStore } from '../stores/toast'
import { useI18n } from '../i18n'
import { formatPrice } from '../utils/format'
import { lineNetCents } from '../utils/cartAmounts'
import { formatTimestamp, formatTime, minutesSince } from '../utils/datetime'
import OrderLineItem from './OrderLineItem.vue'
import VatSummary from './VatSummary.vue'
import PinDialog from './PinDialog.vue'
import TableMover from './TableMover.vue'
import DiscountDialog from './DiscountDialog.vue'
import NoteDialog from './NoteDialog.vue'
import CartMoreMenu from './CartMoreMenu.vue'
import { kitchenIsPrimary, sendableLineCount } from '../utils/cartPrimary'
import type { CartLine, DiscountInput } from '../types'
import * as ipc from '../api/ipc'
import CoursePickerDialog from './CoursePickerDialog.vue'
import { GIFT_VOUCHER_DEPARTMENT } from '../stores/giftVouchers'

const router = useRouter()
const order = useOrderStore()
// Payment parts already taken by hand for this order and kept when the operator came back from the
// checkout (UX audit P4).
const keptParts = computed(() =>
  order.currentOrder ? order.heldPartsFor(order.currentOrder.id).length : 0,
)
const auth = useAuthStore()
const toast = useToastStore()
const { t } = useI18n()

const isDineIn = computed(() => order.currentOrder?.order_type === 'dine_in')

// Dine-in lines not yet sent to the bar / kitchen go last, under an
// amber label; a takeaway's lines all go out with the sale.
const sentLines = computed(() =>
  isDineIn.value ? order.lines.filter((l) => (l.unsent_quantity ?? 0) <= 0) : order.lines,
)
const pendingLines = computed(() =>
  isDineIn.value ? order.lines.filter((l) => (l.unsent_quantity ?? 0) > 0) : [],
)

// Courses on: the lines are grouped by course, lowest first, under a
// header. A takeaway groups too (its slip is printed with the same headers).
const courseGroups = computed(() => {
  const groups = new Map<number, CartLine[]>()
  for (const line of order.lines) {
    const list = groups.get(line.course) ?? []
    list.push(line)
    groups.set(line.course, list)
  }
  return [...groups.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([course, lines]) => ({ course, lines }))
})

const pickingLine = ref<CartLine | null>(null)

async function pickCourse(course: number) {
  const line = pickingLine.value
  pickingLine.value = null
  if (line && course !== line.course) await order.setLineCourse(line.id, course)
}

// "Envoyer la suite": signs what is unsent, releases the next held
// course and prints it.
const sendingNext = ref(false)

async function sendNextCourse() {
  if (sendingNext.value || order.nextHeldCourse === null) return
  sendingNext.value = true
  try {
    const next = await order.sendNextCourse()
    if (next) {
      toast.show(
        next.course === null
          ? t('course_resent')
          : t('course_sent', { course: t(`course_${next.course}`) }),
        'success',
      )
    }
  } catch (e) {
    toast.show(describeError(e), 'error', 6000)
  } finally {
    sendingNext.value = false
  }
}

// "19:05 · 37 min": when the order was opened, and for how long.
const now = ref(new Date())
let nowTimer: ReturnType<typeof setInterval> | undefined
onMounted(() => {
  nowTimer = setInterval(() => (now.value = new Date()), 30_000)
})
onUnmounted(() => clearInterval(nowTimer))
const openedAt = computed(() => {
  const created = order.currentOrder?.created_at
  if (!created) return ''
  return `${formatTimestamp(created, formatTime)} · ${minutesSince(created, now.value)} min`
})

const cartTitle = computed(() => {
  const current = order.currentOrder
  if (current?.table_number) return `${t('order')} · ${t('table')} ${current.table_number}`
  if (current?.order_type === 'takeaway') return `${t('order')} · ${t('takeaway_badge')}`
  return t('order')
})

/** The add that went to [lineId] (its sequence number), for the line's 200 ms flash; 0 for any other line. */
function flashFor(lineId: number): number {
  return order.lastAdd?.lineId === lineId ? order.lastAdd.seq : 0
}

// UX audit C2: one line is selected, its actions show under it. By default it is the newest line
// of the cart; a tap on the selected line folds it, a tap on another one moves the selection, and
// an add selects the line it went to. `undefined` is "no choice yet": the newest line.
const choice = ref<number | null | undefined>(undefined)

/** The newest line the operator can select (a bowl's paid option follows its bowl). */
const newestLineId = computed(() => {
  for (let i = order.lines.length - 1; i >= 0; i--) {
    const line = order.lines[i]
    if (line && (line.bowl_line_id === null || line.bowl_line_id === undefined)) return line.id
  }
  return null
})
const selectedLineId = computed(() =>
  choice.value === undefined ? newestLineId.value : choice.value,
)

function selectLine(lineId: number) {
  choice.value = selectedLineId.value === lineId ? null : lineId
}

watch(
  () => order.lastAdd?.seq,
  () => {
    const add = order.lastAdd
    if (add) choice.value = add.lineId
  },
)

// Another order, or a line that left the cart (removed, paid): back to the newest line.
watch(
  () => order.currentOrder?.id,
  () => {
    choice.value = undefined
  },
)
watch(
  () => order.lines,
  (lines) => {
    if (typeof choice.value === 'number' && !lines.some((l) => l.id === choice.value)) {
      choice.value = undefined
    }
  },
)

function increment(lineId: number) {
  const line = order.lines.find((l) => l.id === lineId)
  if (line) void order.updateQuantity(lineId, line.quantity + 1)
}

function decrement(lineId: number) {
  const line = order.lines.find((l) => l.id === lineId)
  if (line && line.quantity > 1) void order.updateQuantity(lineId, line.quantity - 1)
}

// Send what was ordered to the bar / kitchen without leaving the table
// (event P, the same registration as when leaving the order).
const sending = ref(false)

async function sendToKitchen() {
  if (sending.value || order.unsentCount === 0) return
  sending.value = true
  const count = order.unsentCount
  try {
    await order.leave('send_to_kitchen')
    toast.show(t('sent_to_kitchen', { n: count }), 'success')
  } catch (e) {
    toast.show(`${t('order_register_failed')} : ${describeError(e)}`, 'error', 6000)
  } finally {
    sending.value = false
  }
}

// UX audit C4: while lines wait for the kitchen, "Envoyer en cuisine" is
// the Volt primary (with a pill counting them) and PAYER is a secondary
// outlined button that still shows the total. Once everything is sent
// PAYER is the primary again; a role that cannot pay always has the
// kitchen button as its primary.
const sendableLines = computed(() => sendableLineCount(order.lines, isDineIn.value))
const kitchenPrimary = computed(() =>
  kitchenIsPrimary({
    dineIn: isDineIn.value,
    canTakePayment: auth.canTakePayment,
    sendableLines: sendableLines.value,
  }),
)
// The secondary row (kitchen or PAYER, and "Addition") is empty for a role
// that cannot pay while the kitchen button is the primary.
const showActionRow = computed(
  () => isDineIn.value && !(kitchenPrimary.value && !auth.canTakePayment),
)
const sendPill = computed(() =>
  sendableLines.value === 1
    ? t('send_lines_one')
    : t('send_lines_other', { n: sendableLines.value }),
)

function goToCheckout() {
  if (order.lines.length > 0) {
    router.push({ name: 'checkout' })
  }
}

// Provisional bill for a table (§1.4). Takeaway sales are paid at once.
const canPreBill = computed(
  () => order.currentOrder?.order_type === 'dine_in' && order.lines.length > 0,
)
const preBilling = ref(false)

async function printPreBill() {
  if (!canPreBill.value || preBilling.value) return
  preBilling.value = true
  try {
    await order.preBill()
    toast.show(t('pre_bill_printed'), 'success')
  } catch (e) {
    toast.show(`${t('pre_bill_failed')} : ${describeError(e)}`, 'error', 6000)
  } finally {
    preBilling.value = false
  }
}

// Move the order to another table (event P, cost center change): the
// picker sheet and the one-tap move are TableMover's (C8).
const canTransfer = computed(() => order.canTransfer)
const picking = ref(false)

// Kitchen note of a line: any staff member, on an open order. The dialog
// says whether the note prints now (the line already went to the kitchen)
// or with the order.
const canNote = computed(() => order.currentOrder?.status === 'open')
const noteLineId = ref<number | null>(null)
const quickNotes = ref<string[]>([])

/** UX audit C11: the chips are read each time the dialog opens, so a manager's change shows at once. */
async function openNote(lineId: number) {
  quickNotes.value = await order.loadQuickNotes()
  noteLineId.value = lineId
}
const noteLine = computed(() => order.lines.find((l) => l.id === noteLineId.value) ?? null)

async function saveNote(text: string) {
  const id = noteLineId.value
  noteLineId.value = null
  if (id !== null) await order.setLineNote(id, text)
}

// Discount at the till (manager, PIN): an order discount, a line for
// free, or the removal of either. The dialog collects the input, the PIN
// confirms it, the engine applies every rule.
// An online order has its own discounts: the engine refuses one at the
// till (discount_online_order), so the button and the line actions hide.
const canDiscount = computed(
  () =>
    auth.isManager &&
    order.currentOrder?.status === 'open' &&
    order.lines.length > 0 &&
    !order.lines.some((l) => l.online === true),
)
type DiscountStep =
  | { kind: 'ask'; lineId: number | null }
  | { kind: 'pin-apply'; input: DiscountInput }
  | { kind: 'pin-remove'; lineId: number | null }
const discountStep = ref<DiscountStep | null>(null)

function lineHasOwnChange(lineId: number): boolean {
  const line = order.lines.find((l) => l.id === lineId)
  return Boolean(line?.price_changes?.some((c) => c.scope === 'LINE'))
}
function canOfferLine(lineId: number): boolean {
  const line = order.lines.find((l) => l.id === lineId)
  if (line?.department_id === GIFT_VOUCHER_DEPARTMENT) return false
  return canDiscount.value && !order.orderDiscounted && (line?.price_changes?.length ?? 0) === 0
}
function canRemoveLineDiscount(lineId: number): boolean {
  return canDiscount.value && lineHasOwnChange(lineId)
}

/** What the open dialog applies to: the line's amount, or the order total. */
const discountBase = computed(() => {
  const step = discountStep.value
  if (step?.kind === 'ask' && step.lineId !== null) {
    const line = order.lines.find((l) => l.id === step.lineId)
    return line ? lineNetCents(line) : 0
  }
  // The order total, as on Android (no money arithmetic in the Compose UI): a gift voucher
  // line never shares an order discount, the engine leaves it out (V12 merge decision).
  return order.totalCents
})

async function verifyDiscount(pin: string) {
  const step = discountStep.value
  if (step?.kind === 'pin-apply') await order.applyDiscount(step.input, pin)
  else if (step?.kind === 'pin-remove') await order.removeDiscount(step.lineId, pin)
}

function onDiscountDone() {
  const removed = discountStep.value?.kind === 'pin-remove'
  discountStep.value = null
  toast.show(t(removed ? 'discount_removed' : 'discount_applied'), 'success')
}

// The customer left without paying (UC4109): a manager closes the
// order, confirmed by their PIN; the sale is signed with no payment.
const canCloseUnpaid = computed(
  () => auth.isManager && order.currentOrder?.status === 'open' && order.lines.length > 0,
)
const confirmUnpaid = ref(false)

// UX audit C3: transfer, order discount and "customer left" live in the
// "Plus" menu of the header, with the same role rules as before.
const hasMore = computed(() => canTransfer.value || canDiscount.value || canCloseUnpaid.value)
const moreOpen = ref(false)

function fromMore(action: () => void) {
  moreOpen.value = false
  action()
}

async function closeUnpaid(pin: string) {
  await order.closeUnpaid(pin)
}

async function onUnpaidClosed() {
  confirmUnpaid.value = false
  toast.show(t('unpaid_closed'), 'success')
  await router.push({ name: 'tables' })
}
</script>

<template>
  <!-- UX guide §3: the cart always on the right, 670 px. -->
  <aside class="cart">
    <div class="cart-header">
      <h2>{{ cartTitle }}</h2>
      <button
        v-if="hasMore"
        class="more-btn"
        data-testid="cart-more-open"
        :aria-label="t('cart_more')"
        aria-haspopup="dialog"
        @click="moreOpen = true"
      >
        <!-- Material "more_horiz" -->
        <svg width="24" height="24" viewBox="0 0 24 24" aria-hidden="true">
          <path
            fill="currentColor"
            d="M6 10c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm12 0c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm-6 0c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z"
          />
        </svg>
        <span>{{ t('cart_more') }}</span>
      </button>
      <span class="cart-opened">{{ openedAt }}</span>
    </div>

    <div class="cart-lines scroll-touch">
      <template v-if="order.coursesEnabled">
        <template v-for="group in courseGroups" :key="group.course">
          <div class="course-label" :data-testid="`course-group-${group.course}`">
            {{ t(`course_${group.course}`) }}
          </div>
          <OrderLineItem
            v-for="line in group.lines"
            :key="line.id"
            :line="line"
            :flash="flashFor(line.id)"
            :pending="isDineIn && (line.unsent_quantity ?? 0) > 0"
            :show-state="isDineIn"
            :selected="selectedLineId === line.id"
            show-course
            :can-offer="canOfferLine(line.id)"
            :can-remove-discount="canRemoveLineDiscount(line.id)"
            :can-note="canNote"
            @select="selectLine"
            @note="openNote($event)"
            @increment="increment"
            @decrement="decrement"
            @remove="order.removeLine($event)"
            @offer="discountStep = { kind: 'ask', lineId: $event }"
            @remove-discount="discountStep = { kind: 'pin-remove', lineId: $event }"
            @course="pickingLine = order.lines.find((l) => l.id === $event) ?? null"
          />
        </template>
      </template>
      <template v-else>
        <OrderLineItem
          v-for="line in sentLines"
          :key="line.id"
          :line="line"
          :flash="flashFor(line.id)"
          :show-state="isDineIn"
          :selected="selectedLineId === line.id"
          :can-offer="canOfferLine(line.id)"
          :can-remove-discount="canRemoveLineDiscount(line.id)"
          :can-note="canNote"
          @select="selectLine"
          @note="openNote($event)"
          @increment="increment"
          @decrement="decrement"
          @remove="order.removeLine($event)"
          @offer="discountStep = { kind: 'ask', lineId: $event }"
          @remove-discount="discountStep = { kind: 'pin-remove', lineId: $event }"
        />
        <template v-if="pendingLines.length">
          <div class="pending-label">
            <span class="pending-dot" />{{ t('not_sent_to_kitchen') }}
          </div>
          <OrderLineItem
            v-for="line in pendingLines"
            :key="line.id"
            :line="line"
            :flash="flashFor(line.id)"
            pending
            show-state
            :selected="selectedLineId === line.id"
            :can-offer="canOfferLine(line.id)"
            :can-remove-discount="canRemoveLineDiscount(line.id)"
            :can-note="canNote"
            @select="selectLine"
            @note="openNote($event)"
            @increment="increment"
            @decrement="decrement"
            @remove="order.removeLine($event)"
            @offer="discountStep = { kind: 'ask', lineId: $event }"
            @remove-discount="discountStep = { kind: 'pin-remove', lineId: $event }"
          />
        </template>
      </template>
      <div v-if="order.lines.length === 0" class="cart-empty">
        {{ t('no_items') }}
      </div>
    </div>

    <div class="cart-footer">
      <div class="cart-total">
        <div class="total-block">
          <span class="total-amount">{{ formatPrice(order.totalCents) }} €</span>
          <VatSummary :lines="order.lines" />
        </div>
      </div>

      <p v-if="keptParts > 0" class="held-parts" data-testid="held-parts">
        {{ t('parts_kept', { n: keptParts }) }}
      </p>

      <div v-if="showActionRow" class="cart-actions">
        <button
          v-if="kitchenPrimary"
          class="checkout-btn secondary"
          data-testid="pay-secondary"
          :disabled="order.lines.length === 0"
          @click="goToCheckout"
        >
          <span>{{ t('pay') }}</span>
          <span class="checkout-amount">{{ formatPrice(order.totalCents) }} €</span>
        </button>
        <button
          v-else
          class="kitchen-btn"
          :disabled="order.unsentCount === 0 || sending"
          @click="sendToKitchen"
        >
          {{ t('send_to_kitchen') }}
          <span v-if="order.unsentCount > 0" class="kitchen-count">{{ order.unsentCount }}</span>
        </button>
        <button
          v-if="auth.canTakePayment"
          class="pre-bill-btn"
          :disabled="!canPreBill || preBilling"
          @click="printPreBill"
        >
          {{ t('pre_bill') }}
        </button>
      </div>

      <button
        v-if="isDineIn && order.nextHeldCourse !== null"
        class="next-course-btn"
        :disabled="sendingNext"
        data-testid="send-next-course"
        @click="sendNextCourse"
      >
        {{ t('send_next_course', { course: t(`course_${order.nextHeldCourse}`) }) }}
      </button>

      <!-- UX guide §2.3: the main action, always bottom right. -->
      <button
        v-if="kitchenPrimary"
        class="kitchen-btn primary"
        data-testid="send-primary"
        :disabled="order.unsentCount === 0 || sending"
        @click="sendToKitchen"
      >
        <span>{{ t('send_to_kitchen') }}</span>
        <span v-if="sendableLines > 0" class="kitchen-pill" data-testid="send-pill">{{
          sendPill
        }}</span>
      </button>
      <button
        v-else-if="auth.canTakePayment"
        class="checkout-btn"
        :disabled="order.lines.length === 0"
        @click="goToCheckout"
      >
        <span>{{ t('pay') }}</span>
        <span class="checkout-amount">{{ formatPrice(order.totalCents) }} €</span>
      </button>
    </div>

    <CartMoreMenu
      v-if="moreOpen"
      :can-transfer="canTransfer"
      :can-discount="canDiscount"
      :order-discounted="order.orderDiscounted"
      :can-close-unpaid="canCloseUnpaid"
      @transfer="fromMore(() => (picking = true))"
      @discount="fromMore(() => (discountStep = { kind: 'ask', lineId: null }))"
      @discount-remove="fromMore(() => (discountStep = { kind: 'pin-remove', lineId: null }))"
      @unpaid="fromMore(() => (confirmUnpaid = true))"
      @cancel="moreOpen = false"
    />

    <TableMover v-if="picking" @close="picking = false" />

    <NoteDialog
      v-if="noteLine"
      :initial="noteLine.kitchen_note"
      :chips="quickNotes"
      :sent="noteLine.quantity - (noteLine.unsent_quantity ?? 0) > 0"
      @confirm="saveNote"
      @cancel="noteLineId = null"
    />

    <DiscountDialog
      v-if="discountStep?.kind === 'ask'"
      :base-cents="discountBase"
      :line-id="discountStep.lineId ?? undefined"
      @confirm="discountStep = { kind: 'pin-apply', input: $event }"
      @cancel="discountStep = null"
    />

    <PinDialog
      v-if="
        (discountStep?.kind === 'pin-apply' || discountStep?.kind === 'pin-remove') &&
        auth.currentUser
      "
      :insz="auth.currentUser.insz"
      :user-name="auth.currentUser.name"
      :hint="discountStep.kind === 'pin-apply' ? t('discount_hint') : t('discount_remove_hint')"
      :verify="verifyDiscount"
      @done="onDiscountDone"
      @cancel="discountStep = null"
    />

    <CoursePickerDialog
      v-if="pickingLine"
      :current="pickingLine.course"
      @pick="pickCourse"
      @cancel="pickingLine = null"
    />

    <PinDialog
      v-if="confirmUnpaid && auth.currentUser"
      :insz="auth.currentUser.insz"
      :user-name="auth.currentUser.name"
      :hint="t('unpaid_hint')"
      :verify="closeUnpaid"
      @done="onUnpaidClosed"
      @cancel="confirmUnpaid = false"
    />
  </aside>
</template>

<style scoped>
.cart {
  width: var(--cart-width);
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border);
}

.cart-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px 10px;
  border-bottom: 1px solid var(--border);
}
.cart-header h2 {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}
.more-btn {
  margin-left: auto;
  height: 48px;
  min-width: 48px;
  padding: 0 14px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  white-space: nowrap;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  transition: transform var(--tap-duration) ease;
}
.more-btn:active {
  transform: scale(var(--tap-scale));
}
.cart-opened {
  font-family: var(--font-mono);
  font-size: 15px;
  color: var(--text-secondary);
  white-space: nowrap;
}

.cart-lines {
  flex: 1;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
  overscroll-behavior-y: contain;
  padding: 0 16px;
}

.pending-label {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 0 4px;
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--accent-warning);
}
.pending-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--accent-warning);
}

/* Course header of the grouped cart. */
.course-label {
  padding: 14px 0 4px;
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.cart-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 120px;
  color: var(--text-secondary);
  font-size: var(--font-size-md);
}

.cart-footer {
  border-top: 1px solid var(--border);
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.held-parts {
  margin: 0;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  color: var(--accent-warning);
}

.cart-total {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: 16px;
}
.total-block {
  margin-left: auto;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}
.total-amount {
  font-family: var(--font-mono);
  font-size: 40px;
  font-weight: var(--font-weight-bold);
  line-height: 1.1;
  font-variant-numeric: tabular-nums;
}

.cart-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.kitchen-btn,
.pre-bill-btn {
  height: 64px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  transition: transform var(--tap-duration) ease;
}
.kitchen-btn:active,
.pre-bill-btn:active {
  transform: scale(var(--tap-scale));
}
.kitchen-btn:disabled,
.pre-bill-btn:disabled {
  opacity: 0.35;
  pointer-events: none;
}
.kitchen-count {
  min-width: 26px;
  height: 26px;
  padding: 0 8px;
  border-radius: 13px;
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-family: var(--font-mono);
  font-size: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

/* Volt primary while lines wait for the kitchen (UX audit C4). */
.kitchen-btn.primary {
  width: 100%;
  height: 80px;
  padding: 0 28px;
  background: var(--accent-primary);
  color: var(--text-on-accent);
  font-size: 24px;
  font-weight: var(--font-weight-heavy);
  letter-spacing: 0.04em;
  justify-content: space-between;
}
.kitchen-btn.primary:active {
  filter: brightness(0.9);
}
.kitchen-btn.primary:disabled {
  opacity: 0.3;
}
.kitchen-pill {
  min-width: 26px;
  height: 36px;
  padding: 0 14px;
  border-radius: 18px;
  background: rgba(0, 0, 0, 0.2);
  color: var(--text-on-accent);
  font-family: var(--font-mono);
  font-size: 18px;
  letter-spacing: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

/* Releases the next held course: the kitchen's second action, above Pay. */
.next-course-btn {
  height: 64px;
  border-radius: var(--radius-md);
  background: var(--selected-bg);
  color: var(--selected-fg);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.next-course-btn:active {
  transform: scale(var(--tap-scale));
}
.next-course-btn:disabled {
  opacity: 0.35;
  pointer-events: none;
}

/* Volt: the screen's one main action. Full width × 80 px (§4.1 ≥ 72). */
.checkout-btn {
  width: 100%;
  height: 80px;
  padding: 0 28px;
  border-radius: var(--radius-md);
  background: var(--accent-primary);
  color: var(--text-on-accent);
  font-size: 24px;
  font-weight: var(--font-weight-heavy);
  letter-spacing: 0.04em;
  display: flex;
  align-items: center;
  justify-content: space-between;
  transition: transform var(--tap-duration) ease;
}
.checkout-amount {
  font-family: var(--font-mono);
  font-weight: var(--font-weight-bold);
}
/* Secondary PAYER (lines wait for the kitchen): outlined, still shows the total. */
.checkout-btn.secondary {
  height: 64px;
  padding: 0 20px;
  background: transparent;
  border: 2px solid var(--text-secondary);
  color: var(--text-primary);
  font-size: 18px;
  letter-spacing: 0.02em;
}
.checkout-btn.secondary:active {
  filter: none;
  background: var(--bg-pressed);
}
.checkout-btn:disabled {
  opacity: 0.3;
  pointer-events: none;
}
.checkout-btn:active {
  transform: scale(var(--tap-scale));
  filter: brightness(0.9);
}
</style>
