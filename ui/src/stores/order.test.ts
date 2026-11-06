import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test'
import type { CartLine, MenuItem } from '../types'

// The store talks to the engine through these commands only.
const ipcMock = vi.hoisted(() => ({
  addOrderLine: vi.fn(),
  updateOrderLineQuantity: vi.fn(),
  getOrder: vi.fn(),
  listCartLines: vi.fn(),
  listHeldCourses: vi.fn(),
  getCourseSettings: vi.fn(),
  setActiveOrder: vi.fn(),
  getQuickNotes: vi.fn(),
  transferTable: vi.fn(),
  getTakeawayTag: vi.fn(),
  getOrderCovers: vi.fn(),
  setOrderCovers: vi.fn(),
  getTablePlan: vi.fn(),
}))
vi.mock('../api/ipc', () => ipcMock)

import { MOVE_UNDO_TTL_MS, useOrderStore } from './order'
import { useToastStore } from './toast'
import { useTablePlanStore } from './tablePlan'

const ORDER = { id: 1, order_type: 'dine_in', table_number: 4, status: 'open', total_cents: 0 }

function item(id: number): MenuItem {
  return {
    id,
    category_id: 1,
    name: `P${id}`,
    code: `A${id}`,
    price_cents: 500,
    vat_code: 'B',
    vat_code_takeaway: null,
    open_price: false,
    kitchen_printer: true,
    sort_order: id,
    active: true,
  }
}

function cartLine(id: number, itemId: number, units: [quantity: number, unsent: number]): CartLine {
  const [quantity, unsent] = units
  return {
    id,
    order_id: 1,
    menu_item_id: itemId,
    quantity,
    unit_price_cents: 500,
    vat_code: 'B',
    name: `P${itemId}`,
    modifiers_json: null,
    created_at: '2026-11-05T10:00:00',
    department_id: '1',
    department_name: 'Sushi',
    name_fr: null,
    name_zh: null,
    unsent_quantity: unsent,
    discount_cents: 0,
    price_changes: [],
    kitchen_note: null,
    online: false,
    course: 0,
    kitchen_state: 'unsent',
  } as CartLine
}

