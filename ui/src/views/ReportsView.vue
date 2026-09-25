<script setup lang="ts">
import { describeError } from "../utils/fdmMessages";
import { formatDate as formatDay, formatTime, formatTimestamp } from "../utils/datetime";
import { onMounted, onUnmounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "../stores/auth";
import { useToastStore } from "../stores/toast";
import { useDeliveryStore } from "../stores/delivery";
import { formatPrice } from "../utils/format";
import type { CheckoutResult, OrderLine } from "../types";
import type { CopyResult, RefundableEvent, RefundItem } from "../api/ipc";
import * as ipc from "../api/ipc";
import { useI18n } from "../i18n";
import TicketMediumPicker from "../components/TicketMediumPicker.vue";
import DeliveryOutcomePanel from "../components/DeliveryOutcomePanel.vue";

const router = useRouter();
const auth = useAuthStore();
const toast = useToastStore();
const delivery = useDeliveryStore();
const { t } = useI18n();

const loading = ref(false);
const zHistory = ref<ipc.ZReportSummary[]>([]);

async function loadHistory() {
  try {
    zHistory.value = await ipc.listZReports();
  } catch {
    /* silently degrade - history is non-critical */
  }
}

onMounted(() => {
  if (!auth.currentUser) {
    router.push({ name: "login" });
    return;
  }
  loadHistory();
});

async function generateX() {
  if (!auth.currentUser || loading.value) return;
  loading.value = true;
  try {
    const result = await ipc.generateXReport(auth.currentUser.insz);
    if (result.success) {
      toast.show(`${t('report_generated')} (X)`, "success", 4000);
    } else {
      toast.show(describeError(result.error ?? "Report failed"), "error", 5000);
    }
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
  } finally {
    loading.value = false;
  }
}

async function generateUserX() {
  if (!auth.currentUser || loading.value) return;
  loading.value = true;
  try {
    const result = await ipc.generateUserXReport(auth.currentUser.insz);
    if (result.success) {
      toast.show(`${t('report_generated')} (X util.)`, "success", 4000);
    } else {
      toast.show(describeError(result.error ?? "Report failed"), "error", 5000);
    }
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
  } finally {
    loading.value = false;
  }
}

// A Z closes the day for good: the first tap only arms the button,
// the second (within 5 s) generates it.
const zArmed = ref(false);
let zDisarm: ReturnType<typeof setTimeout> | undefined;
onUnmounted(() => clearTimeout(zDisarm));

async function generateZ() {
  if (!auth.currentUser || loading.value) return;
  if (!zArmed.value) {
    zArmed.value = true;
    zDisarm = setTimeout(() => { zArmed.value = false; }, 5000);
    return;
  }
  clearTimeout(zDisarm);
  zArmed.value = false;
  loading.value = true;
  try {
    const result = await ipc.generateZReport(auth.currentUser.insz);
    if (result.success) {
      const label = result.reportNo
        ? `${t('report_generated')} (Z #${result.reportNo})`
        : `${t('report_generated')} (Z)`;
      toast.show(label, "success", 4000);
      await loadHistory();
    } else {
      toast.show(describeError(result.error ?? "Report failed"), "error", 5000);
    }
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
  } finally {
    loading.value = false;
  }
}

// ── Copy flow (event C) - picker shown before signing ──────────────
const copyOpen = ref(false);
const copyStep = ref<"pick" | "picker" | "processing" | "result">("picker");
const copyTarget = ref<{ fiscalEventId: number | null } | null>(null);
const copyResult = ref<CopyResult | null>(null);
const copyTickets = ref<ipc.CopyableTicket[]>([]);

/** Copy of a Z report, from the history. */
function openCopy(report: { fiscalEventId: number | null }) {
  if (!report.fiscalEventId) return;
  copyTarget.value = report;
  copyResult.value = null;
  delivery.reset();
  delivery.refreshMailStatus();
  copyStep.value = "picker";
  copyOpen.value = true;
}

/** Copy of a ticket a customer received: pick it first. */
async function openTicketCopy() {
  copyResult.value = null;
  copyTarget.value = null;
  try {
    copyTickets.value = await ipc.listCopyableTickets();
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
    return;
  }
  copyStep.value = "pick";
  copyOpen.value = true;
}

function selectCopyTicket(ticket: ipc.CopyableTicket) {
  openCopy({ fiscalEventId: ticket.fiscalEventId });
}

function copyKind(ticket: ipc.CopyableTicket): string {
  if (ticket.eventLabel === "T") return t("copy_kind_training");
  if (ticket.eventLabel === "P") return t("copy_kind_pre_bill");
  return ticket.totalCents < 0 ? t("copy_kind_refund") : t("copy_kind_sale");
}

function closeCopy() {
  copyOpen.value = false;
  copyResult.value = null;
  copyTickets.value = [];
  delivery.reset();
}

async function confirmCopy() {
  const insz = auth.currentUser?.insz;
  const target = copyTarget.value;
  if (!insz || !target?.fiscalEventId) return;
  copyStep.value = "processing";
  try {
    const req = delivery.request.medium === "paper" ? undefined : delivery.request;
    const result = await ipc.copyTicket(target.fiscalEventId, insz, req);
    copyResult.value = result;
    if (result.success) {
      toast.show(t('report_reprint_ok'), "success");
    } else {
      toast.show(describeError(result.error ?? "Copy failed"), "error", 5000);
    }
    copyStep.value = "result";
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
    copyOpen.value = false;
  }
}

function formatDate(iso: string): string {
  return iso ? formatTimestamp(iso, formatDay) : "-";
}

// ── Cash management (Event F) - preset buttons, no keyboard ──────
const cashAmountCents = ref(0);
const cashLoading = ref(false);
const cashPresets = [1000, 2000, 5000, 10000, 20000]; // 10€, 20€, 50€, 100€, 200€

const cashReasons: { key: string; label: Record<string, string> }[] = [
  { key: "fond_de_caisse", label: { fr: "Fond de caisse", en: "Cash float", zh: "备用金" } },
  { key: "retrait", label: { fr: "Retrait", en: "Withdrawal", zh: "提现" } },
  { key: "pourboire", label: { fr: "Pourboire", en: "Tip", zh: "小费" } },
];

async function handleCashInOut(direction: "in" | "out", reason: string) {
  const insz = auth.currentUser?.insz;
  if (!insz || cashAmountCents.value <= 0) return;
  cashLoading.value = true;
  try {
    const result = await ipc.moneyInOut(insz, cashAmountCents.value, direction, reason);
    if (result.success) {
      toast.show(t("cash_success"), "success");
      cashAmountCents.value = 0;
    } else {
      toast.show(describeError(result.error ?? t("cash_failed")), "error", 5000);
    }
  } catch (e) {
    toast.show(describeError(e), "error");
  } finally {
    cashLoading.value = false;
  }
}

async function handleOpenDrawer() {
  const insz = auth.currentUser?.insz;
  if (!insz) return;
  try {
    const result = await ipc.openDrawerFiscal(insz);
    if (result.success) {
      toast.show(t("cash_drawer_opened"), "success");
    } else {
      toast.show(describeError(result.error ?? t("cash_failed")), "error", 5000);
    }
  } catch (e) {
    toast.show(describeError(e), "error");
  }
}

// ── Refund flow ──────────────────────────────────────────────────
const refundOpen = ref(false);
const refundStep = ref<"pick" | "items" | "delivery" | "processing" | "result">("pick");
const refundEvents = ref<RefundableEvent[]>([]);
const refundSelected = ref<RefundableEvent | null>(null);
const refundLines = ref<OrderLine[]>([]);
const refundChecked = ref<Map<number, number>>(new Map());
const refundResult = ref<CheckoutResult | null>(null);

async function openRefund() {
  refundOpen.value = true;
  refundStep.value = "pick";
  refundSelected.value = null;
  refundChecked.value = new Map();
  refundResult.value = null;
  try {
    refundEvents.value = await ipc.listRefundableEvents(20);
  } catch {
    refundEvents.value = [];
  }
}

function closeRefund() {
  refundOpen.value = false;
  refundResult.value = null;
  delivery.reset();
}

async function selectRefundTicket(ev: RefundableEvent) {
  refundSelected.value = ev;
  refundChecked.value = new Map();
  try {
    refundLines.value = await ipc.listOrderLines(ev.orderId);
  } catch {
    refundLines.value = [];
  }
  refundStep.value = "items";
}

function toggleRefundLine(line: OrderLine) {
  const map = new Map(refundChecked.value);
  if (map.has(line.id)) {
    map.delete(line.id);
  } else {
    map.set(line.id, line.quantity);
  }
  refundChecked.value = map;
}

function adjustRefundQty(lineId: number, delta: number) {
  const map = new Map(refundChecked.value);
  const current = map.get(lineId) ?? 0;
  const line = refundLines.value.find(l => l.id === lineId);
  if (!line) return;
  const next = Math.max(1, Math.min(line.quantity, current + delta));
  map.set(lineId, next);
  refundChecked.value = map;
}

function refundTotalCents(): number {
  let total = 0;
  for (const [lineId, qty] of refundChecked.value) {
    const line = refundLines.value.find(l => l.id === lineId);
    if (line) total += line.unit_price_cents * qty;
  }
  return total;
}

function goToRefundDelivery() {
  if (refundChecked.value.size === 0) return;
  delivery.reset();
  delivery.refreshMailStatus();
  refundStep.value = "delivery";
}

async function confirmRefund() {
  const insz = auth.currentUser?.insz;
  const ev = refundSelected.value;
  if (!insz || !ev || refundChecked.value.size === 0) return;

  const items: RefundItem[] = [];
  for (const [lineId, qty] of refundChecked.value) {
    const line = refundLines.value.find(l => l.id === lineId);
    if (line) {
      items.push({
        menuItemId: line.menu_item_id,
        quantity: qty,
      });
    }
  }

  refundStep.value = "processing";
  try {
    const req = delivery.request.medium === "paper" ? undefined : delivery.request;
    const result = await ipc.refundCheckout(ev.fiscalEventId, items, insz, req);
    refundResult.value = result;
    if (result.success) {
      toast.show(t("refund_success"), "success");
    } else {
      toast.show(describeError(result.error ?? "Refund failed"), "error");
    }
    refundStep.value = "result";
  } catch (e) {
    toast.show(describeError(e), "error");
    refundStep.value = "items";
  }
}

function formatRefundTime(iso: string): string {
  return formatTimestamp(iso, formatTime);
}
</script>

<template>
  <div class="reports">
    <header class="header">
      <h1>{{ t('reports') }}</h1>
      <button class="nav-btn" @click="router.push({ name: 'order' })">{{ t('back') }}</button>
    </header>
    <div class="content">
      <!-- Reports -->
      <div class="report-row">
        <div><h2>{{ t('report_x') }}</h2><p>{{ t('report_x_desc') }}</p></div>
        <button class="report-btn x-btn" :disabled="loading" @click="generateX">
          <span v-if="loading" class="spinner" aria-hidden="true"></span>
          <span v-else>{{ t('generate') }} X</span>
        </button>
      </div>
      <div class="report-row">
        <div><h2>{{ t('report_user_x') }}</h2><p>{{ t('report_user_x_desc') }}</p></div>
        <button class="report-btn x-btn" :disabled="loading" @click="generateUserX">
          <span v-if="loading" class="spinner" aria-hidden="true"></span>
          <span v-else>{{ t('generate') }} X util.</span>
        </button>
      </div>
      <div class="report-row">
        <div><h2>{{ t('report_z') }}</h2><p>{{ t('report_z_desc') }}</p></div>
        <button class="report-btn z-btn" :class="{ armed: zArmed }" :disabled="loading" @click="generateZ">
          <span v-if="loading" class="spinner" aria-hidden="true"></span>
          <span v-else>{{ zArmed ? t('confirm_z') : `${t('generate')} Z` }}</span>
        </button>
      </div>

      <!-- Z-report history -->
      <div class="history-section">
        <h3>{{ t('report_history_title') }}</h3>
        <p v-if="zHistory.length === 0" class="empty">{{ t('report_no_history') }}</p>
        <ul v-else class="history-list">
          <li v-for="z in zHistory" :key="z.id" class="history-row">
            <span class="history-label">
              Z #{{ z.sequenceNumber }} · {{ formatDate(z.createdAt) }}
            </span>
            <button
              v-if="z.fiscalEventId"
              class="reprint-btn"
              @click="openCopy(z)"
            >
              {{ t('report_reprint') }}
            </button>
          </li>
        </ul>
      </div>

      <!-- Cash management (Event F) - §7.3: preset buttons, no keyboard -->
      <div class="cash-section">
        <h3>{{ t('cash_section') }}</h3>

        <!-- Amount presets -->
        <div class="cash-presets">
          <button
            v-for="cents in cashPresets"
            :key="cents"
            class="cash-preset"
            :class="{ active: cashAmountCents === cents }"
            @click="cashAmountCents = cents"
          >{{ formatPrice(cents) }}</button>
        </div>

        <!-- In / Out with preset reasons -->
        <div class="cash-actions">
          <button
            v-for="reason in cashReasons"
            :key="reason.key"
            class="cash-action-btn cash-in"
            :disabled="cashLoading || cashAmountCents <= 0"
            @click="handleCashInOut('in', reason.key)"
          >{{ reason.label.fr }} +</button>
        </div>
        <div class="cash-actions">
          <button
            v-for="reason in cashReasons"
            :key="reason.key"
            class="cash-action-btn cash-out"
            :disabled="cashLoading || cashAmountCents <= 0"
            @click="handleCashInOut('out', reason.key)"
          >{{ reason.label.fr }} -</button>
        </div>

        <button class="drawer-btn" @click="handleOpenDrawer">
          {{ t('cash_open_drawer') }}
        </button>
        <button class="refund-btn" @click="openRefund">
          {{ t('refund') }}
        </button>
        <button class="copy-btn" @click="openTicketCopy">
          {{ t('copy_ticket') }}
        </button>
      </div>
    </div>

    <!-- Refund modal overlay (§8.2) -->
    <Teleport to="body">
      <div v-if="refundOpen" class="refund-overlay" @click.self="refundOpen = false">
        <div class="refund-modal">
          <template v-if="refundStep === 'pick'">
            <h3>{{ t('refund_select_ticket') }}</h3>
            <div v-if="refundEvents.length === 0" class="refund-empty">{{ t('refund_none') }}</div>
            <div class="refund-ticket-grid">
              <button
                v-for="ev in refundEvents"
                :key="ev.fiscalEventId"
                class="refund-ticket-btn"
                @click="selectRefundTicket(ev)"
              >
                <span class="rt-num">#{{ ev.ticketNumber }}</span>
                <span v-if="ev.tableNumber" class="rt-table">T{{ ev.tableNumber }}</span>
                <span class="rt-total">{{ formatPrice(ev.totalCents) }}</span>
                <span class="rt-time">{{ formatRefundTime(ev.createdAt) }}</span>
              </button>
            </div>
            <button class="refund-cancel" @click="refundOpen = false">{{ t('back') }}</button>
          </template>

          <template v-else-if="refundStep === 'items'">
            <h3>{{ t('refund_select_items') }}</h3>
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
                  <span class="rl-price">{{ formatPrice(line.unit_price_cents * line.quantity) }}</span>
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
            >{{ t('refund_confirm') }} {{ formatPrice(refundTotalCents()) }}</button>
            <button class="refund-cancel" @click="refundStep = 'pick'">{{ t('back') }}</button>
          </template>

          <template v-else-if="refundStep === 'delivery'">
            <h3>{{ t('refund_confirm') }}</h3>
            <TicketMediumPicker />
            <button
              class="refund-confirm-btn"
              @click="confirmRefund"
            >{{ t('refund_confirm') }} {{ formatPrice(refundTotalCents()) }}</button>
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
              :ticket="refundResult?.ticket ?? null"
              :fiscal-event-id="refundResult?.fiscal_event_id ?? null"
            />
            <p v-if="refundResult && !refundResult.success" class="refund-error">{{ describeError(refundResult.error) }}</p>
            <button class="refund-cancel" @click="closeRefund">{{ t('finish') }}</button>
          </template>
        </div>
      </div>
    </Teleport>

    <!-- Copy modal (event C) - picker shown before signing (§8.2) -->
    <Teleport to="body">
      <div v-if="copyOpen" class="refund-overlay" @click.self="closeCopy">
        <div class="refund-modal">
          <template v-if="copyStep === 'pick'">
            <h3>{{ t('copy_select_ticket') }}</h3>
            <div v-if="copyTickets.length === 0" class="refund-empty">{{ t('copy_none') }}</div>
            <div class="refund-ticket-grid">
              <button
                v-for="tk in copyTickets"
                :key="tk.fiscalEventId"
                class="refund-ticket-btn copy-ticket-btn"
                @click="selectCopyTicket(tk)"
              >
                <span class="rt-num">#{{ tk.ticketNumber }}</span>
                <span class="rt-kind">{{ copyKind(tk) }}</span>
                <span v-if="tk.tableNumber" class="rt-table">T{{ tk.tableNumber }}</span>
                <span class="rt-total">{{ formatPrice(tk.totalCents) }}</span>
                <span class="rt-time">{{ formatRefundTime(tk.createdAt) }}</span>
              </button>
            </div>
            <button class="refund-cancel" @click="closeCopy">{{ t('back') }}</button>
          </template>
          <template v-else-if="copyStep === 'picker'">
            <h3>{{ copyTickets.length ? t('copy_ticket') : t('report_reprint') }}</h3>
            <TicketMediumPicker />
            <button class="refund-confirm-btn" @click="confirmCopy">{{ t('validate') }}</button>
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
              :ticket="copyResult?.ticket ?? null"
              :fiscal-event-id="copyResult?.fiscalEventId ?? null"
            />
            <p v-if="copyResult && !copyResult.success" class="refund-error">{{ describeError(copyResult.error) }}</p>
            <button class="refund-cancel" @click="closeCopy">{{ t('finish') }}</button>
          </template>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.reports { display: flex; flex-direction: column; height: 100vh; background: var(--bg-primary); }
.header { display: flex; align-items: center; justify-content: space-between; padding: 0 var(--space-md); background: var(--bg-secondary); border-bottom: 1px solid rgba(160,160,176,0.1); height: 48px; }
.header h1 { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); }
.nav-btn { padding: var(--space-xs) var(--space-md); font-size: var(--font-size-xs); font-weight: var(--font-weight-bold); border-radius: var(--radius-sm); background: var(--bg-surface); color: var(--text-secondary); min-height: 36px; transition: transform var(--tap-duration) ease; }
.nav-btn:active { transform: scale(var(--tap-scale)); }

