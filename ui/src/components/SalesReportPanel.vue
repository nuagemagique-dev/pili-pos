<script setup lang="ts">
// Non-fiscal sales report (V11-D20): by product, by hour and the concordance with the Z of each
// booking period, for today, yesterday, this week, this month or a typed range. Read only: the
// engine computes it from the signed archive; this panel never adds, signs or edits anything.
import { computed, onMounted, ref } from "vue";
import * as ipc from "../api/ipc";
import type { SalesReport } from "../types";
import { useI18n } from "../i18n";
import { useToastStore } from "../stores/toast";
import { describeError } from "../utils/errorMessages";
import { formatPrice } from "../utils/format";
import {
  calendarToday,
  parseTyped,
  presetRange,
  showDate,
  showTyped,
  type Preset,
} from "../utils/salesRange";
import NumPad from "./NumPad.vue";

const { t } = useI18n();
const toast = useToastStore();

const PRESETS: { id: Preset; label: string }[] = [
  { id: "today", label: "sales_today" },
  { id: "yesterday", label: "sales_yesterday" },
  { id: "week", label: "sales_week" },
  { id: "month", label: "sales_month" },
  { id: "custom", label: "sales_custom" },
];

const preset = ref<Preset>("today");
const report = ref<SalesReport | null>(null);
const loading = ref(false);
const printing = ref(false);
const error = ref<string | null>(null);
// The business date now, once the engine has said it (a service past midnight is still "today").
const businessDate = ref<string | null>(null);
// Custom range: the digits typed on the pad, per field.
const typing = ref<"from" | "to">("from");
const fromDigits = ref("");
const toDigits = ref("");
const sort = ref<"amount" | "quantity">("amount");

const range = ref<[string, string]>([calendarToday(), calendarToday()]);

async function load(from: string, to: string): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    const r = await ipc.salesReport(from, to);
    report.value = r;
    range.value = [from, to];
    businessDate.value = r.businessDate;
  } catch (e) {
    report.value = null;
    error.value = describeError(e);
  } finally {
    loading.value = false;
  }
}

async function pick(next: Preset): Promise<void> {
  preset.value = next;
  if (next === "custom") {
    // Start from empty fields: the range on screen stays until a new one is shown.
    fromDigits.value = "";
    toDigits.value = "";
    typing.value = "from";
    return;
  }
  const [from, to] = presetRange(next, businessDate.value ?? calendarToday());
  await load(from, to);
  // The first guess of "today" was the calendar date: once the engine gives the business date,
  // a preset that moved is loaded again, once.
  const business = businessDate.value;
  if (business) {
    const [f2, t2] = presetRange(next, business);
    if (f2 !== from || t2 !== to) await load(f2, t2);
  }
}

onMounted(() => {
  void pick("today");
});

const typedFrom = computed(() => parseTyped(fromDigits.value));
const typedTo = computed(() => parseTyped(toDigits.value));
const canApply = computed(() => typedFrom.value !== null && typedTo.value !== null && !loading.value);

function onKey(key: string): void {
  const target = typing.value === "from" ? fromDigits : toDigits;
  if (key === "BACKSPACE") target.value = target.value.slice(0, -1);
  else if (key === "CLEAR") target.value = "";
  else if (target.value.length < 8) target.value += key;
  // The "from" date done: carry on with the "to" date.
  if (typing.value === "from" && target.value.length === 8) typing.value = "to";
}

async function applyCustom(): Promise<void> {
  if (typedFrom.value && typedTo.value) await load(typedFrom.value, typedTo.value);
}

const products = computed(() => {
  const rows = [...(report.value?.byProduct ?? [])];
  rows.sort((a, b) =>
    sort.value === "amount"
      ? b.amountCents - a.amountCents || a.name.localeCompare(b.name)
      : b.quantityMilli - a.quantityMilli || a.name.localeCompare(b.name),
  );
  return rows;
});

