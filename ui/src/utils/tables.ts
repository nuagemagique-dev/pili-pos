// How the tables are written and grouped on screen (UX audit T1). A plan can name zones with
// letters ("A 1-20, B 21-40"): a table in a zone reads "A-19", one outside any zone "19". The
// table is still its number: the engine signs and prints on the fiscal ticket the number alone.
// Android: `ui/state/TableLabels.kt`.

import type { TableInfo } from '../api/ipc'

/** Whether `zone` names a zone (a table outside every zone has null). */
export function inZone(zone: string | null | undefined): zone is string {
  return typeof zone === 'string' && zone !== ''
}

/** "A-19" for a table in a zone, "19" for a table without one. */
export function tableLabel(zone: string | null | undefined, number: number): string {
  return inZone(zone) ? `${zone}-${number}` : String(number)
}

/** The tables of one zone, in the order the plan gives them. */
export interface TableGroup {
  /** The zone letters, null for the tables outside every zone. */
  zone: string | null
  /** The name a manager gave the zone, null when it has none. */
  name: string | null
  tables: TableInfo[]
}

/** "A · Salle", or "A" for a zone without a name. */
export function zoneTitle(zone: string, name: string | null): string {
  return name !== null && name !== '' ? `${zone} · ${name}` : zone
}

/**
 * The tables grouped by zone, zones in alphabetical order. A plan without letters is one group
 * with no zone (the screen then draws no header). The tables outside every zone (an open order on a
 * table the plan left out) come last.
 */
export function groupTables(tables: TableInfo[]): TableGroup[] {
  const byZone = new Map<string | null, TableGroup>()
  for (const table of tables) {
    const key = inZone(table.zone) ? table.zone : null
    let group = byZone.get(key)
    if (!group) {
      group = { zone: key, name: inZone(key) ? (table.zoneName ?? null) : null, tables: [] }
      byZone.set(key, group)
    }
    group.tables.push(table)
  }
  const zoned = [...byZone.values()]
    .filter((g) => g.zone !== null)
    .sort((a, b) => (a.zone ?? '').localeCompare(b.zone ?? ''))
  const plain = byZone.get(null)
  return plain ? [...zoned, plain] : zoned
}

/** Whether the plan has zones: the screens then show the headers. */
export function hasZones(tables: TableInfo[]): boolean {
  return tables.some((t) => inZone(t.zone))
}