.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-xl);
  padding: var(--space-xl);
  overflow-y: auto;
}

.report-row { display: flex; align-items: center; justify-content: space-between; width: 100%; max-width: 600px; gap: var(--space-lg); }
.report-row h2 { font-size: var(--font-size-lg); }
.report-row p { font-size: var(--font-size-sm); color: var(--text-secondary); }

.report-btn {
  min-width: 160px; height: 72px;
  font-size: var(--font-size-lg); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  transition: transform var(--tap-duration) ease;
  display: inline-flex; align-items: center; justify-content: center;
}
.report-btn:active { transform: scale(var(--tap-scale)); }
.report-btn:disabled { opacity: 0.6; }
.x-btn { background: var(--bg-surface); color: var(--text-primary); }
.z-btn { background: var(--accent-danger); color: var(--text-on-accent); }
.z-btn.armed { background: var(--accent-warning); }

.spinner {
  width: 28px; height: 28px;
  border: 3px solid rgba(255, 255, 255, 0.25);
  border-top-color: currentColor;
  border-radius: 50%;
  animation: report-spin 800ms linear infinite;
}
@keyframes report-spin { to { transform: rotate(360deg); } }

/* ── Z History ────────────────────────────────────────────── */
.history-section { width: 100%; max-width: 600px; }
.history-section h3 { font-size: var(--font-size-md); font-weight: var(--font-weight-bold); color: var(--text-secondary); margin-bottom: var(--space-md); }
.empty { font-size: var(--font-size-sm); color: var(--text-secondary); }
.history-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: var(--space-sm); }
.history-row { display: flex; align-items: center; justify-content: space-between; padding: var(--space-sm) var(--space-md); background: var(--bg-secondary); border-radius: var(--radius-md); }
.history-label { font-size: var(--font-size-sm); font-weight: var(--font-weight-bold); font-variant-numeric: tabular-nums; }
.reprint-btn { min-height: 48px; padding: 0 var(--space-md); background: var(--bg-surface); color: var(--text-primary); border: none; border-radius: var(--radius-sm); font-size: var(--font-size-sm); font-weight: var(--font-weight-bold); transition: transform var(--tap-duration) ease; }
.reprint-btn:active { transform: scale(var(--tap-scale)); background: var(--bg-pressed); }

