<script setup lang="ts">
// Reports, "Bons cadeaux": check a voucher by its code (cashiers and managers; the engine refuses a waiter), and,
// for a manager, the list of vouchers issued with a status filter and
// the DUPLICATA reprint. Codes are masked everywhere: the full code is
// on the printed voucher only.
import { onMounted, ref } from "vue";
import * as ipc from "../api/ipc";
import { formatPrice } from "../utils/format";
import { describeError } from "../utils/errorMessages";
import { useAuthStore } from "../stores/auth";
import { useToastStore } from "../stores/toast";
import { useI18n } from "../i18n";
import { warnIfSlipFailed } from "../stores/order";
import NumPad from "./NumPad.vue";

const { t } = useI18n();
const auth = useAuthStore();
const toast = useToastStore();

const FILTERS = ["", "active", "used", "expired", "cancelled"] as const;
const CODE_LENGTH = 12;

const digits = ref("");
const found = ref<ipc.GiftVoucherView | null>(null);
const lookupError = ref<string | null>(null);
const busy = ref(false);

const filter = ref<(typeof FILTERS)[number]>("");
const list = ref<ipc.GiftVoucherView[]>([]);
const listError = ref<string | null>(null);
const open = ref<number | null>(null);

function displayDate(iso: string): string {
  const [y, m, d] = iso.split("-");
  return d && m && y ? `${d}.${m}.${y}` : iso;
}

function onKey(key: string) {
  lookupError.value = null;
  found.value = null;
  if (key === "BACKSPACE") digits.value = digits.value.slice(0, -1);
  else if (key === "CLEAR") digits.value = "";
  else if (digits.value.length < CODE_LENGTH) digits.value += key;
}

async function lookup() {
  if (digits.value.length !== CODE_LENGTH || busy.value) return;
  busy.value = true;
  lookupError.value = null;
  try {
    found.value = await ipc.lookupGiftVoucher(digits.value);
  } catch (e) {
    found.value = null;
    lookupError.value = describeError(e);
  } finally {
    busy.value = false;
  }
}

async function loadList() {
  if (!auth.isManager) return;
  listError.value = null;
  try {
    list.value = await ipc.listGiftVouchers(filter.value || undefined);
  } catch (e) {
    listError.value = describeError(e);
  }
}

function pick(f: (typeof FILTERS)[number]) {
  filter.value = f;
  void loadList();
}

async function reprint(v: ipc.GiftVoucherView) {
  try {
    const outcome = await ipc.printGiftVoucher(v.id);
    if (outcome === "printed") toast.show(t("gv_reprinted"), "success", 4000);
    else warnIfSlipFailed(outcome === "failed" ? "failed" : outcome);
    if (outcome === "no_printer") toast.show(t("gv_print_no_printer"), "warning", 6000);
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
  }
}

onMounted(loadList);
</script>

