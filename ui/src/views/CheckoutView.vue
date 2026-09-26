<script setup lang="ts">
import { describeError } from "../utils/errorMessages";
import { ref, computed, watch } from "vue";
import { useRouter } from "vue-router";
import { useOrderStore } from "../stores/order";
import { useDeliveryStore } from "../stores/delivery";
import type { CheckoutPaymentMethod, CheckoutResult } from "../types";
import * as ipc from "../api/ipc";
import { useI18n } from "../i18n";
import { formatEuros } from "../utils/format";
import TicketMediumPicker from "../components/TicketMediumPicker.vue";
import DeliveryOutcomePanel from "../components/DeliveryOutcomePanel.vue";

const router = useRouter();
const order = useOrderStore();
const delivery = useDeliveryStore();
const { t } = useI18n();

const step = ref<"method" | "cash" | "card" | "processing" | "done">("method");
const result = ref<CheckoutResult | null>(null);
const error = ref<string | null>(null);
const cashReceived = ref(0);
// Change handed back, fixed when the sale is recorded: the done screen
// cannot recompute it, the cart (and so the total) is cleared by then.
const changeDue = ref(0);
const cashInput = ref("");

// The picker needs a fresh mail probe every time it's about to be
// shown - cheap, the backend caches the probe for 60s.
watch(step, (s) => { if (s === "method") delivery.refreshMailStatus(); }, { immediate: true });

const totalEur = computed(() => order.totalCents / 100);

// UX Guide §7.5: arrondi belge 5 centimes
const roundedTotal = computed(() => Math.round(totalEur.value * 20) / 20);

const inputAmount = computed(() => {
  if (!cashInput.value) return 0;
  // Parse "5010" as 50.10, "50" as 50.00
  // Input is in cents internally: digits before comma = euros, after = cents
  const raw = cashInput.value.replace(",", ".");
  const parsed = parseFloat(raw);
  return isNaN(parsed) ? 0 : parsed;
});

const changeAmount = computed(() => {
  const diff = inputAmount.value - roundedTotal.value;
  return diff > 0 ? diff : 0;
});

const canValidate = computed(() => inputAmount.value >= roundedTotal.value);

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

function handleKey(value: string) {
  if (value === "C") { cashInput.value = ""; return; }
  if (value === "BS") { cashInput.value = cashInput.value.slice(0, -1); return; }
  if (value === ",") {
    // Only one comma allowed, and not as first char
    if (cashInput.value.includes(",") || !cashInput.value) return;
    cashInput.value += ",";
    return;
  }
  // Max 2 digits after comma
  const commaIdx = cashInput.value.indexOf(",");
  if (commaIdx >= 0 && cashInput.value.length - commaIdx > 2) return;
  // Max 6 chars total (e.g. "200,10")
  if (cashInput.value.length < 6) { cashInput.value += value; }
}

function setQuickAmount(n: number) {
  cashInput.value = n.toString();
}

async function payCash() {
  if (!canValidate.value) return;
  cashReceived.value = inputAmount.value;
  step.value = "processing";
  await doCheckout("cash");
}

function payExact() {
  cashReceived.value = roundedTotal.value;
  step.value = "processing";
  doCheckout("cash");
}

function confirmCard() {
  step.value = "processing";
  doCheckout("card");
}

async function doCheckout(method: CheckoutPaymentMethod) {
  if (!order.currentOrder) return;
  error.value = null;
  try {
    const req = delivery.request.medium === "paper" ? undefined : delivery.request;
    result.value = await ipc.checkout(order.currentOrder.id, method, req);
    if (result.value.success) {
      // In cents: no float leftovers such as 9,999999.
      changeDue.value = method === "cash"
        ? Math.max(0, Math.round(cashReceived.value * 100) - Math.round(roundedTotal.value * 100)) / 100
        : 0;
      // Clear the order from the store immediately so it can't be
      // re-submitted. The "done" screen reads from result.value.
      order.clearOrder();
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

    <!-- Step 1: Cash or Card -->
    <div v-if="step === 'method'" class="checkout-center">
      <div class="total-block">
        <span class="total-label">{{ t('total') }}</span>
        <span class="total-value">{{ formatPrice(roundedTotal) }}</span>
        <span v-if="roundedTotal !== totalEur" class="total-exact">
          ({{ formatPrice(totalEur) }} {{ t('before_rounding') }})
        </span>
      </div>
      <p v-if="error" class="error-msg shake">{{ error }}</p>
      <TicketMediumPicker />
      <div class="pay-methods">
        <button class="pay-btn cash-btn" @click="step = 'cash'; cashInput = ''">{{ t('cash') }}</button>
        <button class="pay-btn card-btn" @click="step = 'card'">{{ t('card') }}</button>
      </div>
      <button class="back-link" @click="router.push({ name: 'order' })">{{ t('back') }}</button>
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

    <!-- Step 2b: Card - wait for terminal, then confirm or cancel -->
    <div v-if="step === 'card'" class="checkout-center">
      <div class="total-block">
        <span class="total-label">{{ t('card') }}</span>
        <span class="total-value">{{ formatPrice(totalEur) }}</span>
      </div>
      <p class="card-hint">{{ t('card_wait') }}</p>
      <button class="card-confirm" @click="confirmCard">{{ t('validate') }}</button>
      <button class="back-link" @click="step = 'method'">{{ t('back') }}</button>
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
          FDM: {{ result.ticket.fdm_ref.fdm_id }}
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
.card-btn { background: var(--bg-surface); color: var(--text-primary); }

.back-link { font-size: var(--font-size-sm); color: var(--text-secondary); padding: var(--space-sm); }

.card-hint { font-size: var(--font-size-md); color: var(--text-secondary); text-align: center; }

/* §4.1: primary action full-width × 120px, same as pay-btn */
.card-confirm {
  width: 100%; height: 120px;
  font-size: var(--font-size-xl); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-lg);
  background: var(--accent-success); color: var(--text-on-accent);
  transition: transform var(--tap-duration) ease;
  letter-spacing: 0.05em;
}
.card-confirm:active { transform: scale(var(--tap-scale)); filter: brightness(0.85); }

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
