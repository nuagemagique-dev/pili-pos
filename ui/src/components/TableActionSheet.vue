<script setup lang="ts">
// The action sheet of an occupied table card (UX audit T7), opened by a long press: the four
// things a server does from the plan without going through the cart. A short tap on the card
// still opens the table. "Addition" and "Payer" are for the roles that may take payments.
import type { TableInfo } from '../api/ipc'
import { useI18n } from '../i18n'
import { tableLabel } from '../utils/tables'
import { formatPrice } from '../utils/format'

defineProps<{
  table: TableInfo
  /** The role may print the bill and take the payment (waiter-cashier, manager). */
  canTakePayment: boolean
}>()

const emit = defineEmits<{
  open: []
  bill: []
  move: []
  pay: []
  close: []
}>()

const { t } = useI18n()
</script>

<template>
  <div class="backdrop" @click.self="emit('close')">
    <div
      class="sheet"
      role="dialog"
      :aria-label="`${t('table')} ${tableLabel(table.zone, table.tableNumber)}`"
      data-testid="table-sheet"
    >
      <h2 class="title">
        {{ t('table') }} {{ tableLabel(table.zone, table.tableNumber) }}
        <span class="total">{{ formatPrice(table.totalCents) }} €</span>
      </h2>
      <div class="actions">
        <button class="act" data-testid="sheet-open" @click="emit('open')">
          {{ t('table_sheet_open') }}
        </button>
        <button v-if="canTakePayment" class="act" data-testid="sheet-bill" @click="emit('bill')">
          {{ t('pre_bill') }}
        </button>
        <button class="act" data-testid="sheet-move" @click="emit('move')">
          {{ t('transfer_mode') }}
        </button>
        <button v-if="canTakePayment" class="act" data-testid="sheet-pay" @click="emit('pay')">
          {{ t('table_sheet_pay') }}
        </button>
      </div>
      <button class="close" data-testid="sheet-close" @click="emit('close')">
        {{ t('table_sheet_close') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  z-index: 50;
}
.sheet {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  width: 520px;
  max-width: 92vw;
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.total {
  font-family: var(--font-mono);
  color: var(--text-secondary);
}
.actions {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-sm);
}
.act {
  height: 80px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.close {
  height: 56px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.act:active,
.close:active {
  transform: scale(var(--tap-scale));
}
</style>