/* ── Cash management ──────────────────────────────────────── */
.cash-section { width: 100%; max-width: 600px; padding: var(--space-md); border: 1px solid rgba(160,160,176,0.15); border-radius: var(--radius-md); display: flex; flex-direction: column; gap: var(--space-md); }
.cash-section h3 { font-size: var(--font-size-md); font-weight: var(--font-weight-bold); color: var(--text-secondary); }

/* §7.3: preset amount buttons - no keyboard */
.cash-presets { display: flex; gap: var(--space-sm); flex-wrap: wrap; }
.cash-preset {
  flex: 1; min-width: 80px; height: 56px;
  font-size: var(--font-size-md); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
  font-variant-numeric: tabular-nums;
}
.cash-preset:active { transform: scale(var(--tap-scale)); }
.cash-preset.active { background: var(--accent-primary); color: var(--text-on-accent); }

/* Reason + direction buttons */
.cash-actions { display: flex; gap: var(--space-sm); }
.cash-action-btn {
  flex: 1; height: 52px;
  font-size: var(--font-size-sm); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md); color: var(--text-on-accent);
  transition: transform var(--tap-duration) ease;
}
.cash-action-btn:active { transform: scale(var(--tap-scale)); }
.cash-action-btn:disabled { opacity: 0.4; }
.cash-in { background: var(--accent-success); }
.cash-out { background: var(--accent-warning); }

