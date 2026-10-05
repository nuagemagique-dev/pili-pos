<script setup lang="ts">
import { describeError } from "../utils/errorMessages";
import { ref, computed, watch } from "vue";
import { useRouter } from "vue-router";
import { useOrderStore, warnIfSlipFailed } from "../stores/order";
import { useDeliveryStore } from "../stores/delivery";
import { usePosStore } from "../stores/pos";
import type { CartLine, CheckoutResult, PaymentPart } from "../types";
import * as ipc from "../api/ipc";
import { useI18n } from "../i18n";
import { formatPrice, round5 } from "../utils/format";
import { useBilingualName } from "../utils/names";
import AppHeader from "../components/AppHeader.vue";
import TicketMediumPicker from "../components/TicketMediumPicker.vue";
import DeliveryOutcomePanel from "../components/DeliveryOutcomePanel.vue";

const router = useRouter();
const order = useOrderStore();
const delivery = useDeliveryStore();
const pos = usePosStore();
const bilingual = useBilingualName();
const { t } = useI18n();

// A sale can be paid in several parts: card and meal vouchers for an
// exact amount each, cash for the rest (the only part rounded to
// 5 cents, and at most one cash part: the backend settles cash as
// "what is left"). Nothing is signed until the whole sale is paid; the
// backend settles the parts again and refuses a payment that does not
// add up.
//
// The equal split (UX guide §7.4) only helps with the amounts: staff
// pick the number of parts, each part is pre-computed, and every part
// taken is one more entry of the same PaymentPart[] list. A cash part
// taken before the last one is sent last (as "the rest"): with all the
// other parts exact, the backend's cash rest is that part's amount, so
// its 5-cent rounding is the one shown here.
type Method = "cash" | "card" | "meal_voucher";
const METHODS: Method[] = ["cash", "card", "meal_voucher"];

interface TakenPart {
  method: Method;
  cents: number;
  /** Change handed back for a cash part, in cents. */
  changeCents: number;
}

const step = ref<"pay" | "processing" | "done">("pay");
const result = ref<CheckoutResult | null>(null);
const error = ref<string | null>(null);
// Change handed back, fixed when the sale is recorded: the done screen
// cannot recompute it, the cart (and so the total) is cleared by then.
const changeDueCents = ref(0);

// §7.1: cash selected by default (PAYER → Exact → Valider).
const method = ref<Method>("cash");
const splitCount = ref(1);
const parts = ref<TakenPart[]>([]);
// The amount being typed ("12,50"): received for cash, charged for
// card / meal vouchers. `fresh`: set by a shortcut or a prefill, the
// next key replaces it instead of appending to it.
const entry = ref("");
const fresh = ref(false);

// The picker needs a fresh mail probe every time it's about to be
// shown - cheap, the backend caches the probe for 60s.
watch(step, (s) => { if (s === "pay") delivery.refreshMailStatus(); }, { immediate: true });

const totalCents = computed(() => order.totalCents);
const paidCents = computed(() => parts.value.reduce((sum, p) => sum + p.cents, 0));
const restCents = computed(() => Math.max(0, totalCents.value - paidCents.value));
const hasCashPart = computed(() => parts.value.some((p) => p.method === "cash"));

/** `total` in `n` parts, the remainder cents on the first parts:
 * 6030 / 4 -> 1508 1508 1507 1507. */
function equalShares(total: number, n: number): number[] {
  const base = Math.floor(total / n);
  const extra = total - base * n;
  return Array.from({ length: n }, (_, i) => base + (i < extra ? 1 : 0));
}

const shares = computed(() => equalShares(totalCents.value, splitCount.value));
const partIndex = computed(() => parts.value.length);
const isLastPart = computed(() => partIndex.value >= splitCount.value - 1);
/** What the current part pays: its share, the rest for the last one. */
const dueCents = computed(() =>
  isLastPart.value ? restCents.value : Math.min(shares.value[partIndex.value], restCents.value),
);
const roundedDueCents = computed(() => round5(dueCents.value));

