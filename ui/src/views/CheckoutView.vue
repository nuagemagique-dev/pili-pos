<script setup lang="ts">
import { describeError } from "../utils/errorMessages";
import { ref, computed, watch } from "vue";
import { useRouter } from "vue-router";
import { useOrderStore, warnIfSlipFailed } from "../stores/order";
import { useDeliveryStore } from "../stores/delivery";
import type { CheckoutResult, PaymentPart } from "../types";
import * as ipc from "../api/ipc";
import { useI18n } from "../i18n";
import { formatEuros } from "../utils/format";
import TicketMediumPicker from "../components/TicketMediumPicker.vue";
import DeliveryOutcomePanel from "../components/DeliveryOutcomePanel.vue";

const router = useRouter();
const order = useOrderStore();
const delivery = useDeliveryStore();
const { t } = useI18n();

// A sale can be paid in several parts: card and meal vouchers for an
// exact amount each, cash for the rest (the only part rounded to
// 5 cents). Nothing is signed until the rest is paid; the backend
// settles the parts again and refuses a payment that does not add up.
type ExactMethod = "card" | "meal_voucher";

const step = ref<"method" | "cash" | "card" | "processing" | "done">("method");
const result = ref<CheckoutResult | null>(null);
const error = ref<string | null>(null);
const cashReceived = ref(0);
// Change handed back, fixed when the sale is recorded: the done screen
// cannot recompute it, the cart (and so the total) is cleared by then.
const changeDue = ref(0);
const cashInput = ref("");
// Parts taken so far (card, meal vouchers), in cents.
const parts = ref<{ method: ExactMethod; cents: number }[]>([]);
// The exact method being entered, and its amount as typed ("12,50").
const exactMethod = ref<ExactMethod>("card");
const exactInput = ref("");

// The picker needs a fresh mail probe every time it's about to be
// shown - cheap, the backend caches the probe for 60s.
watch(step, (s) => { if (s === "method") delivery.refreshMailStatus(); }, { immediate: true });

const paidCents = computed(() => parts.value.reduce((sum, p) => sum + p.cents, 0));
const restCents = computed(() => Math.max(0, order.totalCents - paidCents.value));
// UX Guide §7.5: arrondi belge 5 centimes, on the cash rest only.
const roundedRestCents = computed(() => Math.round(restCents.value / 5) * 5);

const totalEur = computed(() => order.totalCents / 100);
const restEur = computed(() => restCents.value / 100);
const roundedTotal = computed(() => roundedRestCents.value / 100);

/** "12,50" -> 1250 cents; 0 when empty or not a number. */
function toCents(input: string): number {
  if (!input) return 0;
  const parsed = parseFloat(input.replace(",", "."));
  return isNaN(parsed) ? 0 : Math.round(parsed * 100);
}

function toInput(cents: number): string {
  return (cents / 100).toFixed(2).replace(".", ",");
}

const inputAmount = computed(() => toCents(cashInput.value) / 100);

const changeAmount = computed(() => {
  const diff = toCents(cashInput.value) - roundedRestCents.value;
  return diff > 0 ? diff / 100 : 0;
});

const canValidate = computed(() => toCents(cashInput.value) >= roundedRestCents.value);

const exactCents = computed(() => toCents(exactInput.value));
const exactTooHigh = computed(() => exactCents.value > restCents.value);
const canConfirmExact = computed(() => exactCents.value > 0 && !exactTooHigh.value);

// Quick shortcuts: paliers logiques au-dessus du total
const shortcuts = computed(() => {
  const t = roundedTotal.value;
  const result: number[] = [];
  const next5 = Math.ceil(t / 5) * 5;
  if (next5 > t) result.push(next5);
  const next10 = Math.ceil(t / 10) * 10;
  if (next10 > t && !result.includes(next10)) result.push(next10);
  for (const v of [50, 100, 200]) {
    if (v > t && !result.includes(v)) result.push(v);
  }
  return result.slice(0, 4);
});

const formatPrice = formatEuros;

