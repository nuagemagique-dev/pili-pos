<script setup lang="ts">
// The free tables of the plan, to move an order to one of them.
// Tapping a table selects it; "Valider" asks for the move.
import { ref } from 'vue'
import type { TableInfo } from '../api/ipc'
import { useI18n } from '../i18n'

defineProps<{
  /** The tables the order can move to (free ones, the current one left out). */
  tables: TableInfo[]
  /** The table the order is on now. */
  from: number
}>()

const emit = defineEmits<{
  confirm: [tableNumber: number]
  cancel: []
}>()

const { t } = useI18n()
const selected = ref<number | null>(null)
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div class="dialog" role="dialog" :aria-label="t('transfer_table')" data-testid="table-picker">
      <h2 class="title">{{ t('transfer_table') }} · {{ t('table') }} {{ from }}</h2>
      <p class="hint">{{ tables.length ? t('transfer_pick_target') : t('transfer_no_free') }}</p>
      <div class="grid scroll-touch">
        <button
          v-for="tbl in tables"
          :key="tbl.tableNumber"
          class="pick"
          :class="{ selected: selected === tbl.tableNumber }"
          :data-testid="`pick-table-${tbl.tableNumber}`"
          @click="selected = tbl.tableNumber"
        >
          {{ tbl.tableNumber }}
        </button>
      </div>
      <div class="actions">
        <button class="btn btn-cancel" @click="emit('cancel')">{{ t('back') }}</button>
        <button
          class="btn btn-confirm"
          :disabled="selected === null"
          data-testid="transfer-confirm"
          @click="selected !== null && emit('confirm', selected)"
        >
          {{ t('validate') }}
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
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 28px;
  font-weight: var(--font-weight-bold);
  border: 2px solid transparent;
  transition: transform var(--tap-duration) ease;
}
.pick:active {
  transform: scale(var(--tap-scale));
}
.pick.selected {
  border-color: var(--accent-primary);
  background: var(--accent-primary);
  color: var(--text-on-accent);
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
.btn-confirm {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.btn-confirm:disabled {
  opacity: 0.4;
}
</style>
