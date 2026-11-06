<script setup lang="ts">
// UX audit C8: the whole "move this order to another table" flow of the order
// and checkout screens, shared by the header table chip and the cart's "Plus"
// menu. It reads the plan, shows the picker sheet, and a tap on a free table
// moves the order at once; the toast with its "Annuler" is the store's.
import { onMounted, ref } from 'vue'
import * as ipc from '../api/ipc'
import { useOrderStore } from '../stores/order'
import { useToastStore } from '../stores/toast'
import { describeError } from '../utils/errorMessages'
import TablePickerDialog from './TablePickerDialog.vue'

const emit = defineEmits<{ close: [] }>()

const order = useOrderStore()
const toast = useToastStore()
const tables = ref<ipc.TableInfo[]>([])
const ready = ref(false)
const moving = ref(false)

onMounted(async () => {
  try {
    const current = order.currentOrder?.table_number
    tables.value = (await ipc.listTables()).filter((tbl) => tbl.tableNumber !== current)
    ready.value = true
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
    emit('close')
  }
})

async function pick(toTable: number) {
  const current = order.currentOrder
  if (!current || current.table_number === null || moving.value) return
  moving.value = true
  try {
    await order.moveTable(current.id, current.table_number, toTable)
  } catch (e) {
    toast.show(describeError(e), 'error', 6000)
  } finally {
    moving.value = false
    emit('close')
  }
}
</script>

<template>
  <TablePickerDialog
    v-if="ready && order.currentOrder?.table_number != null"
    :tables="tables"
    :from="order.currentOrder.table_number"
    @pick="pick"
    @cancel="emit('close')"
  />
</template>
