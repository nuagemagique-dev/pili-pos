<script setup lang="ts">
import { describeError } from "../utils/errorMessages";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useOrderStore } from "../stores/order";
import { useAuthStore } from "../stores/auth";
import { useToastStore } from "../stores/toast";
import { useI18n } from "../i18n";
import { formatPrice } from "../utils/format";
import { lineNetCents } from "../utils/cartAmounts";
import { formatTimestamp, formatTime, minutesSince } from "../utils/datetime";
import OrderLineItem from "./OrderLineItem.vue";
import VatSummary from "./VatSummary.vue";
import PinDialog from "./PinDialog.vue";
import TablePickerDialog from "./TablePickerDialog.vue";
import DiscountDialog from "./DiscountDialog.vue";
import NoteDialog from "./NoteDialog.vue";
import type { DiscountInput } from "../types";
import * as ipc from "../api/ipc";

const router = useRouter();
const order = useOrderStore();
const auth = useAuthStore();
const toast = useToastStore();
const { t } = useI18n();

const isDineIn = computed(() => order.currentOrder?.order_type === "dine_in");

// Dine-in lines not yet sent to the bar / kitchen go last, under an
// amber label; a takeaway's lines all go out with the sale.
const sentLines = computed(() =>
  isDineIn.value ? order.lines.filter((l) => (l.unsent_quantity ?? 0) <= 0) : order.lines,
);
const pendingLines = computed(() =>
  isDineIn.value ? order.lines.filter((l) => (l.unsent_quantity ?? 0) > 0) : [],
);

// "19:05 · 37 min": when the order was opened, and for how long.
const now = ref(new Date());
let nowTimer: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
  nowTimer = setInterval(() => (now.value = new Date()), 30_000);
});
onUnmounted(() => clearInterval(nowTimer));
const openedAt = computed(() => {
  const created = order.currentOrder?.created_at;
  if (!created) return "";
  return `${formatTimestamp(created, formatTime)} · ${minutesSince(created, now.value)} min`;
});

const cartTitle = computed(() => {
  const current = order.currentOrder;
  if (current?.table_number) return `${t("order")} · ${t("table")} ${current.table_number}`;
  if (current?.order_type === "takeaway") return `${t("order")} · ${t("takeaway_badge")}`;
  return t("order");
});

function increment(lineId: number) {
  const line = order.lines.find((l) => l.id === lineId);
  if (line) void order.updateQuantity(lineId, line.quantity + 1);
}

function decrement(lineId: number) {
  const line = order.lines.find((l) => l.id === lineId);
  if (line && line.quantity > 1) void order.updateQuantity(lineId, line.quantity - 1);
}

// Send what was ordered to the bar / kitchen without leaving the table
// (event P, the same registration as when leaving the order).
const sending = ref(false);

async function sendToKitchen() {
  if (sending.value || order.unsentCount === 0) return;
  sending.value = true;
  const count = order.unsentCount;
  try {
    await order.leave("send_to_kitchen");
    toast.show(t("sent_to_kitchen", { n: count }), "success");
  } catch (e) {
    toast.show(`${t("order_register_failed")} : ${describeError(e)}`, "error", 6000);
  } finally {
    sending.value = false;
  }
}

function goToCheckout() {
  if (order.lines.length > 0) {
    router.push({ name: "checkout" });
  }
}

// Provisional bill for a table (§1.4). Takeaway sales are paid at once.
const canPreBill = computed(
  () => order.currentOrder?.order_type === "dine_in" && order.lines.length > 0,
);
const preBilling = ref(false);

async function printPreBill() {
  if (!canPreBill.value || preBilling.value) return;
  preBilling.value = true;
  try {
    await order.preBill();
    toast.show(t("pre_bill_printed"), "success");
  } catch (e) {
    toast.show(`${t("pre_bill_failed")} : ${describeError(e)}`, "error", 6000);
  } finally {
    preBilling.value = false;
  }
}

// Move the order to another table (event P, cost center change): the
// free tables of the plan, current one left out.
const canTransfer = computed(
  () =>
    isDineIn.value &&
    order.currentOrder?.status === "open" &&
    order.currentOrder.table_number != null &&
    order.lines.length > 0,
);
const picking = ref(false);
const freeTables = ref<ipc.TableInfo[]>([]);
const transferring = ref(false);

