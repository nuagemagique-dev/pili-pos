<script setup lang="ts">
// Correct the payment method of one payment line of a sale signed today
// (event F, command correct_payment). The engine decides what can be
// corrected (list_sale_payments); the screen shows the lines, lets the
// manager pick the line and the real method, then asks for the PIN.
import { computed, onMounted, ref } from "vue";
import * as ipc from "../api/ipc";
import type { CheckoutPaymentMethod } from "../types";
import { useAuthStore } from "../stores/auth";
import { useI18n } from "../i18n";
import { describeError } from "../utils/errorMessages";
import { formatPrice } from "../utils/format";
import PinDialog from "./PinDialog.vue";

const props = defineProps<{
  /** The signed sale (fiscal event id). */
  fiscalEventId: number;
  ticketNumber: number;
}>();

const emit = defineEmits<{
  /** The correction was signed. */
  done: [];
  cancel: [];
}>();

const { t } = useI18n();
const auth = useAuthStore();

const METHODS: CheckoutPaymentMethod[] = ["cash", "card", "meal_voucher"];

const sale = ref<ipc.SalePayments | null>(null);
const loadError = ref<string | null>(null);
const lineIndex = ref<number | null>(null);
const method = ref<CheckoutPaymentMethod | null>(null);
const askPin = ref(false);

/** The method a payment line of this type belongs to. */
function methodOf(paymentType: string): CheckoutPaymentMethod | null {
  if (paymentType === "CASH") return "cash";
  if (paymentType.startsWith("CARD")) return "card";
  if (paymentType === "CHEQUE_MEAL") return "meal_voucher";
  return null;
}

function methodLabel(paymentType: string): string {
  const m = methodOf(paymentType);
  return m ? t(`reports_pc_method_${m}`) : paymentType;
}

/** The lines that can be picked: typed by hand, a payment (not the rounding). */
const selectable = computed(() =>
  (sale.value?.lines ?? []).filter(l => l.amountType === "PAYMENT"),
);

const picked = computed(
  () => selectable.value.find(l => l.index === lineIndex.value) ?? null,
);

function isLocked(line: ipc.SalePaymentLine): boolean {
  return line.inputMethod === "AUTOMATIC";
}

const pickedMethod = computed(() => (picked.value ? methodOf(picked.value.paymentType) : null));

const summary = computed(() => {
  if (!picked.value || !method.value) return "";
  const amount = formatPrice(picked.value.amountCents);
  return t("reports_pc_summary", {
    from: methodLabel(picked.value.paymentType),
    to: t(`reports_pc_method_${method.value}`),
    amount,
  });
});

const corrected = computed(() => {
  const c = sale.value?.corrected;
  return c
    ? t("reports_pc_corrected", { from: methodLabel(c.fromType), to: methodLabel(c.toType) })
    : "";
});

onMounted(async () => {
  try {
    const loaded = await ipc.listSalePayments(props.fiscalEventId);
    sale.value = loaded;
    const free = loaded.lines.filter(l => l.amountType === "PAYMENT" && !isLocked(l));
    if (loaded.correctable && free.length === 1) lineIndex.value = free[0].index;
  } catch (e) {
    loadError.value = describeError(e);
  }
});

function pickLine(line: ipc.SalePaymentLine) {
  if (isLocked(line)) return;
  lineIndex.value = line.index;
  if (method.value && method.value === methodOf(line.paymentType)) method.value = null;
}

async function verify(pin: string) {
  const insz = auth.currentUser?.insz;
  if (!insz || lineIndex.value === null || !method.value) throw new Error("No correction");
  const result = await ipc.correctPayment(props.fiscalEventId, lineIndex.value, method.value, insz, pin);
  // The error code as the backend sent it, for describeError.
  if (!result.success) throw result.error ?? "";
}
</script>

