<script setup lang="ts">
import { computed } from 'vue'
import type { OrderLine, VatCode } from '../types'
import { formatPrice as fmt } from '../utils/format'
import { useI18n } from '../i18n'
import { lineNetCents } from '../utils/cartAmounts'

const { t } = useI18n()

const props = defineProps<{
  /** Cart lines: `discount_cents` (negative) is taken off their amount. */
  lines: Array<OrderLine & { discount_cents?: number }>
}>()

const VAT_RATES: Record<VatCode, number> = { A: 21, B: 12, C: 6, D: 0, X: 0 }

interface VatRow {
  code: VatCode
  rate: number
  totalInclCents: number
  vatCents: number
}

const vatRows = computed<VatRow[]>(() => {
  const map = new Map<VatCode, number>()
  for (const line of props.lines) {
    const total = lineNetCents(line)
    map.set(line.vat_code, (map.get(line.vat_code) ?? 0) + total)
  }
  return (['A', 'B', 'C', 'D'] as VatCode[])
    .filter((code) => map.has(code))
    .map((code) => {
      const totalIncl = map.get(code)!
      const rate = VAT_RATES[code]
      const vatCents =
        rate > 0 ? totalIncl - Math.round((totalIncl * 10000) / (10000 + rate * 100)) : 0
      return { code, rate, totalInclCents: totalIncl, vatCents }
    })
})
</script>

<template>
  <!-- The VAT included in the total, per rate, on one line. -->
  <div v-if="vatRows.length > 0" class="vat-summary">
    {{ t('vat') }}
    <template v-for="(row, i) in vatRows" :key="row.code">
      <span v-if="i > 0"> · </span>{{ row.rate }}% {{ fmt(row.vatCents) }}
    </template>
  </div>
</template>

<style scoped>
.vat-summary {
  font-family: var(--font-mono);
  font-size: 14px;
  color: var(--text-secondary);
  text-align: right;
  font-variant-numeric: tabular-nums;
}
</style>