async function openTablePicker() {
  if (!canTransfer.value || transferring.value) return;
  try {
    const current = order.currentOrder?.table_number;
    freeTables.value = (await ipc.listTables()).filter(
      (tbl) => tbl.status === "free" && tbl.tableNumber !== current,
    );
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
    return;
  }
  picking.value = true;
}

async function transferTo(toTable: number) {
  if (transferring.value) return;
  transferring.value = true;
  try {
    await order.transferTable(toTable);
    picking.value = false;
    toast.show(t("transfer_done", { to: toTable }), "success");
  } catch (e) {
    picking.value = false;
    toast.show(describeError(e), "error", 6000);
  } finally {
    transferring.value = false;
  }
}

// Kitchen note of a line: any staff member, on an open order. The dialog
// says whether the note prints now (the line already went to the kitchen)
// or with the order.
const canNote = computed(() => order.currentOrder?.status === "open");
const noteLineId = ref<number | null>(null);
const noteLine = computed(() => order.lines.find((l) => l.id === noteLineId.value) ?? null);

async function saveNote(text: string) {
  const id = noteLineId.value;
  noteLineId.value = null;
  if (id !== null) await order.setLineNote(id, text);
}

// Discount at the till (manager, PIN): an order discount, a line for
// free, or the removal of either. The dialog collects the input, the PIN
// confirms it, the engine applies every rule.
// An online order has its own discounts: the engine refuses one at the
// till (discount_online_order), so the button and the line actions hide.
const canDiscount = computed(
  () =>
    auth.isManager &&
    order.currentOrder?.status === "open" &&
    order.lines.length > 0 &&
    !order.lines.some((l) => l.online === true),
);
type DiscountStep =
  | { kind: "ask"; lineId: number | null }
  | { kind: "pin-apply"; input: DiscountInput }
  | { kind: "pin-remove"; lineId: number | null };
const discountStep = ref<DiscountStep | null>(null);

function lineHasOwnChange(lineId: number): boolean {
  const line = order.lines.find((l) => l.id === lineId);
  return !!line?.price_changes?.some((c) => c.scope === "LINE");
}
function canOfferLine(lineId: number): boolean {
  const line = order.lines.find((l) => l.id === lineId);
  return canDiscount.value && !order.orderDiscounted && (line?.price_changes?.length ?? 0) === 0;
}
function canRemoveLineDiscount(lineId: number): boolean {
  return canDiscount.value && lineHasOwnChange(lineId);
}

/** What the open dialog applies to: the line's amount, or the order total. */
const discountBase = computed(() => {
  const step = discountStep.value;
  if (step?.kind === "ask" && step.lineId !== null) {
    const line = order.lines.find((l) => l.id === step.lineId);
    return line ? lineNetCents(line) : 0;
  }
  return order.totalCents;
});

async function verifyDiscount(pin: string) {
  const step = discountStep.value;
  if (step?.kind === "pin-apply") await order.applyDiscount(step.input, pin);
  else if (step?.kind === "pin-remove") await order.removeDiscount(step.lineId, pin);
}

function onDiscountDone() {
  const removed = discountStep.value?.kind === "pin-remove";
  discountStep.value = null;
  toast.show(t(removed ? "discount_removed" : "discount_applied"), "success");
}

// The customer left without paying (UC4109): a manager closes the
// order, confirmed by their PIN; the sale is signed with no payment.
const canCloseUnpaid = computed(
  () => auth.isManager && order.currentOrder?.status === "open" && order.lines.length > 0,
);
const confirmUnpaid = ref(false);

async function closeUnpaid(pin: string) {
  await order.closeUnpaid(pin);
}

async function onUnpaidClosed() {
  confirmUnpaid.value = false;
  toast.show(t("unpaid_closed"), "success");
  await router.push({ name: "tables" });
}
</script>

