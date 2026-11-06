import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '../api/ipc'
import { inZone, tableLabel } from '../utils/tables'

/**
 * The zones of the table plan (UX audit T1), for the screens that only know a table number: the
 * order header, the cart title, the checkout chip, the table picker, the toasts. A table in a zone
 * reads "A-19", one outside any zone "19". The engine owns the plan: this reads it at login and
 * after a manager changed it, and learns it again from every list of tables. Android:
 * `ui/state/OrderStore.kt` (`tableLabel`).
 */
export const useTablePlanStore = defineStore('tablePlan', () => {
  // Table number to zone letters. Empty on a plan without letters.
  const zoneByTable = ref<Record<number, string>>({})

  /** How table `number` is written on screen. */
  function label(number: number | null | undefined): string {
    if (number === null || number === undefined) return ''
    return tableLabel(zoneByTable.value[number], number)
  }

  /** The compact form of the ticket lists: "A-19" in a zone, "T19" without one. */
  function shortLabel(number: number): string {
    return zoneByTable.value[number] ? label(number) : `T${number}`
  }

  /** Read the plan from the engine; a till that does not answer keeps plain numbers. */
  async function load() {
    try {
      apply(await ipc.getTablePlan())
    } catch {
      /* plain numbers until the next read */
    }
  }

  /** Learn the zones from a plan the engine just returned. */
  function apply(plan: ipc.TablePlan) {
    const next: Record<number, string> = {}
    for (const zone of plan.zones) {
      for (const number of zone.numbers) next[number] = zone.letter
    }
    zoneByTable.value = next
  }

  /** Learn the zones from a list of tables (each one says its own zone). */
  function learn(tables: ipc.TableInfo[]) {
    const next = { ...zoneByTable.value }
    for (const table of tables) {
      if (inZone(table.zone)) next[table.tableNumber] = table.zone
      else delete next[table.tableNumber]
    }
    zoneByTable.value = next
  }

  return { zoneByTable, label, shortLabel, load, apply, learn }
})
