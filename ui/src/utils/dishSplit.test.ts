import { describe, expect, it } from 'vite-plus/test'
import {
  allLeftPicked,
  cashDueCents,
  dishLines,
  paidUnits,
  pickedCents,
  pickedCount,
  pickedUnits,
  toggleLine,
  unitShares,
  unitsFit,
  unitsLeft,
  withUnits,
  type DishLine,
  type DishPick,
  type DishUnits,
} from './dishSplit'

const line = (id: number, quantity: number, netCents: number): DishLine => ({
  id,
  quantity,
  netCents,
})
const none = new Map<number, number>()

describe('unitShares', () => {
  it('puts the remainder cents on the first units and keeps the sum', () => {
    expect(unitShares(1000, 3)).toEqual([334, 333, 333])
    expect(unitShares(1001, 3)).toEqual([334, 334, 333])
    expect(unitShares(1002, 3).reduce((a, b) => a + b, 0)).toBe(1002)
  })

  it('gives a line of one unit its whole net', () => {
    expect(unitShares(1152, 1)).toEqual([1152])
  })

  it('gives a free line free units', () => {
    expect(unitShares(0, 4)).toEqual([0, 0, 0, 0])
  })

  it('keeps the sum for a negative net too', () => {
    expect(unitShares(-100, 3).reduce((a, b) => a + b, 0)).toBe(-100)
  })

  it('has no unit for a line of no quantity', () => {
    expect(unitShares(500, 0)).toEqual([])
  })
})

describe('dishLines', () => {
  it('reads the net after the line discounts', () => {
    const cart = [
      { id: 1, quantity: 2, unit_price_cents: 500, discount_cents: -100 },
      { id: 2, quantity: 1, unit_price_cents: 300, discount_cents: -300 },
    ]
    expect(dishLines(cart)).toEqual([line(1, 2, 900), line(2, 1, 0)])
  })
})

describe('picking units', () => {
  const lines = [line(1, 3, 1000), line(2, 1, 460), line(3, 2, 0)]

  it('adds up the units checked, line by line', () => {
    // One unit of the line of 3 (334) and the single dish.
    expect(pickedCents(lines, none, { 1: 1, 2: 1 })).toBe(334 + 460)
    expect(pickedCents(lines, none, { 1: 3 })).toBe(1000)
  })

  it('the units paid already are the next ones not taken again', () => {
    // A first part took 1 unit (334): the next unit of the line costs 333.
    const paid = paidUnits([{ units: [{ lineId: 1, count: 1 }] }])
    expect(unitsLeft(lines[0], paid)).toBe(2)
    expect(pickedCents(lines, paid, { 1: 1 })).toBe(333)
    expect(pickedCents(lines, paid, { 1: 2 })).toBe(666)
  })

  it('every part together pays the order total to the cent', () => {
    const total = lines.reduce((s, l) => s + l.netCents, 0)
    const taken: { units: DishUnits[] }[] = []
    let sum = 0
    const picks: DishPick[] = [{ 1: 1 }, { 1: 1 }, { 1: 1, 2: 1, 3: 2 }]
    for (const pick of picks) {
      sum += pickedCents(lines, paidUnits(taken), pick)
      taken.push({ units: pickedUnits(lines, pick) })
    }
    expect(sum).toBe(total)
  })

  it('clamps a check to the units left and removes an empty one', () => {
    expect(withUnits({}, { id: 1, left: 1 }, 3)).toEqual({ 1: 1 })
    expect(withUnits({ 1: 1 }, { id: 1, left: 1 }, 0)).toEqual({})
    expect(withUnits({}, { id: 1, left: 3 }, -2)).toEqual({})
  })

  it('a tap checks every unit left, a second tap clears them', () => {
    const once = toggleLine({}, { id: 1, left: 3 })
    expect(once).toEqual({ 1: 3 })
    expect(toggleLine(once, { id: 1, left: 3 })).toEqual({})
    // A partial check is completed first.
    expect(toggleLine({ 1: 1 }, { id: 1, left: 3 })).toEqual({ 1: 3 })
  })

  it('knows when every unit left is checked', () => {
    expect(allLeftPicked(lines, none, { 1: 3, 2: 1, 3: 2 })).toBe(true)
    expect(allLeftPicked(lines, none, { 1: 3, 2: 1 })).toBe(false)
    const paid = paidUnits([{ units: [{ lineId: 1, count: 3 }] }])
    expect(allLeftPicked(lines, paid, { 2: 1, 3: 2 })).toBe(true)
  })

  it('lists the units checked in cart order and counts them', () => {
    expect(pickedUnits(lines, { 2: 1, 1: 2 })).toEqual([
      { lineId: 1, count: 2 },
      { lineId: 2, count: 1 },
    ])
    expect(pickedCount({ 1: 2, 2: 1 })).toBe(3)
  })
})

describe('unitsFit', () => {
  const lines = [line(1, 3, 1000)]
  it('accepts units the cart still has', () => {
    expect(
      unitsFit(lines, [{ units: [{ lineId: 1, count: 2 }] }, { units: [{ lineId: 1, count: 1 }] }]),
    ).toBe(true)
    expect(unitsFit(lines, [{}])).toBe(true)
  })
  it('refuses a line that left or shrank', () => {
    expect(unitsFit(lines, [{ units: [{ lineId: 9, count: 1 }] }])).toBe(false)
    expect(
      unitsFit(lines, [{ units: [{ lineId: 1, count: 2 }] }, { units: [{ lineId: 1, count: 2 }] }]),
    ).toBe(false)
  })
})

describe('cashDueCents (several cash parts, one rounded cash line)', () => {
  it('is the part rounded to 5 cents when no cash was taken before', () => {
    expect(cashDueCents(0, 1486)).toBe(1485)
    expect(cashDueCents(0, 496)).toBe(495)
    expect(cashDueCents(0, 1003)).toBe(1005)
  })

  it('shares the rounding of the whole cash between the parts', () => {
    // Three friends pay 10,03 each in cash: the one cash line is 30,09, rounded 30,10.
    const parts = [1003, 1003, 1003]
    let taken = 0
    const dues = parts.map((p) => {
      const due = cashDueCents(taken, p)
      taken += p
      return due
    })
    expect(dues).toEqual([1005, 1000, 1005])
    expect(dues.reduce((a, b) => a + b, 0)).toBe(3010)
  })

  it('always adds up to the rounded total, whatever the parts', () => {
    for (const parts of [
      [1486],
      [496, 495, 495],
      [333, 333, 334],
      [101, 102, 103, 104],
      [1, 1, 1, 1, 1],
    ]) {
      let taken = 0
      let dueSum = 0
      for (const p of parts) {
        dueSum += cashDueCents(taken, p)
        taken += p
      }
      expect(dueSum).toBe(Math.round(taken / 5) * 5)
      // The rounding of the single cash line stays within 2 cents.
      expect(Math.abs(dueSum - taken)).toBeLessThanOrEqual(2)
    }
  })
})