<template>
  <section class="tab-body vouchers" data-testid="gv-report">
    <div class="panel lookup-panel">
      <h2>{{ t('gv_lookup_title') }}</h2>
      <p class="code" data-testid="gv-report-code">{{ (digits.match(/.{1,4}/g) ?? []).join(' ') || ' ' }}</p>
      <NumPad :model-value="digits" @update:model-value="onKey" />
      <button class="act" :disabled="digits.length !== CODE_LENGTH || busy" data-testid="gv-report-lookup" @click="lookup">
        {{ t('gv_lookup') }}
      </button>
      <p v-if="lookupError" class="call-error" role="alert" data-testid="gv-report-error">{{ lookupError }}</p>
      <div v-if="found" class="card" data-testid="gv-report-found">
        <div class="row">
          <span>{{ found.code_masked }}</span>
          <span class="status" :class="found.status">{{ t(`gv_status_${found.status}`) }}</span>
        </div>
        <div class="row"><span>{{ t('gv_balance') }}</span><strong>{{ formatPrice(found.balance_cents) }} €</strong></div>
        <div class="row small"><span>{{ t('gv_expires', { date: displayDate(found.expires_on) }) }}</span></div>
      </div>
    </div>

    <div v-if="auth.isManager" class="panel list-panel">
      <h2>{{ t('gv_list_title') }}</h2>
      <div class="filters" role="tablist">
        <button
          v-for="f in FILTERS"
          :key="f || 'all'"
          class="filter"
          :class="{ active: filter === f }"
          :data-testid="`gv-filter-${f || 'all'}`"
          @click="pick(f)"
        >{{ f ? t(`gv_status_${f}`) : t('gv_filter_all') }}</button>
      </div>
      <p v-if="listError" class="call-error" role="alert">{{ listError }}</p>
      <p v-else-if="list.length === 0" class="empty" data-testid="gv-empty">{{ t('gv_empty') }}</p>
      <ul v-else class="vouchers-list" data-testid="gv-list">
        <li v-for="v in list" :key="v.id" class="voucher" :data-testid="`gv-row-${v.id}`">
          <button class="voucher-head" @click="open = open === v.id ? null : v.id">
            <span class="mono">{{ v.code_masked }}</span>
            <span class="status" :class="v.status">{{ t(`gv_status_${v.status}`) }}</span>
            <strong>{{ formatPrice(v.balance_cents) }} / {{ formatPrice(v.face_value_cents) }} €</strong>
          </button>
          <div v-if="open === v.id" class="detail">
            <div class="row small">
              <span>{{ t('gv_issued_on', { date: displayDate(v.issued_on) }) }}</span>
              <span>{{ t('gv_expires', { date: displayDate(v.expires_on) }) }}</span>
            </div>
            <div class="row small"><span>{{ t('gv_movements') }}</span></div>
            <div v-for="(m, i) in v.movements" :key="i" class="row small">
              <span>{{ t(`gv_movement_${m.kind}`) }} · {{ displayDate(m.date) }} · {{ m.ticket }}</span>
              <span>{{ formatPrice(m.amount_cents) }} €</span>
            </div>
            <button class="act" :data-testid="`gv-reprint-${v.id}`" @click="reprint(v)">{{ t('gv_reprint') }}</button>
          </div>
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.vouchers { flex-direction: row; gap: 24px; align-items: flex-start; }
.panel { display: flex; flex-direction: column; gap: 12px; padding: 20px; border-radius: var(--radius-md); background: var(--bg-secondary); }
.lookup-panel { align-items: center; flex-shrink: 0; }
.list-panel { flex: 1; min-width: 0; max-height: 100%; overflow-y: auto; }
h2 { font-size: 24px; font-weight: var(--font-weight-heavy); }
.code {
  min-width: 320px; text-align: center; font-family: var(--font-mono);
  font-size: var(--font-size-xl); font-weight: var(--font-weight-bold); color: var(--accent-primary);
}
.act {
  height: 56px; padding: 0 20px; border-radius: var(--radius-md);
  background: var(--accent-primary); color: var(--text-on-accent);
  font-size: 16px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.act:active { transform: scale(var(--tap-scale)); }
.act:disabled { opacity: 0.4; }
.call-error { font-size: 15px; color: var(--accent-danger); overflow-wrap: anywhere; }
.card { width: 100%; display: flex; flex-direction: column; gap: 6px; font-size: var(--font-size-md); }
.row { display: flex; justify-content: space-between; gap: 16px; align-items: baseline; }
.row.small { font-size: var(--font-size-sm); color: var(--text-secondary); }
.status { font-weight: var(--font-weight-bold); }
.status.active { color: var(--accent-success); }
.status.used, .status.expired, .status.cancelled { color: var(--accent-danger); }
.filters { display: flex; flex-wrap: wrap; gap: 8px; }
.filter {
  height: 48px; padding: 0 16px; border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-primary); font-weight: var(--font-weight-bold);
}
.filter.active { background: var(--selected-bg); color: var(--selected-fg); }
.empty { color: var(--text-secondary); }
.vouchers-list { margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; }
.voucher { border-bottom: 1px solid var(--border); }
.voucher-head {
  width: 100%; min-height: 64px; display: grid; grid-template-columns: 1fr auto auto;
  gap: 16px; align-items: center; text-align: left; font-size: 17px;
}
.mono { font-family: var(--font-mono); }
.detail { display: flex; flex-direction: column; gap: 6px; padding: 8px 0 16px; }
</style>
