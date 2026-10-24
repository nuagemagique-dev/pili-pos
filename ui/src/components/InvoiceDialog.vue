<script setup lang="ts">
// The B2B invoice dialog (event I, M150): customer, preview, signing,
// result. The state lives in the invoice store; the engine owns every
// rule (VAT check digits, lines and totals read from the archived
// sales, numbering, printing, the Peppol queue). Text is typed on the
// on-screen TextKeyboard (the kiosk has no OS keyboard, V11-D19).
import { computed } from 'vue'
import { useInvoiceStore, type CustomerField } from '../stores/invoice'
import { formatPrice } from '../utils/format'
import { describeError } from '../utils/errorMessages'
import { useI18n } from '../i18n'
import TextKeyboard from './TextKeyboard.vue'

const inv = useInvoiceStore()
const { t } = useI18n()

const STEPS = ['customer', 'preview', 'result'] as const
const stepIndex = computed(() => {
  if (inv.step === 'processing') return 1
  return STEPS.indexOf(inv.step as (typeof STEPS)[number])
})

/** The enter key of the keyboard walks the form top to bottom. */
const ORDER: CustomerField[] = ['vatNumber', 'name', 'street', 'postalCode', 'city', 'countryCode']

function nextField() {
  const at = ORDER.indexOf(inv.activeField)
  inv.activeField = ORDER[Math.min(at + 1, ORDER.length - 1)] ?? 'vatNumber'
}

const maxLength = computed(() => {
  if (inv.activeField === 'countryCode') return 2
  if (inv.activeField === 'vatNumber') return 16
  if (inv.activeField === 'postalCode') return 10
  return 80
})

const chipClass = computed(() => `chip-${inv.peppolStatus}`)
const chipText = computed(() => t(`invoice_peppol_${inv.peppolStatus}`))
const chipNote = computed(() => {
  if (inv.lookup && !inv.lookup.deliverable) return t('invoice_foreign_note')
  if (inv.peppolStatus === 'not_registered') return t('invoice_peppol_not_registered_note')
  if (inv.peppolStatus === 'unknown' && inv.lookup) return t('invoice_peppol_unknown_note')
  return null
})

const salesTotalCents = computed(() =>
  inv.sales
    .filter((s) => inv.selected.includes(s.fiscalEventId))
    .reduce((sum, s) => sum + s.totalCents, 0),
)

function field(f: CustomerField): string {
  return inv.fieldValue(f)
}

function quantity(milli: number): string {
  if (milli % 1000 === 0) return String(milli / 1000)
  return (milli / 1000).toLocaleString('fr-BE', { maximumFractionDigits: 3 })
}

function rate(hundredths: number): string {
  return `${(hundredths / 100).toLocaleString('fr-BE', { maximumFractionDigits: 2 })} %`
}

const deliveryText = computed(() =>
  inv.result?.deliveryStatus === 'queued'
    ? t('invoice_delivery_queued')
    : t('invoice_delivery_none'),
)

const fields: { id: CustomerField; label: string; wide?: boolean }[] = [
  { id: 'name', label: 'invoice_field_name', wide: true },
  { id: 'street', label: 'invoice_field_street', wide: true },
  { id: 'postalCode', label: 'invoice_field_postal_code' },
  { id: 'city', label: 'invoice_field_city' },
  { id: 'countryCode', label: 'invoice_field_country' },
]
</script>