function quantity(milli: number): string {
  if (milli % 1000 === 0) return String(milli / 1000);
  return (milli / 1000).toLocaleString("fr-BE", { minimumFractionDigits: 3, maximumFractionDigits: 3 });
}

function euros(cents: number): string {
  return `${formatPrice(cents)} €`;
}

function dayText(d: SalesReport["days"][number]): string {
  if (d.open) return t("sales_open");
  if (d.zNumber === null || d.zTotalCents === null) return t("sales_z_missing");
  if (d.matchesZ) return t("sales_z_equal", { no: d.zNumber });
  return t("sales_z_gap", {
    no: d.zNumber,
    gap: formatPrice(d.gapCents ?? 0),
    z: formatPrice(d.zTotalCents),
  });
}

async function print(): Promise<void> {
  if (printing.value) return;
  printing.value = true;
  try {
    await ipc.printSalesReport(range.value[0], range.value[1]);
    toast.show(t("sales_printed"), "success", 4000);
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
  } finally {
    printing.value = false;
  }
}
</script>

<template>
  <section class="sales" data-testid="sales-report">
    <div class="panel controls">
      <div class="head">
        <h2>{{ t('sales_title') }}</h2>
        <button
          class="print-btn"
          data-testid="sales-print"
          :disabled="printing || !report"
          @click="print"
        >{{ t('sales_print') }}</button>
      </div>
      <p class="notice" data-testid="sales-notice">{{ t('sales_not_fiscal') }}</p>
      <div class="presets" role="tablist">
        <button
          v-for="p in PRESETS"
          :key="p.id"
          class="preset"
          :class="{ active: preset === p.id }"
          :data-testid="`sales-preset-${p.id}`"
          @click="pick(p.id)"
        >{{ t(p.label) }}</button>
      </div>

      <div v-if="preset === 'custom'" class="custom" data-testid="sales-custom">
        <div class="fields">
          <button
            class="field"
            :class="{ active: typing === 'from' }"
            data-testid="sales-field-from"
            @click="typing = 'from'"
          ><span class="field-label">{{ t('sales_from') }}</span><span class="mono">{{ showTyped(fromDigits) }}</span></button>
          <button
            class="field"
            :class="{ active: typing === 'to' }"
            data-testid="sales-field-to"
            @click="typing = 'to'"
          ><span class="field-label">{{ t('sales_to') }}</span><span class="mono">{{ showTyped(toDigits) }}</span></button>
        </div>
        <NumPad :model-value="typing === 'from' ? fromDigits : toDigits" @update:model-value="onKey" />
        <button class="apply-btn" data-testid="sales-apply" :disabled="!canApply" @click="applyCustom">{{ t('sales_apply') }}</button>
      </div>

      <p v-if="report" class="range" data-testid="sales-range">{{ showDate(range[0]) }} - {{ showDate(range[1]) }}</p>
      <p v-if="loading" class="status">{{ t('sales_loading') }}</p>
      <p v-else-if="error" class="status error" data-testid="sales-error">{{ error }}</p>

      <div v-if="report" class="concordance">
        <h3>{{ t('sales_concordance') }}</h3>
        <p v-if="report.days.length === 0" class="status">{{ t('sales_empty') }}</p>
        <ul class="days scroll-touch">
          <li
            v-for="(d, i) in report.days"
            :key="i"
            class="day"
            :class="{ open: d.open, gap: !d.open && !d.matchesZ }"
            data-testid="sales-day"
          >
            <span class="day-date mono">{{ showDate(d.bookingDate) }}</span>
            <span class="day-text">{{ dayText(d) }}</span>
            <span class="day-total mono">{{ euros(d.totalCents) }}</span>
          </li>
        </ul>
      </div>

      <dl v-if="report" class="totals">
        <div><dt>{{ t('sales_tickets') }}</dt><dd class="mono" data-testid="sales-tickets">{{ report.ticketCount }}</dd></div>
        <div v-if="report.refundsCents !== 0"><dt>{{ t('sales_refunds') }}</dt><dd class="mono" data-testid="sales-refunds">{{ euros(report.refundsCents) }}</dd></div>
        <div v-if="report.priceChangesCents !== 0"><dt>{{ t('sales_price_changes') }}</dt><dd class="mono" data-testid="sales-price-changes">{{ euros(report.priceChangesCents) }}</dd></div>
        <div v-if="report.roundingCents !== 0"><dt>{{ t('sales_rounding') }}</dt><dd class="mono" data-testid="sales-rounding">{{ euros(report.roundingCents) }}</dd></div>
        <div class="grand"><dt>{{ t('sales_total') }}</dt><dd class="mono" data-testid="sales-total">{{ euros(report.totalCents) }}</dd></div>
      </dl>
    </div>

    <div class="panel tables">
      <div class="products">
        <div class="table-head">
          <h3>{{ t('sales_by_product') }}</h3>
          <div class="sorts">
            <button
              class="sort"
              :class="{ active: sort === 'amount' }"
              data-testid="sales-sort-amount"
              @click="sort = 'amount'"
            >{{ t('sales_sort_amount') }}</button>
            <button
              class="sort"
              :class="{ active: sort === 'quantity' }"
              data-testid="sales-sort-quantity"
              @click="sort = 'quantity'"
            >{{ t('sales_sort_quantity') }}</button>
          </div>
        </div>
        <div class="cols"><span>{{ t('sales_col_product') }}</span><span>{{ t('sales_col_qty') }}</span><span>{{ t('sales_col_amount') }}</span></div>
        <ul class="rows scroll-touch">
          <li v-for="p in products" :key="p.productId + p.name" class="prow" data-testid="sales-product">
            <span class="pname">{{ p.name }}</span>
            <span class="mono" :class="{ negative: p.quantityMilli < 0 }">{{ quantity(p.quantityMilli) }}</span>
            <span class="mono" :class="{ negative: p.amountCents < 0 }">{{ euros(p.amountCents) }}</span>
          </li>
        </ul>
      </div>
      <div class="hours">
        <h3>{{ t('sales_by_hour') }}</h3>
        <ul class="bars scroll-touch">
          <li v-for="h in report?.byHour ?? []" :key="h.hour" class="hour" data-testid="sales-hour">
            <span class="hour-label mono">{{ String(h.hour).padStart(2, '0') }}h</span>
            <span class="bar-track">
              <span
                class="bar"
                data-testid="sales-hour-bar"
                :style="{ width: `${Math.max(2, h.barPercent)}%` }"
              ></span>
            </span>
            <span class="hour-amount mono">{{ euros(h.amountCents) }}</span>
            <span class="hour-tickets">{{ t('sales_hour_tickets', { n: h.tickets }) }}</span>
          </li>
        </ul>
      </div>
    </div>
  </section>