<template>
  <!-- UX guide §3: the cart always on the right, 670 px. -->
  <aside class="cart">
    <div class="cart-header">
      <h2>{{ cartTitle }}</h2>
      <button
        v-if="canTransfer"
        class="transfer-btn"
        data-testid="transfer-table"
        :disabled="transferring"
        @click="openTablePicker"
      >
        {{ t('transfer_table') }}
      </button>
      <span class="cart-opened">{{ openedAt }}</span>
    </div>

    <div class="cart-lines scroll-touch">
      <OrderLineItem
        v-for="line in sentLines"
        :key="line.id"
        :line="line"
        :can-offer="canOfferLine(line.id)"
        :can-remove-discount="canRemoveLineDiscount(line.id)"
        :can-note="canNote"
        @note="noteLineId = $event"
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
          pending
          :can-offer="canOfferLine(line.id)"
          :can-remove-discount="canRemoveLineDiscount(line.id)"
          :can-note="canNote"
          @note="noteLineId = $event"
          @increment="increment"
          @decrement="decrement"
          @remove="order.removeLine($event)"
        />
      </template>
      <div v-if="order.lines.length === 0" class="cart-empty">
        {{ t('no_items') }}
      </div>
    </div>

    <div class="cart-footer">
      <div class="cart-total">
        <div v-if="canCloseUnpaid || canDiscount" class="manager-actions">
          <button
            v-if="canDiscount && !order.orderDiscounted"
            class="discount-btn"
            data-testid="discount-open"
            @click="discountStep = { kind: 'ask', lineId: null }"
          >
            {{ t('discount_button') }}
          </button>
          <button
            v-if="canDiscount && order.orderDiscounted"
            class="discount-btn"
            data-testid="discount-remove"
            @click="discountStep = { kind: 'pin-remove', lineId: null }"
          >
            {{ t('discount_remove') }}
          </button>
          <button v-if="canCloseUnpaid" class="unpaid-btn" @click="confirmUnpaid = true">
            {{ t('unpaid_close') }}
          </button>
        </div>
        <div class="total-block">
          <span class="total-amount">{{ formatPrice(order.totalCents) }} €</span>
          <VatSummary :lines="order.lines" />
        </div>
      </div>

      <div v-if="isDineIn" class="cart-actions">
        <button
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

      <!-- UX guide §2.3: the main action, always bottom right. -->
      <button
        v-if="auth.canTakePayment"
        class="checkout-btn"
        :disabled="order.lines.length === 0"
        @click="goToCheckout"
      >
        <span>{{ t('pay') }}</span>
        <span class="checkout-amount">{{ formatPrice(order.totalCents) }} €</span>
      </button>
    </div>

    <TablePickerDialog
      v-if="picking && order.currentOrder?.table_number != null"
      :tables="freeTables"
      :from="order.currentOrder.table_number"
      @confirm="transferTo"
      @cancel="picking = false"
    />

    <NoteDialog
      v-if="noteLine"
      :initial="noteLine.kitchen_note"
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
      v-if="(discountStep?.kind === 'pin-apply' || discountStep?.kind === 'pin-remove') && auth.currentUser"
      :insz="auth.currentUser.insz"
      :user-name="auth.currentUser.name"
      :hint="discountStep.kind === 'pin-apply' ? t('discount_hint') : t('discount_remove_hint')"
      :verify="verifyDiscount"
      @done="onDiscountDone"
      @cancel="discountStep = null"
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
  align-items: baseline;
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
.transfer-btn {
  margin-left: auto;
  height: 48px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  white-space: nowrap;
  transition: transform var(--tap-duration) ease;
}
.transfer-btn:active { transform: scale(var(--tap-scale)); }
.transfer-btn:disabled { opacity: 0.35; pointer-events: none; }
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

.manager-actions { display: flex; flex-direction: column; gap: 8px; align-items: flex-start; }
.discount-btn {
  height: 48px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  white-space: nowrap;
  transition: transform var(--tap-duration) ease;
}
.discount-btn:active { transform: scale(var(--tap-scale)); }

/* Rare manager action: small, outlined, apart from the payment buttons. */
.unpaid-btn {
  height: 48px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  border: 1px solid var(--accent-danger);
  color: var(--accent-danger);
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.unpaid-btn:active { transform: scale(var(--tap-scale)); }

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
.pre-bill-btn:active { transform: scale(var(--tap-scale)); }
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
.checkout-btn:disabled {
  opacity: 0.3;
  pointer-events: none;
}
.checkout-btn:active {
  transform: scale(var(--tap-scale));
  filter: brightness(0.9);
}
</style>
