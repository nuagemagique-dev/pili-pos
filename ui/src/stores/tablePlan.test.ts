import { createPinia, setActivePinia } from 'pinia'
import type { TableInfo } from '../api/ipc'
import { beforeEach, describe, expect, it, vi } from 'vite-plus/test'

const ipcMock = vi.hoisted(() => ({ getTablePlan: vi.fn() }))
vi.mock('../api/ipc', () => ipcMock)

import { useTablePlanStore } from './tablePlan'

const ZONED = {
  plan: 'A 1-3, B 4-5',
  table_count: 5,
  zone_names: '',
  zones: [
    { letter: 'A', name: null, numbers: [1, 2, 3] },
    { letter: 'B', name: null, numbers: [4, 5] },
  ],
  log: [],
}

const base: TableInfo = {
  tableNumber: 1,
  zone: null,
  zoneName: null,
  label: '1',
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

describe('table plan store (T1)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.resetAllMocks()
  })

  it('writes plain numbers until it knows the zones', () => {
    const plan = useTablePlanStore()
    expect(plan.label(19)).toBe('19')
    expect(plan.label(null)).toBe('')
  })

  it('reads the zones from the engine and labels each table', async () => {
    ipcMock.getTablePlan.mockResolvedValue(ZONED)
    const plan = useTablePlanStore()
    await plan.load()
    expect(plan.label(2)).toBe('A-2')
    expect(plan.label(5)).toBe('B-5')
    // A table left out of the plan has no zone.
    expect(plan.label(9)).toBe('9')
    expect(plan.shortLabel(2)).toBe('A-2')
    expect(plan.shortLabel(9)).toBe('T9')
  })

  it('keeps plain numbers when the engine does not answer', async () => {
    ipcMock.getTablePlan.mockRejectedValue('not_logged_in')
    const plan = useTablePlanStore()
    await plan.load()
    expect(plan.label(2)).toBe('2')
  })

  it('learns the zones again from a plan the manager just saved', () => {
    const plan = useTablePlanStore()
    plan.apply(ZONED)
    expect(plan.label(4)).toBe('B-4')
    plan.apply({ ...ZONED, zones: [] })
    expect(plan.label(4)).toBe('4')
  })

  it('learns from a list of tables, zone or not', () => {
    const plan = useTablePlanStore()
    plan.apply(ZONED)
    plan.learn([
      { ...base, tableNumber: 1, zone: null },
      { ...base, tableNumber: 7, zone: 'C' },
    ])
    expect(plan.label(1)).toBe('1')
    expect(plan.label(7)).toBe('C-7')
    expect(plan.label(2)).toBe('A-2')
  })
})