/** Numpad input into `target`: digits, one comma, 2 decimals, 6 chars. */
function typeInto(target: typeof cashInput, value: string) {
  if (value === "C") { target.value = ""; return; }
  if (value === "BS") { target.value = target.value.slice(0, -1); return; }
  if (value === ",") {
    // Only one comma allowed, and not as first char
    if (target.value.includes(",") || !target.value) return;
    target.value += ",";
    return;
  }
  // Max 2 digits after comma
  const commaIdx = target.value.indexOf(",");
  if (commaIdx >= 0 && target.value.length - commaIdx > 2) return;
  // Max 6 chars total (e.g. "200,10")
  if (target.value.length < 6) { target.value += value; }
}

function handleKey(value: string) {
  typeInto(cashInput, value);
}

function handleExactKey(value: string) {
  typeInto(exactInput, value);
}

function setQuickAmount(n: number) {
  cashInput.value = n.toString();
}

function openCash() {
  cashInput.value = "";
  step.value = "cash";
}

/** Card or meal vouchers: the amount starts at the rest, can be lowered. */
function openExact(method: ExactMethod) {
  exactMethod.value = method;
  exactInput.value = toInput(restCents.value);
  step.value = "card";
}

function clearParts() {
  parts.value = [];
  error.value = null;
}

function payload(withCash: boolean): PaymentPart[] {
  const list: PaymentPart[] = parts.value.map((p) => ({ method: p.method, amount_cents: p.cents }));
  if (withCash) list.push({ method: "cash" });
  return list;
}

async function payCash() {
  if (!canValidate.value) return;
  cashReceived.value = inputAmount.value;
  step.value = "processing";
  await doCheckout(payload(true));
}

function payExact() {
  cashReceived.value = roundedTotal.value;
  step.value = "processing";
  doCheckout(payload(true));
}

/** Confirm the card or meal voucher amount: the sale is signed once
 *  nothing is left to pay, otherwise back to the methods for the rest. */
function confirmExact() {
  if (!canConfirmExact.value) return;
  parts.value.push({ method: exactMethod.value, cents: exactCents.value });
  if (restCents.value > 0) {
    step.value = "method";
    return;
  }
  cashReceived.value = 0;
  step.value = "processing";
  doCheckout(payload(false));
}

async function doCheckout(payments: PaymentPart[]) {
  if (!order.currentOrder) return;
  error.value = null;
  const withCash = payments.some((p) => p.method === "cash");
  const cashDueCents = roundedRestCents.value;
  try {
    const req = delivery.request.medium === "paper" ? undefined : delivery.request;
    result.value = await ipc.checkout(order.currentOrder.id, payments, req);
    if (result.value.success) {
      warnIfSlipFailed(result.value.kitchen_slip);
      // In cents: no float leftovers such as 9,999999.
      changeDue.value = withCash
        ? Math.max(0, Math.round(cashReceived.value * 100) - cashDueCents) / 100
        : 0;
      // Clear the order from the store immediately so it can't be
      // re-submitted. The "done" screen reads from result.value.
      order.clearOrder();
      parts.value = [];
      // Next customer starts from Papier.
      delivery.reset();
      step.value = "done";
    } else {
      error.value = describeError(result.value.error);
      step.value = "method";
    }
  } catch (e) {
    error.value = describeError(e);
    step.value = "method";
  }
}

function finish() {
  result.value = null;
  router.push({ name: "order" });
}
</script>