<template>
  <Teleport to="body">
    <div
      v-if="inv.open"
      class="inv-overlay"
      data-testid="invoice-dialog"
      @click.self="inv.step === 'customer' && inv.closeDialog()"
    >
      <div class="inv-modal" role="dialog" :aria-label="t('invoice_title')">
        <header class="inv-head">
          <h3>{{ t('invoice_title') }}</h3>
          <ol class="inv-steps">
            <li :class="{ on: stepIndex === 0 }">{{ t('invoice_step_customer') }}</li>
            <li :class="{ on: stepIndex === 1 }">{{ t('invoice_step_preview') }}</li>
            <li :class="{ on: stepIndex === 2 }">{{ t('invoice_step_done') }}</li>
          </ol>
          <span class="inv-sales">{{
            t('invoice_sales_chosen', {
              n: inv.selected.length,
              total: formatPrice(salesTotalCents),
            })
          }}</span>
        </header>

        <!-- Step 1: the customer -->
        <template v-if="inv.step === 'customer'">
          <div class="inv-customer">
            <div class="inv-form">
              <div class="fld-block">
                <span class="fld-label">{{ t('invoice_search_label') }}</span>
                <button
                  class="fld"
                  :class="{ active: inv.activeField === 'search' }"
                  data-testid="invoice-search-field"
                  @click="inv.activeField = 'search'"
                >
                  <span v-if="field('search')">{{ field('search') }}</span>
                  <span v-else class="ph">{{ t('invoice_search_placeholder') }}</span>
                </button>
                <ul
                  v-if="inv.searchResults.length > 0"
                  class="results"
                  data-testid="invoice-search-results"
                >
                  <li v-for="c in inv.searchResults" :key="c.id">
                    <button class="result" @click="inv.pickCustomer(c)">
                      <span class="result-name">{{ c.name }}</span>
                      <span class="result-vat">{{ c.vatNumber }}</span>
                    </button>
                  </li>
                </ul>
                <p v-else-if="field('search')" class="muted" data-testid="invoice-no-match">
                  {{ t('invoice_no_match') }}
                </p>
              </div>

              <div class="fld-block">
                <span class="fld-label">{{ t('invoice_vat_label') }}</span>
                <div class="vat-row">
                  <button
                    class="fld vat"
                    :class="{ active: inv.activeField === 'vatNumber' }"
                    data-testid="invoice-vat-field"
                    @click="inv.activeField = 'vatNumber'"
                  >
                    <span v-if="field('vatNumber')">{{ field('vatNumber') }}</span>
                    <span v-else class="ph">BE0123456789</span>
                  </button>
                  <button
                    class="check-btn"
                    data-testid="invoice-vat-check"
                    :disabled="inv.lookupBusy || !field('vatNumber')"
                    @click="inv.checkVat()"
                  >
                    {{ inv.lookupBusy ? t('invoice_vat_checking') : t('invoice_vat_check') }}
                  </button>
                  <span
                    v-if="inv.lookup"
                    class="chip"
                    :class="chipClass"
                    data-testid="invoice-chip"
                    >{{ chipText }}</span
                  >
                </div>
                <p v-if="chipNote" class="muted" data-testid="invoice-chip-note">{{ chipNote }}</p>
              </div>

              <div class="grid">
                <div v-for="f in fields" :key="f.id" class="fld-block" :class="{ wide: f.wide }">
                  <span class="fld-label">{{ t(f.label) }}</span>
                  <button
                    class="fld"
                    :class="{ active: inv.activeField === f.id }"
                    :data-testid="`invoice-field-${f.id}`"
                    @click="inv.activeField = f.id"
                  >
                    {{ field(f.id) }}
                  </button>
                </div>
              </div>

              <p
                v-if="inv.customerError"
                class="inv-error"
                role="alert"
                data-testid="invoice-error"
              >
                {{ inv.customerError }}
              </p>
            </div>

            <TextKeyboard
              :model-value="field(inv.activeField)"
              :max-length="maxLength"
              @update:model-value="inv.setField(inv.activeField, $event)"
              @submit="nextField"
            />
          </div>
          <div class="inv-actions">
            <button class="act secondary" data-testid="invoice-cancel" @click="inv.closeDialog()">
              {{ t('back') }}
            </button>
            <button
              class="act primary"
              data-testid="invoice-continue"
              :disabled="!inv.formComplete || inv.busy || inv.lookupBusy"
              @click="inv.continueToPreview()"
            >
              {{ t('invoice_continue') }}
            </button>
          </div>
        </template>

        <!-- Step 2: what the invoice contains -->
        <template v-else-if="inv.step === 'preview' && inv.preview">
          <div class="inv-preview" data-testid="invoice-preview">
            <section class="pv-col">
              <h4>{{ t('invoice_preview_customer') }}</h4>
              <p class="pv-customer">
                <strong>{{ inv.form.name }}</strong
                ><br />
                {{ inv.form.street }}<br />
                {{ inv.form.postalCode }} {{ inv.form.city }} ({{ inv.form.countryCode }})<br />
                <span class="mono">{{ inv.form.vatNumber }}</span>
              </p>
              <span class="chip" :class="chipClass" data-testid="invoice-chip">{{ chipText }}</span>
              <p v-if="chipNote" class="muted">{{ chipNote }}</p>

              <h4>{{ t('invoice_preview_tickets') }}</h4>
              <ul class="pv-sales">
                <li v-for="s in inv.preview.sales" :key="s.fiscalEventId">
                  <span class="mono">#{{ s.ticketNumber }}</span>
                  <span class="mono muted"
                    >{{ s.fdmId }} {{ s.eventCounter }}/{{ s.totalCounter }}</span
                  >
                  <span class="mono" :class="{ neg: s.totalCents < 0 }"
                    >{{ formatPrice(s.totalCents) }} €</span
                  >
                </li>
              </ul>
            </section>

            <section class="pv-col wide">
              <h4>{{ t('invoice_preview_lines') }}</h4>
              <ul class="pv-lines" data-testid="invoice-lines">
                <li v-for="(l, i) in inv.preview.lines" :key="i">
                  <span class="mono qty">{{ quantity(l.quantityMilli) }}x</span>
                  <span class="desc">{{ l.description }}</span>
                  <span class="mono vl">{{ l.vatLabel }}</span>
                  <span class="mono amt" :class="{ neg: l.totalCents < 0 }">{{
                    formatPrice(l.totalCents)
                  }}</span>
                </li>
              </ul>

              <h4>{{ t('invoice_preview_vat') }}</h4>
              <table class="pv-vat" data-testid="invoice-vat">
                <thead>
                  <tr>
                    <th></th>
                    <th>{{ t('invoice_vat_base') }}</th>
                    <th>{{ t('invoice_vat_amount') }}</th>
                    <th>{{ t('invoice_vat_total') }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="v in inv.preview.vat" :key="v.label">
                    <td class="mono">{{ v.label }} · {{ rate(v.rateHundredths) }}</td>
                    <td class="mono">{{ formatPrice(v.baseCents) }}</td>
                    <td class="mono">{{ formatPrice(v.vatCents) }}</td>
                    <td class="mono">{{ formatPrice(v.totalCents) }}</td>
                  </tr>
                </tbody>
              </table>

              <p class="pv-total" data-testid="invoice-total">
                <span>{{ t('invoice_total') }}</span>
                <span class="mono">{{ formatPrice(inv.preview.totalCents) }} €</span>
              </p>
              <p class="muted" data-testid="invoice-already-paid">
                {{ t('invoice_already_paid', { total: formatPrice(inv.preview.totalCents) }) }}
              </p>
              <p
                v-for="(p, i) in inv.previewProblems"
                :key="i"
                class="inv-error"
                role="alert"
                data-testid="invoice-problem"
              >
                {{ p }}
              </p>
            </section>
          </div>
          <div class="inv-actions">
            <button class="act secondary" data-testid="invoice-back" @click="inv.backToCustomer()">
              {{ t('back') }}
            </button>
            <button
              class="act primary"
              data-testid="invoice-confirm"
              :disabled="inv.busy || inv.previewProblems.length > 0"
              @click="inv.confirm()"
            >
              {{ t('invoice_confirm') }}
            </button>
          </div>
        </template>

        <!-- Signing -->
        <template v-else-if="inv.step === 'processing'">
          <div class="inv-processing" data-testid="invoice-processing">
            <div class="spinner"></div>
            <p>{{ t('processing_fdm') }}</p>
          </div>
        </template>

        <!-- Step 3: the result -->
        <template v-else>
          <div class="inv-result" data-testid="invoice-result">
            <template v-if="inv.result?.success">
              <h4 class="ok">{{ t('invoice_done_title', { no: inv.result.invoiceNo ?? '' }) }}</h4>
              <p>{{ inv.result.printed ? t('invoice_printed') : t('invoice_not_printed') }}</p>
              <p class="muted">{{ deliveryText }}</p>
            </template>
            <template v-else>
              <h4 class="bad">{{ t('invoice_failed') }}</h4>
              <p class="inv-error" role="alert" data-testid="invoice-error">
                {{ describeError(inv.result?.error) }}
              </p>
            </template>
          </div>
          <div class="inv-actions">
            <button class="act primary" data-testid="invoice-finish" @click="inv.closeDialog()">
              {{ t('finish') }}
            </button>
          </div>
        </template>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.inv-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
}
.inv-modal {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 24px;
  width: 96vw;
  max-width: 1840px;
  max-height: 94vh;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
}
.inv-head {
  display: flex;
  align-items: center;
  gap: 24px;
}
.inv-head h3 {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}
.inv-steps {
  display: flex;
  gap: 8px;
  list-style: none;
  margin: 0;
  padding: 0;
}
.inv-steps li {
  padding: 8px 16px;
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.inv-steps li.on {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.inv-sales {
  margin-left: auto;
  font-family: var(--font-mono);
  font-size: var(--font-size-md);
  color: var(--text-secondary);
}

.inv-customer {
  display: flex;
  gap: 24px;
  align-items: flex-start;
}
.inv-form {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.fld-block {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}
.fld-label {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
.fld {
  min-height: 56px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  border: 2px solid transparent;
  text-align: left;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.fld.active {
  border-color: var(--accent-primary);
}
.ph {
  color: var(--text-secondary);
  font-weight: var(--font-weight-normal);
}
.vat-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.vat-row .vat {
  flex: 1;
  font-family: var(--font-mono);
}
.check-btn {
  height: 56px;
  padding: 0 20px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  border: 2px solid var(--accent-info);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.check-btn:disabled {
  opacity: 0.4;
}
.check-btn:active {
  transform: scale(var(--tap-scale));
}

.chip {
  padding: 8px 14px;
  border-radius: 999px;
  white-space: nowrap;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--text-on-accent);
}
.chip-registered {
  background: var(--accent-success);
}
.chip-not_registered {
  background: var(--accent-danger);
}
.chip-unknown {
  background: var(--accent-warning);
}

.grid {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 12px;
}
.grid .wide {
  grid-column: 1 / -1;
}
.grid .fld-block:nth-child(3) {
  grid-column: 1;
}
.grid .fld-block:nth-child(4) {
  grid-column: 2;
}
.grid .fld-block:nth-child(5) {
  grid-column: 3;
}

.results {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 232px;
  overflow-y: auto;
}
.result {
  width: 100%;
  min-height: 56px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: var(--font-size-md);
}
.result-name {
  font-weight: var(--font-weight-bold);
}
.result-vat {
  font-family: var(--font-mono);
  color: var(--text-secondary);
}
.result:active {
  transform: scale(var(--tap-scale));
}

.muted {
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}
.mono {
  font-family: var(--font-mono);
}
.neg {
  color: var(--accent-danger);
}
.inv-error {
  color: var(--accent-danger);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}

.inv-actions {
  display: flex;
  gap: 16px;
}
.act {
  height: 72px;
  border-radius: var(--radius-md);
  font-size: 22px;
  font-weight: var(--font-weight-heavy);
  transition: transform var(--tap-duration) ease;
}
.act:active {
  transform: scale(var(--tap-scale));
}
.act:disabled {
  opacity: 0.3;
  pointer-events: none;
}
.act.secondary {
  width: 240px;
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: 17px;
}
.act.primary {
  flex: 1;
  background: var(--accent-primary);
  color: var(--text-on-accent);
}

.inv-preview {
  display: flex;
  gap: 32px;
  align-items: flex-start;
}
.pv-col {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.pv-col.wide {
  flex: 2;
}
.pv-col h4 {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-heavy);
}
.pv-customer {
  font-size: var(--font-size-md);
  line-height: 1.5;
}
.pv-sales {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.pv-sales li {
  display: flex;
  gap: 16px;
  justify-content: space-between;
  font-size: var(--font-size-md);
}
.pv-lines {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 300px;
  overflow-y: auto;
}
.pv-lines li {
  display: grid;
  grid-template-columns: 56px minmax(0, 1fr) 32px 96px;
  gap: 12px;
  align-items: center;
  min-height: 44px;
  border-bottom: 1px solid var(--border);
  font-size: var(--font-size-md);
}
.pv-lines .amt {
  text-align: right;
}
.pv-lines .vl {
  color: var(--text-secondary);
}
.pv-vat {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-md);
}
.pv-vat th {
  text-align: right;
  color: var(--text-secondary);
  font-weight: var(--font-weight-bold);
  padding: 4px 8px;
}
.pv-vat td {
  text-align: right;
  padding: 6px 8px;
  border-top: 1px solid var(--border);
}
.pv-vat td:first-child {
  text-align: left;
}
.pv-total {
  display: flex;
  justify-content: space-between;
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
}

.inv-processing {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
  padding: 64px 0;
}
.inv-processing .spinner {
  width: 48px;
  height: 48px;
  border: 4px solid var(--bg-surface);
  border-top: 4px solid var(--accent-primary);
}
.inv-processing p {
  color: var(--text-secondary);
  font-size: var(--font-size-md);
}

.inv-result {
  display: flex;
  flex-direction: column;
  gap: 12px;
  align-items: center;
  padding: 48px 0;
  text-align: center;
  font-size: var(--font-size-lg);
}
.inv-result h4 {
  font-size: 32px;
  font-weight: var(--font-weight-heavy);
}
.inv-result .ok {
  color: var(--accent-success);
}
.inv-result .bad {
  color: var(--accent-danger);
}
</style>