/** "Reste à payer": rounded when cash is about to pay all of it. */
const restShownCents = computed(() =>
  method.value === "cash" && isLastPart.value ? round5(restCents.value) : restCents.value,
);

/** A split in more parts than cents would give a part of 0. */
function splitAllowed(n: number): boolean {
  return parts.value.length === 0 && (n === 1 || n <= totalCents.value);
}

/** "12,50" -> 1250 cents; 0 when empty or not a number. */
function toCents(input: string): number {
  if (!input) return 0;
  const parsed = parseFloat(input.replace(",", "."));
  return isNaN(parsed) ? 0 : Math.round(parsed * 100);
}

function toInput(cents: number): string {
  return (cents / 100).toFixed(2).replace(".", ",");
}

const entryCents = computed(() => toCents(entry.value));
const entryShown = computed(() => {
  if (!entry.value) return "0,00";
  return fresh.value ? formatPrice(entryCents.value) : entry.value;
});

// Cash: what was received against the rounded part.
const canPayCash = computed(() => entryCents.value >= roundedDueCents.value);
const changeCents = computed(() => Math.max(0, entryCents.value - roundedDueCents.value));

// Card / meal vouchers: an amount up to the rest.
const exactTooHigh = computed(() => method.value !== "cash" && entryCents.value > restCents.value);
const canConfirmExact = computed(() => entryCents.value > 0 && !exactTooHigh.value);

const canValidate = computed(() => (method.value === "cash" ? canPayCash.value : canConfirmExact.value));

/** Likely amounts handed over (§7.3): the next 5 / 10 € above the
 * amount due, then bank notes above it. */
const quickAmounts = computed(() => {
  const due = roundedDueCents.value / 100;
  const candidates = [Math.ceil(due / 5) * 5, Math.ceil(due / 10) * 10, 20, 50, 100, 200, 500];
  return [...new Set(candidates.filter((n) => n > due))].sort((a, b) => a - b).slice(0, 3);
});

function methodLabel(m: Method): string {
  return t(`method_${m}`);
}

/** Every part: taken, the current one, and the ones to come. */
const partChips = computed(() => {
  const count = Math.max(splitCount.value, parts.value.length + (restCents.value > 0 ? 1 : 0));
  const chips: { state: "paid" | "current" | "next"; label: string }[] = [];
  let left = restCents.value;
  for (let i = 0; i < count; i++) {
    const taken = parts.value[i];
    if (taken) {
      chips.push({ state: "paid", label: `${methodLabel(taken.method)} · ${formatPrice(taken.cents)} € ✓` });
      continue;
    }
    const amount = i >= splitCount.value - 1 ? left : Math.min(shares.value[i], left);
    left -= amount;
    chips.push(
      i === partIndex.value
        ? { state: "current", label: `${methodLabel(method.value)} · ${formatPrice(amount)} €` }
        : { state: "next", label: `${formatPrice(amount)} €` },
    );
  }
  return chips;
});

const itemCount = computed(() => order.lines.reduce((sum, l) => sum + l.quantity, 0));
const orderTitle = computed(() => {
  const o = order.currentOrder;
  const where = o?.order_type === "dine_in" && o.table_number
    ? `${t("table")} ${o.table_number}`
    : t("takeaway_badge");
  return `${t("order")} · ${where}`;
});

/** Notes / modifiers of a line, when it has some (a JSON list of
 * strings, or a plain string). */
function lineNotes(line: CartLine): string | null {
  const raw = line.modifiers_json;
  if (!raw) return null;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed.map(String).join(" · ") || null;
    if (typeof parsed === "string") return parsed || null;
  } catch { /* not JSON: shown as is */ }
  return raw;
}

// ── Entry ──

/** A new part or method: cash starts empty (tap Exact or an amount),
 * card / meal vouchers start at the amount due, which can be lowered. */
function prefill() {
  if (method.value === "cash") {
    entry.value = "";
    fresh.value = false;
  } else {
    entry.value = dueCents.value > 0 ? toInput(dueCents.value) : "";
    fresh.value = true;
  }
}
watch([method, partIndex, splitCount, totalCents], prefill, { immediate: true });