.drawer-btn {
  width: 100%; height: 52px;
  font-size: var(--font-size-md); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.drawer-btn:active { transform: scale(var(--tap-scale)); }

.refund-btn {
  width: 100%; height: 52px;
  font-size: var(--font-size-md); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--accent-danger); color: var(--text-on-accent);
  transition: transform var(--tap-duration) ease;
}
.refund-btn:active { transform: scale(var(--tap-scale)); }

/* ── Refund modal ─────────────────────────────────────────── */
.refund-overlay {
  position: fixed; inset: 0; z-index: 1000;
  background: rgba(0, 0, 0, 0.7);
  display: flex; align-items: center; justify-content: center;
}
.refund-modal {
  background: var(--bg-primary); border-radius: var(--radius-lg);
  padding: var(--space-lg);
  width: 90vw; max-width: 600px; max-height: 85vh;
  display: flex; flex-direction: column; gap: var(--space-md);
  overflow-y: auto; -webkit-overflow-scrolling: touch;
}
.refund-modal h3 { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); text-align: center; }
.refund-empty { text-align: center; color: var(--text-secondary); font-size: var(--font-size-md); padding: var(--space-xl) 0; }
.refund-error { color: var(--accent-danger); font-size: var(--font-size-sm); text-align: center; }

.refund-ticket-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: var(--space-sm); overflow-y: auto; }
.refund-ticket-btn {
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: var(--space-xs);
  min-height: 100px; padding: var(--space-sm);
  background: var(--bg-surface); border-radius: var(--radius-md);
  transition: transform var(--tap-duration) ease;
}
.refund-ticket-btn:active { transform: scale(var(--tap-scale)); }
.rt-num { font-size: var(--font-size-md); font-weight: var(--font-weight-bold); }
.copy-btn {
  width: 100%; height: 52px;
  font-size: var(--font-size-md); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.copy-btn:active { transform: scale(var(--tap-scale)); }
.rt-kind { font-size: var(--font-size-xs); font-weight: var(--font-weight-bold); color: var(--text-secondary); }
.rt-table { font-size: var(--font-size-xs); color: var(--accent-primary); }
.rt-total { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); color: var(--accent-primary); font-variant-numeric: tabular-nums; }
.rt-time { font-size: var(--font-size-xs); color: var(--text-secondary); }