<template>
  <div class="checkout">

    <!-- Step 1: Cash, card or meal vouchers (several parts allowed) -->
    <div v-if="step === 'method'" class="checkout-center">
      <div class="total-block">
        <span class="total-label">{{ parts.length ? t('rest_to_pay') : t('total') }}</span>
        <span class="total-value">{{ formatPrice(roundedTotal) }}</span>
        <span v-if="roundedTotal !== restEur" class="total-exact">
          ({{ formatPrice(restEur) }} {{ t('before_rounding') }})
        </span>
      </div>
      <ul v-if="parts.length" class="paid-parts">
        <li class="paid-part total-line">
          <span>{{ t('total') }}</span><span>{{ formatPrice(totalEur) }}</span>
        </li>
        <li v-for="(p, i) in parts" :key="i" class="paid-part">
          <span>{{ t(p.method) }}</span><span>{{ formatPrice(p.cents / 100) }}</span>
        </li>
      </ul>
      <p v-if="error" class="error-msg shake">{{ error }}</p>
      <TicketMediumPicker v-if="!parts.length" />
      <div class="pay-methods">
        <button class="pay-btn cash-btn" @click="openCash">{{ t('cash') }}</button>
        <button class="pay-btn card-btn" @click="openExact('card')">{{ t('card') }}</button>
        <button class="pay-btn meal-btn" @click="openExact('meal_voucher')">{{ t('meal_voucher') }}</button>
      </div>
      <button v-if="parts.length" class="back-link parts-clear" @click="clearParts">{{ t('clear_payments') }}</button>
      <button v-else class="back-link" @click="router.push({ name: 'order' })">{{ t('back') }}</button>
    </div>

    <!-- Step 2: Cash - numpad + shortcuts -->
    <div v-if="step === 'cash'" class="cash-layout">
      <div class="cash-left">
        <div class="cash-total">
          {{ t('to_pay') }}: <strong>{{ formatPrice(roundedTotal) }}</strong>
        </div>

        <div class="cash-input-display">
          <span class="cash-input-label">{{ t('received') }}</span>
          <span class="cash-input-value">{{ cashInput || '0' }}<span v-if="cashInput && !cashInput.includes(',')" class="cash-eur">€</span></span>
        </div>

        <div v-if="canValidate" class="cash-change">
          {{ t('change') }}: <strong>{{ formatPrice(changeAmount) }}</strong>
        </div>

        <!-- Shortcuts -->
        <div class="cash-shortcuts">
          <button class="shortcut exact" @click="payExact">{{ t('exact') }}</button>
          <button
            v-for="s in shortcuts"
            :key="s"
            class="shortcut"
            @click="setQuickAmount(s)"
          >{{ s }}</button>
        </div>
      </div>

      <div class="cash-right">
        <!-- Numpad -->
        <div class="numpad">
          <button v-for="d in ['1','2','3','4','5','6','7','8','9']" :key="d" class="nk" @click="handleKey(d)">{{ d }}</button>
          <button class="nk nk-comma" @click="handleKey(',')">,</button>
          <button class="nk" @click="handleKey('0')">0</button>
          <button class="nk nk-back" @click="handleKey('BS')">&#9003;</button>
        </div>
        <button class="nk-wide nk-clear" @click="handleKey('C')">{{ t('clear') }}</button>

        <div class="cash-actions">
          <button class="act-btn secondary" @click="step = 'method'">{{ t('back') }}</button>
          <button class="act-btn primary" :disabled="!canValidate" @click="payCash">{{ t('validate') }}</button>
        </div>
      </div>
    </div>

    <!-- Step 2b: Card or meal vouchers - the amount (the rest by
         default), taken on the terminal, then confirmed -->
    <div v-if="step === 'card'" class="cash-layout exact-layout">
      <div class="cash-left">
        <div class="cash-total">
          {{ t(exactMethod) }} · {{ t('rest_to_pay') }}: <strong>{{ formatPrice(restEur) }}</strong>
        </div>
        <div class="cash-input-display">
          <span class="cash-input-label">{{ t('amount') }}</span>
          <span class="cash-input-value exact-value">{{ exactInput || '0' }}</span>
        </div>
        <p v-if="exactTooHigh" class="exact-too-high">{{ t('amount_over_rest') }}</p>
        <p class="card-hint">{{ t('card_wait') }}</p>
      </div>
      <div class="cash-right">
        <div class="numpad">
          <button v-for="d in ['1','2','3','4','5','6','7','8','9']" :key="d" class="nk" @click="handleExactKey(d)">{{ d }}</button>
          <button class="nk nk-comma" @click="handleExactKey(',')">,</button>
          <button class="nk" @click="handleExactKey('0')">0</button>
          <button class="nk nk-back" @click="handleExactKey('BS')">&#9003;</button>
        </div>
        <button class="nk-wide nk-clear" @click="handleExactKey('C')">{{ t('clear') }}</button>
        <div class="cash-actions">
          <button class="act-btn secondary back-link-exact" @click="step = 'method'">{{ t('back') }}</button>
          <button class="act-btn primary card-confirm" :disabled="!canConfirmExact" @click="confirmExact">{{ t('validate') }}</button>
        </div>
      </div>
    </div>

    <!-- Step 3: Processing -->
    <div v-if="step === 'processing'" class="checkout-center">
      <div class="spinner"></div>
      <p class="processing-text">{{ t('processing_fdm') }}</p>
    </div>

    <!-- Step 4: Done -->
    <div v-if="step === 'done'" class="checkout-center">
      <div class="success-icon">OK</div>
      <div v-if="result?.ticket" class="receipt-mini">
        <div class="receipt-label">{{ result.ticket.ticket_type_label }}</div>
        <div class="receipt-total">{{ formatPrice(result.ticket.total) }}</div>
        <div v-if="changeDue > 0" class="receipt-change">
          {{ t('change') }}: {{ formatPrice(changeDue) }}
        </div>
        <div v-if="result.ticket.fdm_ref" class="receipt-fdm">
          FDM: {{ result.ticket.fdm_ref.fdmId }}
        </div>
      </div>
      <div v-if="!result?.fdm_connected" class="fdm-warn">
        {{ t('fdm_no_sig') }}
      </div>
      <DeliveryOutcomePanel
        :outcome="result?.delivery ?? null"
        :ticket="result?.ticket ?? null"
        :fiscal-event-id="result?.fiscal_event_id ?? null"
      />
      <div class="done-actions">
        <button class="act-btn primary" @click="finish">{{ t('finish') }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.checkout {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-primary);
  padding: var(--space-md);
}