</template>

<style scoped>
.sales { flex: 1; min-height: 0; display: grid; grid-template-columns: 460px minmax(0, 1fr); gap: 24px; }
.panel {
  background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: var(--radius-lg); padding: 20px;
  display: flex; flex-direction: column; gap: 12px; min-height: 0; overflow: hidden;
}
.panel h2 { font-size: 26px; font-weight: var(--font-weight-heavy); line-height: 1.2; }
h3 { font-size: 20px; font-weight: var(--font-weight-bold); }
.head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.notice { font-size: 15px; color: var(--text-secondary); }
.mono { font-family: var(--font-mono); }
.negative { color: var(--accent-danger); }

.print-btn, .apply-btn {
  height: 56px; padding: 0 24px; border-radius: var(--radius-md);
  background: var(--selected-bg); color: var(--selected-fg);
  font-size: 18px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.print-btn:active, .apply-btn:active { transform: scale(var(--tap-scale)); }
.print-btn:disabled, .apply-btn:disabled { opacity: 0.5; }

.presets { display: grid; grid-template-columns: repeat(3, 1fr); gap: 4px; padding: 4px; background: var(--bg-surface); border-radius: 12px; }
.preset {
  height: 56px; border-radius: 8px; font-size: 17px; font-weight: var(--font-weight-bold);
  color: var(--text-primary); background: transparent;
  transition: transform var(--tap-duration) ease;
}
.preset.active { background: var(--selected-bg); color: var(--selected-fg); }
.preset:active { transform: scale(var(--tap-scale)); }

.custom { display: flex; flex-direction: column; align-items: center; gap: 10px; }
.fields { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; width: 100%; }
.field {
  height: 64px; border-radius: var(--radius-md); background: var(--bg-surface); color: var(--text-primary);
  display: flex; flex-direction: column; align-items: flex-start; justify-content: center; padding: 0 14px;
  border: 2px solid transparent; font-size: 20px;
}
.field.active { border-color: var(--selected-bg); }
.field-label { font-size: 13px; color: var(--text-secondary); }

.range { font-size: 18px; font-weight: var(--font-weight-bold); }
.status { font-size: 16px; color: var(--text-secondary); }
.status.error { color: var(--accent-danger); }

.concordance { display: flex; flex-direction: column; gap: 6px; min-height: 0; flex: 1; }
.days { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; overflow-y: auto; min-height: 0; }
.day {
  display: grid; grid-template-columns: auto 1fr auto; gap: 10px; align-items: center;
  padding: 10px 12px; border-radius: var(--radius-md); background: var(--bg-surface); font-size: 16px;
}
.day.open { border-left: 6px solid var(--accent-warning); }
.day.gap { border-left: 6px solid var(--accent-danger); }
.day:not(.open):not(.gap) { border-left: 6px solid var(--accent-success, var(--selected-bg)); }
.day-text { color: var(--text-secondary); }

.totals { margin: 0; display: flex; flex-direction: column; gap: 4px; padding-top: 8px; border-top: 1px solid var(--border); }
.totals div { display: flex; justify-content: space-between; font-size: 17px; }
.totals dt { color: var(--text-secondary); }
.totals dd { margin: 0; }
.totals .grand { font-size: 24px; font-weight: var(--font-weight-heavy); }
.totals .grand dt { color: var(--text-primary); }

.tables { display: grid; grid-template-rows: minmax(0, 3fr) minmax(0, 2fr); gap: 16px; }
.products, .hours { display: flex; flex-direction: column; gap: 8px; min-height: 0; }
.table-head { display: flex; align-items: center; justify-content: space-between; }
.sorts { display: flex; gap: 4px; }
.sort {
  height: 44px; padding: 0 16px; border-radius: 8px; background: var(--bg-surface);
  color: var(--text-primary); font-size: 15px; font-weight: var(--font-weight-bold);
}
.sort.active { background: var(--selected-bg); color: var(--selected-fg); }
.cols, .prow { display: grid; grid-template-columns: minmax(0, 1fr) 90px 130px; gap: 10px; align-items: center; }
.cols { font-size: 13px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.06em; }
.cols span:not(:first-child), .prow span:not(.pname) { text-align: right; }
.rows, .bars { list-style: none; margin: 0; padding: 0; overflow-y: auto; min-height: 0; flex: 1; }
.prow { padding: 8px 4px; border-bottom: 1px solid var(--border); font-size: 17px; }
.pname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.hour { display: grid; grid-template-columns: 56px minmax(0, 1fr) 110px 90px; gap: 10px; align-items: center; padding: 4px 0; font-size: 15px; }
.bar-track { height: 18px; background: var(--bg-surface); border-radius: 9px; overflow: hidden; display: block; }
.bar { display: block; height: 100%; background: var(--selected-bg); border-radius: 9px; }
.hour-amount { text-align: right; }
.hour-tickets { color: var(--text-secondary); text-align: right; font-size: 13px; }
</style>
