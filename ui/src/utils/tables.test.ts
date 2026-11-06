import { describe, expect, it } from 'vite-plus/test'
import type { TableInfo } from '../api/ipc'
import { groupTables, hasZones, inZone, tableLabel, zoneTitle } from './tables'

function table(n: number, zone: string | null, zoneName: string | null = null): TableInfo {
  return {
    tableNumber: n,
    zone,
    zoneName,
    label: tableLabel(zone, n),
    openedByInitial: null,
    covers: null,
    orderId: null,
    totalCents: 0,
    lineCount: 0,
    status: 'free',
    openedAt: null,
    preBillPrinted: false,
    heldCourse: null,
  }
}

describe('table labels (T1)', () => {
  it('writes A-19 in a zone and 19 without one', () => {
    expect(tableLabel('A', 19)).toBe('A-19')
    expect(tableLabel('TE', 3)).toBe('TE-3')
    expect(tableLabel(null, 19)).toBe('19')
    expect(tableLabel(undefined, 19)).toBe('19')
    expect(tableLabel('', 19)).toBe('19')
  })

  it('knows what a zone is', () => {
    expect(inZone('A')).toBe(true)
    expect(inZone(null)).toBe(false)
    expect(inZone('')).toBe(false)
  })

  it('titles a zone with its name when it has one', () => {
    expect(zoneTitle('A', 'Salle')).toBe('A · Salle')
    expect(zoneTitle('B', null)).toBe('B')
  })
})

describe('grouping the tables by zone (T1)', () => {
  it('a plan without letters is one group with no zone and no header', () => {
    const tables = [table(1, null), table(2, null)]
    expect(hasZones(tables)).toBe(false)
    const groups = groupTables(tables)
    expect(groups).toHaveLength(1)
    expect(groups[0]).toMatchObject({ zone: null, name: null })
    expect(groups[0].tables.map((t) => t.tableNumber)).toEqual([1, 2])
  })

  it('groups by zone in alphabetical order and keeps the order of the plan inside a zone', () => {
    const tables = [
      table(21, 'B', 'Terrasse'),
      table(1, 'A', 'Salle'),
      table(22, 'B', 'Terrasse'),
      table(2, 'A', 'Salle'),
    ]
    expect(hasZones(tables)).toBe(true)
    const groups = groupTables(tables)
    expect(groups.map((g) => g.zone)).toEqual(['A', 'B'])
    expect(groups[0].name).toBe('Salle')
    expect(groups[0].tables.map((t) => t.tableNumber)).toEqual([1, 2])
    expect(groups[1].tables.map((t) => t.tableNumber)).toEqual([21, 22])
  })

  it('puts a table the plan left out (no zone) last', () => {
    const groups = groupTables([table(9, null), table(1, 'A'), table(21, 'B')])
    expect(groups.map((g) => g.zone)).toEqual(['A', 'B', null])
    expect(groups[2].tables[0].tableNumber).toBe(9)
  })

  it('a zone without a name has none', () => {
    expect(groupTables([table(1, 'A')])[0].name).toBeNull()
  })
})
