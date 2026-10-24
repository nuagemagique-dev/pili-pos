<script setup lang="ts">
// The issued invoices (Reports, tickets tab): number, customer, total,
// state of the Peppol delivery, and the copy (event C on the invoice,
// printed with the invoice layout; managers only, as the engine rules).
import { ref } from 'vue'
import { useInvoiceStore } from '../stores/invoice'
import { useAuthStore } from '../stores/auth'
import { useToastStore } from '../stores/toast'
import { formatPrice } from '../utils/format'
import { describeError } from '../utils/errorMessages'
import { useI18n } from '../i18n'

const inv = useInvoiceStore()
const auth = useAuthStore()
const toast = useToastStore()
const { t } = useI18n()

const copying = ref<number | null>(null)

/** "2026-10-17" (the Brussels date of the invoice) as "17/10/2026". */
function issueDay(isoDate: string): string {
  const [y, m, d] = isoDate.slice(0, 10).split('-')
  return y && m && d ? `${d}/${m}/${y}` : isoDate
}

function statusKey(status: string | null): string {
  if (status === 'queued' || status === 'accepted' || status === 'failed')
    return `invoice_status_${status}`
  return 'invoice_status_none'
}

async function copy(fiscalEventId: number) {
  if (copying.value !== null) return
  copying.value = fiscalEventId
  try {
    const result = await inv.copyInvoice(fiscalEventId)
    if (result.success) toast.show(t('report_reprint_ok'), 'success', 3000)
    else toast.show(describeError(result.error), 'error', 5000)
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
  } finally {
    copying.value = null
  }
}
</script>

<template>
  <div class="panel invoices-panel" data-testid="invoice-list">
    <div class="inv-list-head">
      <h2>{{ t('invoices_title') }}</h2>
      <p class="hint">{{ t('invoices_hint') }}</p>
    </div>
    <p v-if="inv.invoices.length === 0" class="none">{{ t('invoices_none') }}</p>
    <ul v-else class="inv-rows">
      <li
        v-for="i in inv.invoices"
        :key="i.id"
        class="inv-row"
        :data-testid="`invoice-row-${i.id}`"
      >
        <span class="no">{{ i.invoiceNo }}</span>
        <span class="who">
          <span class="who-name">{{ i.customerName }}</span>
          <span class="who-vat">{{ i.customerVatNumber }}</span>
        </span>
        <span class="when">{{ issueDay(i.issueDate) }}</span>
        <span class="amt">{{ formatPrice(i.totalCents) }} €</span>
        <span
          class="state"
          :class="`st-${i.deliveryStatus ?? 'none'}`"
          data-testid="invoice-status"
          >{{ t(statusKey(i.deliveryStatus)) }}</span
        >
        <span class="acts">
          <button
            v-if="auth.isManager"
            class="copy-btn"
            :data-testid="`invoice-copy-${i.id}`"
            :disabled="copying !== null"
            @click="copy(i.fiscalEventId)"
          >
            {{ t('invoice_copy') }}
          </button>
        </span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.panel {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 0;
}
.panel h2 {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}
.invoices-panel {
  flex: 2;
}
.inv-list-head {
  display: flex;
  align-items: baseline;
  gap: 24px;
}
.hint {
  font-size: 17px;
  color: var(--text-secondary);
}
.none {
  font-size: 18px;
  color: var(--text-secondary);
  padding: 16px 0;
  text-align: center;
}
.inv-rows {
  list-style: none;
  margin: 0;
  padding: 0;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
}
.inv-row {
  display: grid;
  grid-template-columns: 180px minmax(0, 1fr) 120px 140px 220px 120px;
  align-items: center;
  gap: 16px;
  min-height: 72px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}
.inv-row:last-child {
  border-bottom: none;
}
.no {
  font-family: var(--font-mono);
  font-size: 20px;
  font-weight: var(--font-weight-heavy);
}
.who {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.who-name {
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.who-vat {
  font-family: var(--font-mono);
  font-size: 14px;
  color: var(--text-secondary);
}
.when {
  font-family: var(--font-mono);
  font-size: 17px;
  color: var(--text-secondary);
}
.amt {
  font-family: var(--font-mono);
  font-size: 20px;
  font-weight: var(--font-weight-bold);
  text-align: right;
}
.state {
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
.st-queued {
  color: var(--accent-warning);
}
.st-accepted {
  color: var(--accent-success);
}
.st-failed {
  color: var(--accent-danger);
}
.acts {
  display: flex;
  justify-content: flex-end;
}
.copy-btn {
  height: 56px;
  min-width: 112px;
  padding: 0 20px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.copy-btn:active {
  transform: scale(var(--tap-scale));
}
.copy-btn:disabled {
  opacity: 0.4;
}
</style>