.refund-lines { display: flex; flex-direction: column; }
.refund-line {
  display: flex; align-items: center; justify-content: space-between;
  min-height: 64px; padding: var(--space-sm) var(--space-md);
  border-bottom: 1px solid rgba(160,160,176,0.1);
  transition: background var(--tap-duration) ease;
}
.refund-line.selected { background: rgba(231, 76, 60, 0.1); }
.rl-info { flex: 1; }
.rl-name { font-size: var(--font-size-sm); font-weight: var(--font-weight-bold); display: block; }
.rl-price { font-size: var(--font-size-sm); color: var(--text-secondary); font-variant-numeric: tabular-nums; }
.rl-orig-qty { font-size: var(--font-size-sm); color: var(--text-secondary); min-width: 40px; text-align: right; }
.rl-qty { display: flex; align-items: center; gap: var(--space-xs); font-size: var(--font-size-sm); font-weight: var(--font-weight-bold); }
.rl-qty-btn {
  width: 48px; height: 48px; border-radius: var(--radius-sm);
  background: var(--bg-surface); color: var(--text-primary);
  font-size: var(--font-size-lg); font-weight: var(--font-weight-bold);
  display: flex; align-items: center; justify-content: center;
  transition: transform var(--tap-duration) ease;
}
.rl-qty-btn:active { transform: scale(var(--tap-scale)); }
.rl-max { font-size: var(--font-size-xs); color: var(--text-secondary); }

.refund-confirm-btn {
  width: 100%; height: 72px;
  font-size: var(--font-size-lg); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--accent-danger); color: var(--text-on-accent);
  transition: transform var(--tap-duration) ease;
  font-variant-numeric: tabular-nums;
}
.refund-confirm-btn:active { transform: scale(var(--tap-scale)); }
.refund-confirm-btn:disabled { opacity: 0.3; pointer-events: none; }

.refund-cancel {
  width: 100%; height: 52px;
  font-size: var(--font-size-md); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-secondary);
  transition: transform var(--tap-duration) ease;
}
.refund-cancel:active { transform: scale(var(--tap-scale)); }

.refund-processing {
  display: flex; flex-direction: column; align-items: center;
  gap: var(--space-lg); padding: var(--space-xl) 0;
}
.refund-processing .spinner {
  width: 48px; height: 48px;
  border: 4px solid var(--bg-surface); border-top: 4px solid var(--accent-danger);
  border-radius: 50%; animation: report-spin 0.8s linear infinite;
}
.refund-processing p { color: var(--text-secondary); font-size: var(--font-size-md); }
</style>