/** Numpad: digits, one comma, 2 decimals, 6 chars. */
function handleKey(value: string) {
  if (fresh.value) {
    // A shortcut or a prefilled amount is replaced, not appended to.
    entry.value = "";
    fresh.value = false;
    if (value === "BS") return;
  }
  if (value === "BS") { entry.value = entry.value.slice(0, -1); return; }
  if (value === ",") {
    // Only one comma allowed, and not as first char
    if (entry.value.includes(",") || !entry.value) return;
    entry.value += ",";
    return;
  }
  // Max 2 digits after comma
  const commaIdx = entry.value.indexOf(",");
  if (commaIdx >= 0 && entry.value.length - commaIdx > 2) return;
  // Max 6 chars total (e.g. "200,10")
  if (entry.value.length < 6) entry.value += value;
}

function setExact() {
  entry.value = toInput(method.value === "cash" ? roundedDueCents.value : dueCents.value);
  fresh.value = true;
}

const exactActive = computed(() =>
  fresh.value && entryCents.value === (method.value === "cash" ? roundedDueCents.value : dueCents.value),
);

function setQuickAmount(n: number) {
  entry.value = n.toString();
  fresh.value = true;
}

function quickActive(n: number): boolean {
  return fresh.value && entryCents.value === n * 100;
}

function selectMethod(m: Method) {
  if (m === "cash" && hasCashPart.value) return;
  method.value = m;
}

function selectSplit(n: number) {
  if (splitAllowed(n)) splitCount.value = n;
}

function clearParts() {
  parts.value = [];
  error.value = null;
  method.value = "cash";
}

// ── Payment ──

function payload(all: TakenPart[]): PaymentPart[] {
  const list: PaymentPart[] = all
    .filter((p) => p.method !== "cash")
    .map((p) => ({ method: p.method, amount_cents: p.cents }));
  if (all.some((p) => p.method === "cash")) list.push({ method: "cash" });
  return list;
}

/** Take the current part: the sale is signed once nothing is left to
 * pay, otherwise on to the next part. */
function validate() {
  if (!canValidate.value) return;
  const isCash = method.value === "cash";
  const part: TakenPart = isCash
    ? { method: "cash", cents: dueCents.value, changeCents: changeCents.value }
    : { method: method.value, cents: entryCents.value, changeCents: 0 };
  if (part.cents >= restCents.value) {
    doCheckout([...parts.value, part]);
    return;
  }
  parts.value.push(part);
  // One cash part at most; otherwise back to cash, the default.
  method.value = hasCashPart.value ? (isCash ? "card" : method.value) : "cash";
}

async function doCheckout(all: TakenPart[]) {
  if (!order.currentOrder) return;
  error.value = null;
  step.value = "processing";
  try {
    const req = delivery.request.medium === "paper" ? undefined : delivery.request;
    result.value = await ipc.checkout(order.currentOrder.id, payload(all), req);
    if (result.value.success) {
      warnIfSlipFailed(result.value.kitchen_slip);
      changeDueCents.value = all.reduce((sum, p) => sum + p.changeCents, 0);
      // Clear the order from the store immediately so it can't be
      // re-submitted. The "done" screen reads from result.value.
      order.clearOrder();
      parts.value = [];
      // Next customer starts from Papier.
      delivery.reset();
      step.value = "done";
    } else {
      error.value = describeError(result.value.error);
      step.value = "pay";
    }
  } catch (e) {
    error.value = describeError(e);
    step.value = "pay";
  }
}

function finish() {
  result.value = null;
  router.push({ name: "order" });
}
</script>