.checkout-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-lg);
  max-width: 600px;
  width: 100%;
}

/* ── Total display ── */
.total-block { text-align: center; }
.total-label { display: block; font-size: var(--font-size-lg); color: var(--text-secondary); }
.total-value { display: block; font-size: 56px; font-weight: var(--font-weight-bold); color: var(--text-primary); font-variant-numeric: tabular-nums; line-height: 1.1; }
.total-exact { font-size: var(--font-size-sm); color: var(--text-secondary); }

.error-msg { color: var(--accent-danger); font-size: var(--font-size-sm); padding: var(--space-sm) var(--space-md); background: rgba(231,76,60,0.1); border-radius: var(--radius-sm); border: 1px solid var(--accent-danger); width: 100%; text-align: center; }

/* ── Pay method buttons ── */
.pay-methods { display: flex; gap: var(--space-md); width: 100%; }
.pay-btn { flex: 1; height: 120px; font-size: var(--font-size-xl); font-weight: var(--font-weight-bold); border-radius: var(--radius-lg); color: var(--text-on-accent); transition: transform var(--tap-duration) ease; letter-spacing: 0.05em; }
.pay-btn:active { transform: scale(var(--tap-scale)); }
.cash-btn { background: var(--accent-success); }
.meal-btn { background: var(--bg-surface); color: var(--text-primary); }
.paid-parts { list-style: none; width: 100%; display: flex; flex-direction: column; gap: var(--space-xs); font-size: var(--font-size-md); font-variant-numeric: tabular-nums; }
.paid-part { display: flex; justify-content: space-between; padding: var(--space-xs) var(--space-md); background: var(--bg-surface); border-radius: var(--radius-sm); }
.paid-part.total-line { background: transparent; color: var(--text-secondary); }
.exact-too-high { color: var(--accent-danger); font-size: var(--font-size-sm); font-weight: var(--font-weight-bold); }
.card-btn { background: var(--bg-surface); color: var(--text-primary); }