describe('order store: adds and the digit window units (C7, C9)', () => {
  beforeEach(() => {
    // The refusal text reads the terminal language from localStorage.
    vi.stubGlobal('localStorage', {
      getItem: () => null,
      setItem: () => undefined,
    })
    // The toast store runs a timer on window.
    vi.useFakeTimers()
    vi.stubGlobal('window', globalThis)
    setActivePinia(createPinia())
    vi.resetAllMocks()
    ipcMock.getOrder.mockResolvedValue(ORDER)
    ipcMock.getCourseSettings.mockResolvedValue({ enabled: false, category_courses: [] })
    ipcMock.updateOrderLineQuantity.mockResolvedValue(undefined)
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  async function started() {
    const order = useOrderStore()
    ipcMock.listCartLines.mockResolvedValue([])
    await order.loadOrder(1)
    return order
  }

  it('answers the line a new add went to and notes it for the beep and the flash', async () => {
    const order = await started()
    ipcMock.addOrderLine.mockResolvedValue({ ...cartLine(7, 1, [2, 2]) })

    const add = await order.addItemTracked(item(1), 2)

    expect(add).toMatchObject({ lineId: 7, itemId: 1, units: 2 })
    expect(order.lastAdd).toEqual(add)
    expect(order.lines.map((l) => l.id)).toEqual([7])
  })

  it('a second add of the same product merges and notes the merged line', async () => {
    const order = await started()
    ipcMock.addOrderLine.mockResolvedValue(cartLine(7, 1, [1, 1]))
    const first = await order.addItemTracked(item(1), 1)

    const second = await order.addItemTracked(item(1), 2)

    expect(ipcMock.addOrderLine).toHaveBeenCalledTimes(1)
    expect(ipcMock.updateOrderLineQuantity).toHaveBeenCalledWith(7, 3)
    expect(second).toMatchObject({ lineId: 7, units: 2 })
    expect(second?.seq).toBeGreaterThan(first?.seq ?? 0)
  })

  it('a refused add notes nothing and answers null', async () => {
    const order = await started()
    ipcMock.addOrderLine.mockRejectedValue('order_not_open')

    expect(await order.addItemTracked(item(1))).toBeNull()
    expect(order.lastAdd).toBeNull()
  })

  it('the digit window adds extra units to the same line through the quantity command', async () => {
    const order = await started()
    ipcMock.addOrderLine.mockResolvedValue(cartLine(7, 1, [1, 1]))
    const add = await order.addItemTracked(item(1), 1)

    expect(await order.addUnits(add?.lineId ?? 0, 2)).toBe(true)

    expect(ipcMock.updateOrderLineQuantity).toHaveBeenCalledWith(7, 3)
    expect(order.lines[0].quantity).toBe(3)
    expect(order.lastAdd).toMatchObject({ lineId: 7, units: 2 })
  })

  it('never overwrites units a merged line already had: only the delta is added', async () => {
    const order = await started()
    ipcMock.addOrderLine.mockResolvedValue(cartLine(7, 1, [4, 4]))
    await order.addItemTracked(item(1), 4)
    // The tap merged one more unit into the line of 4.
    const add = await order.addItemTracked(item(1), 1)
    expect(order.lines[0].quantity).toBe(5)

    await order.addUnits(add?.lineId ?? 0, 2)

    expect(ipcMock.updateOrderLineQuantity).toHaveBeenLastCalledWith(7, 7)
  })

  it('never goes under the units already sent to the kitchen', async () => {
    const order = await started()
    // 5 units on the line, 2 of them already sent: the quantity cannot go under 3.
    ipcMock.addOrderLine.mockResolvedValue(cartLine(7, 1, [5, 5]))
    await order.addItemTracked(item(1), 5)
    order.lines[0].unsent_quantity = 3

    await order.addUnits(7, -10)

    expect(ipcMock.updateOrderLineQuantity).toHaveBeenLastCalledWith(7, 3)
  })

  it('a zero delta calls no command but still confirms with a beep and a flash', async () => {
    const order = await started()
    ipcMock.addOrderLine.mockResolvedValue(cartLine(7, 1, [1, 1]))
    const add = await order.addItemTracked(item(1), 1)

    expect(await order.addUnits(7, 0)).toBe(true)

    expect(ipcMock.updateOrderLineQuantity).not.toHaveBeenCalled()
    expect(order.lastAdd?.seq).toBeGreaterThan(add?.seq ?? 0)
  })

  it('does nothing for a line that is gone', async () => {
    const order = await started()

    expect(await order.addUnits(99, 2)).toBe(false)

    expect(ipcMock.updateOrderLineQuantity).not.toHaveBeenCalled()
  })
})

describe('order store: quick notes for the kitchen note dialog (C11)', () => {
  beforeEach(() => {
    vi.stubGlobal('localStorage', { getItem: () => null, setItem: () => undefined })
    vi.useFakeTimers()
    vi.stubGlobal('window', globalThis)
    setActivePinia(createPinia())
    vi.resetAllMocks()
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  it("reads the installation's list from the engine each time", async () => {
    const order = useOrderStore()
    ipcMock.getQuickNotes.mockResolvedValueOnce({ notes: ['Sans wasabi', 'Bien cuit'] })
    ipcMock.getQuickNotes.mockResolvedValueOnce({ notes: ['Épicé'] })

    expect(await order.loadQuickNotes()).toEqual(['Sans wasabi', 'Bien cuit'])
    expect(await order.loadQuickNotes()).toEqual(['Épicé'])
    expect(ipcMock.getQuickNotes).toHaveBeenCalledTimes(2)
  })

  it('answers no chip when the engine does not answer: the note can still be typed', async () => {
    const order = useOrderStore()
    ipcMock.getQuickNotes.mockRejectedValue('internal')

    expect(await order.loadQuickNotes()).toEqual([])
  })

  it('keeps an empty list empty', async () => {
    const order = useOrderStore()
    ipcMock.getQuickNotes.mockResolvedValue({ notes: [] })

    expect(await order.loadQuickNotes()).toEqual([])
  })
})

describe('order store: moving a table in one tap (C8)', () => {
  const MOVED = {
    orderId: 1,
    fromTable: 4,
    toTable: 9,
    kitchenSlip: 'printed',
    transferSlip: 'printed',
  }

  beforeEach(() => {
    vi.stubGlobal('localStorage', { getItem: () => null, setItem: () => undefined })
    vi.useFakeTimers()
    vi.stubGlobal('window', globalThis)
    setActivePinia(createPinia())
    vi.resetAllMocks()
    ipcMock.getCourseSettings.mockResolvedValue({ enabled: false, category_courses: [] })
    ipcMock.listCartLines.mockResolvedValue([cartLine(7, 1, [1, 0])])
    ipcMock.transferTable.mockResolvedValue(MOVED)
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  async function onTable4() {
    const order = useOrderStore()
    ipcMock.getOrder.mockResolvedValue(ORDER)
    await order.loadOrder(1)
    return order
  }

  it('moves the order at once, reads it again and toasts "Table 4 déplacée en 9" with Annuler for 5 s', async () => {
    const order = await onTable4()
    ipcMock.getOrder.mockResolvedValue({ ...ORDER, table_number: 9 })

    await order.moveTable(1, 4, 9)

    expect(ipcMock.transferTable).toHaveBeenCalledWith(1, 9)
    expect(order.currentOrder?.table_number).toBe(9)
    const toast = useToastStore()
    expect(toast.toasts.map((t) => t.message)).toEqual(['Table 4 déplacée en 9'])
    expect(toast.toasts[0].action?.label).toBe('Annuler')
    expect(MOVE_UNDO_TTL_MS).toBe(5000)
    vi.advanceTimersByTime(4700)
    expect(toast.toasts).toHaveLength(1)
    vi.advanceTimersByTime(600)
    expect(toast.toasts).toHaveLength(0)
  })

  it('Annuler moves it back with a second transfer and says so', async () => {
    const order = await onTable4()
    ipcMock.getOrder.mockResolvedValue({ ...ORDER, table_number: 9 })
    await order.moveTable(1, 4, 9)
    ipcMock.getOrder.mockResolvedValue(ORDER)
    const toast = useToastStore()

    toast.runAction(toast.toasts[0].id)
    await vi.advanceTimersByTimeAsync(0)

    expect(ipcMock.transferTable).toHaveBeenCalledTimes(2)
    expect(ipcMock.transferTable).toHaveBeenLastCalledWith(1, 4)
    expect(order.currentOrder?.table_number).toBe(4)
    expect(toast.toasts.map((t) => t.message)).toEqual(['Table 9 déplacée en 4'])
    expect(toast.toasts[0].action).toBeUndefined()
  })

  it('throws a refusal for the caller and leaves no toast', async () => {
    const order = await onTable4()
    ipcMock.transferTable.mockRejectedValue('table_transfer_target_busy')

    await expect(order.moveTable(1, 4, 9)).rejects.toBe('table_transfer_target_busy')

    expect(useToastStore().toasts).toHaveLength(0)
    expect(order.tableMoves).toBe(0)
  })

  it('shows the refusal of an undo as an error toast', async () => {
    const order = await onTable4()
    await order.moveTable(1, 4, 9)
    ipcMock.transferTable.mockRejectedValue('table_transfer_target_busy')
    const toast = useToastStore()

    toast.runAction(toast.toasts[0].id)
    await vi.advanceTimersByTimeAsync(0)

    expect(toast.toasts.map((t) => t.variant)).toEqual(['error'])
  })

  it('counts each move and undo for the Tables screen', async () => {
    const order = await onTable4()
    await order.moveTable(1, 4, 9)
    expect(order.tableMoves).toBe(1)
    const toast = useToastStore()
    toast.runAction(toast.toasts[0].id)
    await vi.advanceTimersByTimeAsync(0)
    expect(order.tableMoves).toBe(2)
  })

  it('canTransfer needs an open dine-in order with a table and a line', async () => {
    const order = await onTable4()
    expect(order.canTransfer).toBe(true)
    order.lines = []
    expect(order.canTransfer).toBe(false)
  })
})

describe('order store: online collect and the parts kept (O2, P4)', () => {
  const ONLINE = {
    id: 9,
    order_type: 'takeaway',
    table_number: null,
    status: 'open',
    total_cents: 2850,
  }

  beforeEach(() => {
    vi.stubGlobal('localStorage', { getItem: () => null, setItem: () => undefined })
    vi.useFakeTimers()
    vi.stubGlobal('window', globalThis)
    setActivePinia(createPinia())
    vi.resetAllMocks()
    ipcMock.getCourseSettings.mockResolvedValue({ enabled: false, category_courses: [] })
    ipcMock.listCartLines.mockResolvedValue([cartLine(7, 1, [1, 0])])
    ipcMock.setActiveOrder.mockResolvedValue(undefined)
    ipcMock.getOrder.mockResolvedValue(ONLINE)
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  it('shows an online order at the checkout without making it the engine active order', async () => {
    const order = useOrderStore()
    await order.beginOnlineCollect(9, 'card')

    expect(order.currentOrder?.id).toBe(9)
    expect(order.lines).toHaveLength(1)
    expect(order.onlineCollect).toEqual({ method: 'card' })
    expect(ipcMock.setActiveOrder).not.toHaveBeenCalled()
  })

  it('leaving it registers nothing with the engine and clears the order', async () => {
    const order = useOrderStore()
    await order.beginOnlineCollect(9, 'cash')
    ipcMock.setActiveOrder.mockClear()

    await order.leave('navigate')

    expect(order.currentOrder).toBeNull()
    expect(order.onlineCollect).toBeNull()
    expect(ipcMock.setActiveOrder).toHaveBeenCalledWith(null)
  })

  it('keeps the manual parts of an order, for that order only', () => {
    const order = useOrderStore()
    const part = { method: 'card' as const, cents: 500, changeCents: 0 }
    order.holdParts(1, [part])

    expect(order.heldPartsFor(1)).toEqual([part])
    expect(order.heldPartsFor(2)).toEqual([])
  })

  it('holding no part forgets the record', () => {
    const order = useOrderStore()
    order.holdParts(1, [{ method: 'cash', cents: 500, changeCents: 0 }])
    order.holdParts(1, [])

    expect(order.heldParts).toBeNull()
  })

  it('keeps the dishes a terminal card part paid, even with no manual part', () => {
    const order = useOrderStore()
    order.holdParts(1, [], { 7: [{ lineId: 3, count: 2 }] })

    expect(order.heldEftUnitsFor(1)).toEqual({ 7: [{ lineId: 3, count: 2 }] })
    expect(order.heldEftUnitsFor(2)).toEqual({})
    expect(order.heldPartsFor(1)).toEqual([])
    order.holdParts(1, [])
    expect(order.heldParts).toBeNull()
  })

  it('forgets the parts when the order is cleared or the till locks', () => {
    const order = useOrderStore()
    order.holdParts(1, [{ method: 'cash', cents: 500, changeCents: 0 }])
    order.resetLocal()

    expect(order.heldPartsFor(1)).toEqual([])
  })
})

describe('order store: covers and zones (T5, T1)', () => {
  beforeEach(() => {
    vi.stubGlobal('localStorage', { getItem: () => null, setItem: () => undefined })
    vi.useFakeTimers()
    vi.stubGlobal('window', globalThis)
    setActivePinia(createPinia())
    vi.resetAllMocks()
    ipcMock.getCourseSettings.mockResolvedValue({ enabled: false, category_courses: [] })
    ipcMock.listCartLines.mockResolvedValue([cartLine(7, 1, [1, 0])])
    ipcMock.setActiveOrder.mockResolvedValue(undefined)
    ipcMock.getOrder.mockResolvedValue(ORDER)
    ipcMock.getOrderCovers.mockResolvedValue(null)
    ipcMock.transferTable.mockResolvedValue({ orderId: 1, fromTable: 4, toTable: 9 })
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  it('reads the covers of a table order when it is loaded', async () => {
    ipcMock.getOrderCovers.mockResolvedValue(4)
    const order = useOrderStore()
    await order.loadOrder(1)
    expect(ipcMock.getOrderCovers).toHaveBeenCalledWith(1)
    expect(order.covers).toBe(4)
  })

  it('a takeaway has no covers and asks nothing', async () => {
    ipcMock.getOrder.mockResolvedValue({ ...ORDER, order_type: 'takeaway', table_number: null })
    ipcMock.getTakeawayTag.mockResolvedValue(null)
    const order = useOrderStore()
    await order.loadOrder(1)
    expect(ipcMock.getOrderCovers).not.toHaveBeenCalled()
    expect(order.covers).toBeNull()
  })

  it('sets and clears the covers through the engine', async () => {
    const order = useOrderStore()
    await order.loadOrder(1)
    ipcMock.setOrderCovers.mockResolvedValue(6)
    expect(await order.setCovers(6)).toBe(true)
    expect(ipcMock.setOrderCovers).toHaveBeenCalledWith(1, 6)
    expect(order.covers).toBe(6)
    ipcMock.setOrderCovers.mockResolvedValue(null)
    await order.setCovers(null)
    expect(ipcMock.setOrderCovers).toHaveBeenLastCalledWith(1, null)
    expect(order.covers).toBeNull()
  })

  it('a refusal is a toast and the covers keep their value', async () => {
    ipcMock.getOrderCovers.mockResolvedValue(3)
    const order = useOrderStore()
    await order.loadOrder(1)
    ipcMock.setOrderCovers.mockRejectedValue('order_not_open')
    expect(await order.setCovers(5)).toBe(false)
    expect(order.covers).toBe(3)
    expect(useToastStore().toasts.map((t) => t.variant)).toEqual(['error'])
  })

  it('forgets the covers with the local order', async () => {
    ipcMock.getOrderCovers.mockResolvedValue(3)
    const order = useOrderStore()
    await order.loadOrder(1)
    order.resetLocal()
    expect(order.covers).toBeNull()
  })

  it('the move toast and its undo name the tables with their zone', async () => {
    const plan = useTablePlanStore()
    plan.apply({
      plan: 'A 1-5, B 6-9',
      table_count: 9,
      zone_names: '',
      zones: [
        { letter: 'A', name: null, numbers: [4] },
        { letter: 'B', name: null, numbers: [9] },
      ],
      log: [],
    })
    const order = useOrderStore()
    await order.loadOrder(1)
    ipcMock.getOrder.mockResolvedValue({ ...ORDER, table_number: 9 })
    await order.moveTable(1, 4, 9)
    const toast = useToastStore()
    expect(toast.toasts.map((t) => t.message)).toEqual(['Table A-4 déplacée en B-9'])
    ipcMock.getOrder.mockResolvedValue(ORDER)
    toast.runAction(toast.toasts[0].id)
    await vi.advanceTimersByTimeAsync(0)
    expect(toast.toasts.map((t) => t.message)).toEqual(['Table B-9 déplacée en A-4'])
  })
})