<template>
  <div class="checkout">
    <AppHeader />

    <div v-if="step === 'pay'" class="pay-row">
      <section class="pay-main">
        <!-- §7.2: the three methods side by side -->
        <div class="methods">
          <button
            v-for="m in METHODS"
            :key="m"
            class="method-btn"
            :class="[m === 'cash' ? 'cash-btn' : m === 'card' ? 'card-btn' : 'meal-btn', { active: method === m }]"
            :disabled="m === 'cash' && hasCashPart"
            @click="selectMethod(m)"
          >{{ methodLabel(m) }}</button>
        </div>

        <!-- §7.4: equal split -->
        <div class="split-row">
          <span class="row-label">{{ t('split_into') }}</span>
          <div class="segmented split-options">
            <button
              v-for="n in 5"
              :key="n"
              class="seg-btn split-btn"
              :class="{ active: splitCount === n }"
              :disabled="splitCount !== n && !splitAllowed(n)"
              @click="selectSplit(n)"
            >{{ n }}</button>
          </div>
          <span class="split-hint">{{ t('equal_parts') }}</span>
        </div>

        <div v-if="partChips.length > 1" class="part-chips">
          <div v-for="(chip, i) in partChips" :key="i" class="part-chip" :class="chip.state">
            <span class="chip-title">{{ t('part_n', { n: i + 1 }) }}</span>
            <span class="chip-label">{{ chip.label }}</span>
          </div>
        </div>

        <div class="entry-grid">
          <div class="entry-left">
            <div class="section-label">{{ method === 'cash' ? t('amount_received') : t('amount') }}</div>
            <button class="shortcut exact" :class="{ active: exactActive }" @click="setExact">
              <span>{{ t('exact') }}</span>
              <span class="exact-amounts">
                <span class="exact-main">{{ formatPrice(method === 'cash' ? roundedDueCents : dueCents) }} €</span>
                <span v-if="method === 'cash' && roundedDueCents !== dueCents" class="exact-sub">
                  {{ t('exact_rounded', { exact: formatPrice(dueCents) }) }}
                </span>
              </span>
            </button>

            <template v-if="method === 'cash'">
              <div class="quick-amounts">
                <button
                  v-for="n in quickAmounts"
                  :key="n"
                  class="shortcut quick"
                  :class="{ active: quickActive(n) }"
                  @click="setQuickAmount(n)"
                >{{ n }} €</button>
              </div>
              <div class="change-box" :class="{ pending: !canPayCash }">
                <span class="change-label">{{ t('change') }}</span>
                <span class="change-value">{{ canPayCash ? `${formatPrice(changeCents)} €` : '-' }}</span>
              </div>
            </template>
            <template v-else>
              <p v-if="exactTooHigh" class="exact-too-high">{{ t('amount_over_rest') }}</p>
              <p class="card-hint">{{ t('card_wait') }}</p>
              <p v-if="hasCashPart" class="card-hint">{{ t('cash_part_taken') }}</p>
            </template>
          </div>

          <div class="entry-right">
            <div class="entry-display">
              <span class="entry-label">{{ method === 'cash' ? t('received') : t('amount') }}</span>
              <span class="entry-value" :class="{ 'exact-value': method !== 'cash' }">{{ entryShown }}</span>
            </div>
            <div class="numpad">
              <button v-for="d in ['1','2','3','4','5','6','7','8','9']" :key="d" class="nk" @click="handleKey(d)">{{ d }}</button>
              <button class="nk nk-comma" @click="handleKey(',')">,</button>
              <button class="nk" @click="handleKey('0')">0</button>
              <button class="nk nk-back" @click="handleKey('BS')">&#9003;</button>
            </div>
          </div>
        </div>

        <!-- Note 05: ticket medium, only when email is configured -->
        <div v-if="delivery.mailStatus?.configured && !parts.length" class="ticket-row">
          <span class="row-label">{{ t('ticket_medium') }}</span>
          <TicketMediumPicker />
        </div>
      </section>

      <aside class="summary">
        <div class="summary-head">
          <h2>{{ orderTitle }}</h2>
          <span class="item-count">{{ t('items_count', { n: itemCount }) }}</span>
        </div>

        <div class="summary-lines scroll-touch">
          <div v-for="line in order.lines" :key="line.id" class="sum-line">
            <span class="line-qty">{{ line.quantity }}×</span>
            <div class="line-name">
              <span class="name-primary" :class="{ zh: pos.language === 'zh' && !bilingual(line).isCode, code: bilingual(line).isCode }">{{ bilingual(line).primary }}</span>
              <span v-if="bilingual(line).secondary" class="name-secondary">{{ bilingual(line).secondary }}</span>
              <span v-if="lineNotes(line)" class="line-notes">{{ lineNotes(line) }}</span>
            </div>
            <span class="line-total">{{ formatPrice(line.unit_price_cents * line.quantity) }}</span>
          </div>
        </div>

        <div class="summary-foot">
          <div class="foot-row total-row">
            <span>{{ t('total') }}</span><span>{{ formatPrice(totalCents) }} €</span>
          </div>
          <div v-for="(p, i) in parts" :key="i" class="foot-row paid-part">
            <span>{{ t('part_n', { n: i + 1 }) }} · {{ methodLabel(p.method) }}</span>
            <span>− {{ formatPrice(p.cents) }} €</span>
          </div>
          <div class="rest-row">
            <span class="total-label">{{ t('rest_to_pay') }}</span>
            <span class="rest-amount"><span class="total-value">{{ formatPrice(restShownCents) }}</span> €</span>
          </div>

          <p v-if="error" class="error-msg shake">{{ error }}</p>

          <div class="foot-actions">
            <button v-if="parts.length" class="back-btn parts-clear" @click="clearParts">{{ t('clear_payments') }}</button>
            <button v-else class="back-btn" @click="router.push({ name: 'order' })">{{ t('back') }}</button>
            <button
              class="validate-btn"
              :class="method === 'cash' ? 'cash-confirm' : 'card-confirm'"
              :disabled="!canValidate"
              @click="validate"
            >
              <span class="validate-label">{{ splitCount > 1 ? t('validate_part', { n: partIndex + 1 }) : t('validate') }}</span>
              <span v-if="method === 'cash' && entry" class="validate-amount">
                {{ t('received_amount', { amount: formatPrice(entryCents) }) }}
              </span>
              <span v-else-if="method !== 'cash' && entryCents > 0" class="validate-amount">
                {{ formatPrice(entryCents) }} €
              </span>
            </button>
          </div>
        </div>
      </aside>
    </div>

    <!-- Processing -->
    <div v-else-if="step === 'processing'" class="status-screen">
      <div class="checkout-center">
        <div class="spinner"></div>
        <p class="processing-text">{{ t('processing_fdm') }}</p>
      </div>
    </div>

    <!-- Done -->
    <div v-else class="status-screen">
      <div class="checkout-center">
        <div class="success-icon">OK</div>
        <div v-if="result?.ticket" class="receipt-mini">
          <div class="receipt-label">{{ result.ticket.ticket_type_label }}</div>
          <div class="receipt-total">{{ formatPrice(Math.round(result.ticket.total * 100)) }} €</div>
          <div v-if="changeDueCents > 0" class="receipt-change">
            <span>{{ t('change') }}</span>
            <span class="receipt-change-value">{{ formatPrice(changeDueCents) }} €</span>
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
          :fiscal-event-id="result?.fiscal_event_id ?? null"
        />
        <div class="done-actions">
          <button class="act-btn primary" @click="finish">{{ t('finish') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.checkout {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  color: var(--text-primary);
}

button { transition: transform var(--tap-duration) ease; }
button:active:not(:disabled) { transform: scale(var(--tap-scale)); }

.pay-row { flex: 1; min-height: 0; display: flex; overflow: hidden; }

/* ── Left: methods, split, amount entry ── */
.pay-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 24px 32px;
  overflow-y: auto;
}

.methods { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
.method-btn {
  height: 88px;
  border-radius: 14px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 22px;
  font-weight: 800;
  letter-spacing: 0.02em;
  text-transform: uppercase;
}
.method-btn.active { background: var(--selected-bg); color: var(--selected-fg); }
.method-btn:disabled { opacity: 0.35; }

.row-label { font-size: 18px; font-weight: 700; color: var(--text-secondary); min-width: 170px; }
.split-row { display: flex; align-items: center; gap: 16px; }
.split-hint { font-size: 17px; color: var(--text-secondary); }

.segmented {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  gap: 4px;
  padding: 4px;
  background: var(--bg-surface);
  border-radius: 12px;
}
.split-options { width: 420px; }
.seg-btn {
  height: 48px;
  border-radius: 8px;
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
  font-family: var(--font-mono);
}
.seg-btn.active { background: var(--selected-bg); color: var(--selected-fg); }
.seg-btn:disabled { color: var(--text-secondary); opacity: 0.5; }

.part-chips { display: flex; gap: 8px; }
.part-chip {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 14px;
  border-radius: 10px;
}
.part-chip.paid { background: var(--bg-surface); }
.part-chip.current { box-shadow: inset 0 0 0 2px var(--selected-bg); }
.part-chip.next { background: var(--bg-surface); color: var(--text-secondary); }
.chip-title { font-family: var(--font-mono); font-size: 13px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; }
.chip-label { font-size: 16px; font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

.entry-grid {
  flex: 1;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 32px;
  min-height: 0;
}
.entry-left, .entry-right { display: flex; flex-direction: column; gap: 12px; }

.section-label {
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: 700;
  color: var(--text-secondary);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.shortcut {
  height: 88px;
  border-radius: 10px;
  background: var(--bg-surface);
  color: var(--text-primary);
}
.shortcut.active { background: var(--selected-bg); color: var(--selected-fg); }
.shortcut.exact {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  font-size: 24px;
  font-weight: 700;
}
.exact-amounts { display: flex; flex-direction: column; align-items: flex-end; line-height: 1.15; font-family: var(--font-mono); }
.exact-main { font-size: 28px; }
.exact-sub { font-size: 14px; color: var(--text-secondary); }
.shortcut.exact.active .exact-sub { color: var(--selected-fg); }

.quick-amounts { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
.shortcut.quick { font-family: var(--font-mono); font-size: 28px; font-weight: 700; }

/* §5.3: change due, 72px */
.change-box {
  margin-top: auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 28px;
  border-radius: 16px;
  border: 2px solid var(--accent-success);
  color: var(--accent-success);
}
.change-box.pending { border-color: var(--border); color: var(--text-secondary); }
.change-label { font-size: 28px; font-weight: 800; }
.change-value { font-family: var(--font-mono); font-size: 72px; font-weight: 700; line-height: 1; }

.exact-too-high { color: var(--accent-danger); font-size: 17px; font-weight: 700; }
.card-hint { font-size: 17px; color: var(--text-secondary); }

.entry-display {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 72px;
  padding: 0 20px;
  border-radius: 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
}
.entry-label { font-size: 16px; color: var(--text-secondary); }
.entry-value { font-family: var(--font-mono); font-size: 40px; font-weight: 700; }

.numpad { display: grid; grid-template-columns: repeat(3, 120px); gap: 8px; }
.nk {
  height: 80px;
  border-radius: 10px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 30px;
  font-weight: 700;
}
.nk:active { background: var(--bg-pressed); }
.nk-comma { color: var(--text-secondary); }
.nk-back { color: var(--accent-danger); }

.ticket-row {
  display: flex;
  align-items: center;
  gap: 16px;
  padding-top: 14px;
  border-top: 1px solid var(--border);
}
/* TicketMediumPicker is shared with the reports screen: restyled here
 * as the design's segmented control without touching the component. */
.ticket-row :deep(.medium-picker) { flex: 1; min-width: 0; }
.ticket-row :deep(.medium-options) {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  gap: 4px;
  padding: 4px;
  background: var(--bg-surface);
  border-radius: 12px;
}
.ticket-row :deep(.medium-btn) {
  height: 48px;
  border-radius: 8px;
  font-size: 18px;
  font-weight: 700;
  background: transparent;
  color: var(--text-primary);
}
.ticket-row :deep(.medium-btn.active) { background: var(--selected-bg); color: var(--selected-fg); }
.ticket-row :deep(.medium-note) { text-align: left; font-size: 13px; }

/* ── Right: the order, read-only ── */
.summary {
  width: 670px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border);
  min-height: 0;
}
.summary-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px 10px;
  border-bottom: 1px solid var(--border);
}
.summary-head h2 { font-size: 28px; font-weight: 800; line-height: 1.2; }
.item-count { font-family: var(--font-mono); font-size: 15px; color: var(--text-secondary); white-space: nowrap; }

.summary-lines { flex: 1; min-height: 0; overflow-y: auto; padding: 0 16px; }
.sum-line {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 16px;
  min-height: 56px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
.line-qty { font-family: var(--font-mono); font-size: 17px; font-weight: 700; color: var(--text-secondary); min-width: 36px; }
.line-name { min-width: 0; display: flex; flex-direction: column; }
.name-primary { font-size: 17px; font-weight: 700; line-height: 1.3; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.name-primary.zh { font-family: var(--font-zh); }
.name-primary.code { font-family: var(--font-mono); }
.name-secondary { font-size: 13px; color: var(--text-secondary); line-height: 1.3; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.line-notes { font-size: 13px; color: var(--accent-warning); }
.line-total { font-family: var(--font-mono); font-size: 17px; font-weight: 700; }

.summary-foot {
  border-top: 1px solid var(--border);
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.foot-row {
  display: flex;
  justify-content: space-between;
  font-family: var(--font-mono);
  font-size: 17px;
  color: var(--text-secondary);
}
.rest-row { display: flex; justify-content: space-between; align-items: baseline; }
.total-label { font-size: 20px; font-weight: 700; }
.rest-amount { font-family: var(--font-mono); font-size: 44px; font-weight: 700; line-height: 1.1; }

.error-msg {
  color: var(--accent-danger);
  font-size: 16px;
  font-weight: 700;
  padding: 10px 14px;
  background: color-mix(in srgb, var(--accent-danger) 12%, transparent);
  border: 1px solid var(--accent-danger);
  border-radius: 10px;
}

.foot-actions { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 8px; margin-top: 4px; }
.back-btn {
  height: 80px;
  padding: 0 24px;
  border-radius: 10px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 17px;
  font-weight: 700;
}
.validate-btn {
  height: 80px;
  padding: 0 24px;
  border-radius: 10px;
  background: var(--accent-primary);
  color: var(--text-on-accent);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.validate-btn:disabled { opacity: 0.35; }
.validate-label { font-size: 22px; font-weight: 800; text-transform: uppercase; }
.validate-amount { font-family: var(--font-mono); font-size: 18px; white-space: nowrap; }

/* ── Processing / done ── */
.status-screen { flex: 1; min-height: 0; display: flex; align-items: center; justify-content: center; padding: 24px; }
.checkout-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
  max-width: 600px;
  width: 100%;
}
.spinner {
  width: 56px;
  height: 56px;
  border: 4px solid var(--bg-surface);
  border-top-color: var(--accent-primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
.processing-text { color: var(--text-secondary); font-size: 18px; }

.success-icon {
  width: 96px;
  height: 96px;
  border-radius: 50%;
  background: var(--accent-success);
  color: var(--text-on-accent);
  font-size: 28px;
  font-weight: 800;
  display: flex;
  align-items: center;
  justify-content: center;
}
.receipt-mini { display: flex; flex-direction: column; align-items: center; gap: 8px; width: 100%; }
.receipt-label { font-family: var(--font-mono); font-size: 14px; font-weight: 700; letter-spacing: 0.08em; color: var(--text-secondary); }
.receipt-total { font-family: var(--font-mono); font-size: 44px; font-weight: 700; }
.receipt-change {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 28px;
  border-radius: 16px;
  border: 2px solid var(--accent-success);
  color: var(--accent-success);
  font-size: 28px;
  font-weight: 800;
}
.receipt-change-value { font-family: var(--font-mono); font-size: 72px; font-weight: 700; line-height: 1; }
.receipt-fdm { font-family: var(--font-mono); font-size: 13px; color: var(--text-secondary); }
.fdm-warn {
  padding: 10px 16px;
  border: 1px solid var(--accent-warning);
  color: var(--accent-warning);
  border-radius: 10px;
  font-size: 16px;
  font-weight: 700;
}
.done-actions { display: flex; width: 100%; }
.act-btn {
  flex: 1;
  height: 80px;
  font-size: 22px;
  font-weight: 800;
  border-radius: 10px;
}
.act-btn.primary { background: var(--accent-primary); color: var(--text-on-accent); }
</style>