.back-link { font-size: var(--font-size-sm); color: var(--text-secondary); padding: var(--space-sm); }

.card-hint { font-size: var(--font-size-md); color: var(--text-secondary); text-align: center; }

/* §4.1: primary action full-width × 120px, same as pay-btn */

/* ── Cash layout: left info + right numpad ── */
.cash-layout {
  display: flex;
  gap: var(--space-xl);
  align-items: flex-start;
  max-width: 800px;
  width: 100%;
}

.cash-left {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.cash-total {
  font-size: var(--font-size-lg);
  color: var(--text-secondary);
}
.cash-total strong {
  color: var(--text-primary);
  font-size: var(--font-size-xl);
}

.cash-input-display {
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
  padding: var(--space-lg);
  text-align: center;
}

.cash-input-label {
  display: block;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  margin-bottom: var(--space-xs);
}

.cash-input-value {
  font-size: 48px;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.cash-eur {
  font-size: 28px;
  color: var(--text-secondary);
  margin-left: 4px;
}

.cash-change {
  font-size: var(--font-size-xl);
  color: var(--accent-success);
  text-align: center;
}
.cash-change strong {
  font-size: 36px;
}

.cash-shortcuts {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-sm);
}

.shortcut {
  height: 56px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.shortcut:active { transform: scale(var(--tap-scale)); }
.shortcut.exact { background: var(--accent-success); color: var(--text-on-accent); grid-column: 1 / -1; }

.cash-right {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

/* ── Numpad ── */
.numpad {
  display: grid;
  grid-template-columns: repeat(3, 96px);
  gap: var(--space-xs);
}

.nk {
  height: 72px;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.nk:active { transform: scale(var(--tap-scale)); background: var(--bg-pressed); }
.nk-comma { color: var(--text-secondary); font-size: 32px; }
.nk-back { color: var(--accent-danger); }

.nk-wide {
  width: 100%;
  height: 48px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  color: var(--accent-warning);
  transition: transform var(--tap-duration) ease;
}
.nk-wide:active { transform: scale(var(--tap-scale)); }

.cash-actions {
  display: flex;
  gap: var(--space-sm);
}

/* ── Shared buttons ── */
.act-btn {
  flex: 1;
  height: 72px;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  transition: transform var(--tap-duration) ease;
}
.act-btn:active { transform: scale(var(--tap-scale)); }
.act-btn:disabled { opacity: 0.3; pointer-events: none; }
.act-btn.primary { background: var(--accent-primary); color: var(--text-on-accent); }
.act-btn.secondary { background: var(--bg-surface); color: var(--text-primary); }

/* ── Processing ── */
.spinner { width: 48px; height: 48px; border: 4px solid var(--bg-surface); border-top: 4px solid var(--accent-primary); border-radius: 50%; animation: spin 0.8s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.processing-text { color: var(--text-secondary); font-size: var(--font-size-md); }

/* ── Done ── */
.success-icon { width: 80px; height: 80px; border-radius: 50%; background: var(--accent-success); color: var(--text-on-accent); font-size: var(--font-size-xl); font-weight: var(--font-weight-bold); display: flex; align-items: center; justify-content: center; }
.receipt-mini { text-align: center; }
.receipt-label { font-size: var(--font-size-sm); color: var(--text-secondary); }
.receipt-total { font-size: var(--font-size-xl); font-weight: var(--font-weight-bold); }
.receipt-change { font-size: var(--font-size-lg); color: var(--accent-success); font-weight: var(--font-weight-bold); }
.receipt-fdm { font-size: var(--font-size-xs); color: var(--text-secondary); margin-top: var(--space-xs); }
.fdm-warn { padding: var(--space-sm) var(--space-md); background: rgba(243,156,18,0.1); color: var(--accent-warning); border-radius: var(--radius-sm); font-size: var(--font-size-sm); }
.done-actions { display: flex; gap: var(--space-md); width: 100%; }
</style>
