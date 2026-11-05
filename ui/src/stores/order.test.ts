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
}))
vi.mock('../api/ipc', () => ipcMock)

import { useOrderStore } from './order'

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
