<script setup lang="ts">
import { computed } from "vue";
import type { OrderLine, VatCode } from "../types";
import { formatPrice as fmt } from "../utils/format";

const props = defineProps<{
  lines: OrderLine[];
}>();

const VAT_RATES: Record<VatCode, number> = { A: 21, B: 12, C: 6, D: 0, X: 0 };

interface VatRow {
  code: VatCode;
  rate: number;
  totalInclCents: number;
  vatCents: number;
}

const vatRows = computed<VatRow[]>(() => {
  const map = new Map<VatCode, number>();
  for (const line of props.lines) {
    const total = line.unit_price_cents * line.quantity;
    map.set(line.vat_code, (map.get(line.vat_code) ?? 0) + total);
  }
  return (["A", "B", "C", "D"] as VatCode[])
    .filter((code) => map.has(code))
    .map((code) => {
      const totalIncl = map.get(code)!;
      const rate = VAT_RATES[code];
      const vatCents = rate > 0 ? totalIncl - Math.round((totalIncl * 10000) / (10000 + rate * 100)) : 0;
      return { code, rate, totalInclCents: totalIncl, vatCents };
    });
});
</script>

<template>
  <div v-if="vatRows.length > 0" class="vat-summary">
    <div class="vat-row vat-header">
      <span>TVA</span><span>Taux</span><span>Base</span><span>TVA</span>
    </div>
    <div v-for="row in vatRows" :key="row.code" class="vat-row">
      <span class="vat-code">{{ row.code }}</span>
      <span>{{ row.rate }}%</span>
      <span>{{ fmt(row.totalInclCents - row.vatCents) }}</span>
      <span>{{ fmt(row.vatCents) }}</span>
    </div>
  </div>
</template>

<style scoped>
.vat-summary {
  padding: var(--space-sm) var(--space-md);
  border-top: 1px solid rgba(160, 160, 176, 0.1);
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
}

.vat-row {
  display: grid;
  grid-template-columns: 36px 48px 1fr 1fr;
  gap: var(--space-xs);
  padding: 2px 0;
}

.vat-header {
  font-weight: var(--font-weight-bold);
  border-bottom: 1px solid rgba(160, 160, 176, 0.1);
  padding-bottom: var(--space-xs);
  margin-bottom: var(--space-xs);
}

.vat-code { font-weight: var(--font-weight-bold); }
</style>
