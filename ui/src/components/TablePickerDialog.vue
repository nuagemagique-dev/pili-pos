<script setup lang="ts">
// The table picker sheet (UX audit C8): every other table of the plan, to
// move the open order. A free table is one tap and the move happens at
// once (no "Valider"); a busy one is dimmed, disabled and shows its total.
import type { TableInfo } from '../api/ipc'
import { useI18n } from '../i18n'
import { useTablePlanStore } from '../stores/tablePlan'
import { tableLabel } from '../utils/tables'
import { formatPrice } from '../utils/format'

defineProps<{
  /** The tables of the plan except the one the order is on. */
  tables: TableInfo[]
  /** The table the order is on now. */
  from: number
}>()

const emit = defineEmits<{
  pick: [tableNumber: number]
  cancel: []
}>()

const { t } = useI18n()
const plan = useTablePlanStore()
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div class="dialog" role="dialog" :aria-label="t('transfer_table')" data-testid="table-picker">
      <h2 class="title">{{ t('transfer_table') }} · {{ t('table') }} {{ plan.label(from) }}</h2>
      <p class="hint">
        {{
          tables.some((tbl) => tbl.status === 'free')
            ? t('transfer_pick_target')
            : t('transfer_no_free')
        }}
      </p>
      <div class="grid scroll-touch">
        <button
          v-for="tbl in tables"
          :key="tbl.tableNumber"
          class="pick"
          :class="{ busy: tbl.status !== 'free' }"
          :disabled="tbl.status !== 'free'"
          :data-testid="`pick-table-${tbl.tableNumber}`"
          @click="emit('pick', tbl.tableNumber)"
        >
          <span class="pick-num">{{ tableLabel(tbl.zone, tbl.tableNumber) }}</span>
          <span v-if="tbl.status !== 'free'" class="pick-total"
            >{{ formatPrice(tbl.totalCents) }} €</span
          >
        </button>
      </div>
      <div class="actions">
        <button class="btn btn-cancel" data-testid="transfer-cancel" @click="emit('cancel')">
          {{ t('back') }}
        </button>
      </div>
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
.dialog {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  width: 720px;
  max-height: 90vh;
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.hint {
  font-size: var(--font-size-md);
  color: var(--text-secondary);
}
.grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: var(--space-sm);
  overflow-y: auto;
  max-height: 50vh;
}
.pick {
  height: 72px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-weight: var(--font-weight-bold);
  border: 2px solid transparent;
  transition: transform var(--tap-duration) ease;
}
.pick:active:not(:disabled) {
  transform: scale(var(--tap-scale));
}
.pick-num {
  font-size: 28px;
  line-height: 1.1;
}
/* A busy table: dimmed, never tappable, its total in small type. */
.pick.busy {
  opacity: 0.4;
}
.pick-total {
  font-size: var(--font-size-xs);
  font-weight: 500;
  color: var(--text-secondary);
}
.actions {
  display: flex;
  gap: var(--space-sm);
  width: 100%;
}
.btn {
  flex: 1;
  height: 56px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  transition: transform var(--tap-duration) ease;
}
.btn:active {
  transform: scale(var(--tap-scale));
}
.btn-cancel {
  background: var(--bg-surface);
  color: var(--text-secondary);
}
</style>