<template>
  <div class="overlay" @click.self="emit('cancel')">
    <div class="modal" role="dialog" :aria-label="t('reports_pc_title')" data-testid="pc-dialog">
      <h3>
        {{ t('reports_pc_title') }}<span class="modal-ticket"> · #{{ ticketNumber }}</span>
      </h3>

      <p v-if="loadError" class="refusal" data-testid="pc-refusal">{{ loadError }}</p>
      <p v-else-if="!sale" class="muted">…</p>

      <template v-else-if="!sale.correctable">
        <p class="refusal" data-testid="pc-refusal">
          {{ sale.refusal ? describeError(sale.refusal) : t('reports_pc_not_possible') }}
        </p>
        <p v-if="corrected" class="done-note" data-testid="pc-corrected">{{ corrected }}</p>
      </template>

      <template v-else>
        <p class="step-title">{{ t('reports_pc_pick_line') }}</p>
        <div class="lines">
          <button
            v-for="line in selectable"
            :key="line.index"
            class="pay-line"
            :class="{ selected: lineIndex === line.index, locked: isLocked(line) }"
            :disabled="isLocked(line)"
            :data-testid="`pc-line-${line.index}`"
            @click="pickLine(line)"
          >
            <span class="pl-text">
              {{ t('reports_pc_line', { method: methodLabel(line.paymentType), amount: formatPrice(line.amountCents) }) }}
              <span v-if="line.brand" class="pl-brand"> · {{ line.brand }}</span>
            </span>
            <span v-if="isLocked(line)" class="pl-mark">{{ t('reports_pc_terminal') }}</span>
          </button>
        </div>
        <p v-if="selectable.some(isLocked)" class="muted">{{ t('reports_pc_terminal_note') }}</p>

        <p class="step-title">{{ t('reports_pc_pick_method') }}</p>
        <div class="methods">
          <button
            v-for="m in METHODS"
            :key="m"
            class="method-btn"
            :class="{ selected: method === m }"
            :disabled="lineIndex === null || m === pickedMethod"
            :data-testid="`pc-method-${m}`"
            @click="method = m"
          >{{ t(`reports_pc_method_${m}`) }}</button>
        </div>

        <p v-if="summary" class="summary" data-testid="pc-summary">{{ summary }}</p>

        <button
          class="confirm-btn"
          :disabled="lineIndex === null || method === null"
          data-testid="pc-confirm"
          @click="askPin = true"
        >{{ t('reports_pc_confirm') }}</button>
      </template>

      <button class="cancel-btn" data-testid="pc-close" @click="emit('cancel')">{{ t('back') }}</button>
    </div>

    <PinDialog
      v-if="askPin && auth.currentUser"
      :insz="auth.currentUser.insz"
      :user-name="auth.currentUser.name"
      :hint="t('reports_pc_pin_hint')"
      :verify="verify"
      @done="emit('done')"
      @cancel="askPin = false"
    />
  </div>
</template>

<style scoped>
.overlay {
  position: fixed; inset: 0; z-index: 1000;
  background: rgba(0, 0, 0, 0.7);
  display: flex; align-items: center; justify-content: center;
}
.modal {
  background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 24px;
  width: 90vw; max-width: 640px; max-height: 85vh;
  display: flex; flex-direction: column; gap: 16px;
  overflow-y: auto; -webkit-overflow-scrolling: touch;
}
h3 { font-size: 28px; font-weight: var(--font-weight-heavy); line-height: 1.2; }
.modal-ticket { font-family: var(--font-mono); color: var(--text-secondary); }
.step-title { font-size: var(--font-size-md); font-weight: var(--font-weight-bold); }
.muted { color: var(--text-secondary); font-size: 16px; }
.refusal { color: var(--accent-danger); font-size: var(--font-size-md); font-weight: var(--font-weight-bold); }
.done-note { font-size: var(--font-size-md); color: var(--text-primary); }

.lines, .methods { display: flex; flex-direction: column; gap: 8px; }
.methods { flex-direction: row; }
.pay-line {
  min-height: 64px; padding: 8px 20px; border-radius: var(--radius-md);
  display: flex; align-items: center; justify-content: space-between; gap: 12px;
  background: var(--bg-surface); color: var(--text-primary);
  border: 2px solid transparent;
  font-size: 20px; font-weight: var(--font-weight-bold);
  font-variant-numeric: tabular-nums;
  transition: transform var(--tap-duration) ease;
}
.pay-line:active { transform: scale(var(--tap-scale)); }
.pay-line.selected { border-color: var(--accent-primary); }
.pay-line.locked { opacity: 0.5; }
.pl-brand { color: var(--text-secondary); font-weight: normal; }
.pl-mark {
  font-size: 15px; padding: 4px 10px; border-radius: var(--radius-sm);
  background: var(--bg-secondary); color: var(--text-secondary);
}

.method-btn {
  flex: 1; height: 72px; border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-primary);
  border: 2px solid transparent;
  font-size: 20px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.method-btn:active { transform: scale(var(--tap-scale)); }
.method-btn.selected {
  background: var(--accent-primary); color: var(--text-on-accent); border-color: var(--accent-primary);
}
.method-btn:disabled { opacity: 0.3; pointer-events: none; }

.summary {
  font-size: 22px; font-weight: var(--font-weight-heavy); text-align: center;
  font-variant-numeric: tabular-nums;
}
.confirm-btn {
  width: 100%; height: 72px; flex-shrink: 0;
  font-size: 22px; font-weight: var(--font-weight-heavy);
  border-radius: var(--radius-md);
  background: var(--accent-primary); color: var(--text-on-accent);
  transition: transform var(--tap-duration) ease;
}
.confirm-btn:active { transform: scale(var(--tap-scale)); }
.confirm-btn:disabled { opacity: 0.3; pointer-events: none; }
.cancel-btn {
  width: 100%; height: 56px; flex-shrink: 0;
  font-size: 17px; font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-secondary);
  transition: transform var(--tap-duration) ease;
}
.cancel-btn:active { transform: scale(var(--tap-scale)); }
</style>
