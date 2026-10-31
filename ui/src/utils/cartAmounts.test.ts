import { describe, expect, it } from 'vite-plus/test'
import { lineGrossCents, lineNetCents, netTotalCents } from './cartAmounts'

describe('amounts of a weighed line', () => {
  const weighed = { unit_price_cents: 4500, quantity: 1, weighed_amount_cents: 1251 }

  it('takes the amount of the scale, not the price per kilo', () => {
    expect(lineGrossCents(weighed)).toBe(1251)
  })

  it('takes its discount off that amount', () => {
    expect(lineNetCents({ ...weighed, discount_cents: -251 })).toBe(1000)
  })

  it('adds up with the lines sold by the piece', () => {
    expect(netTotalCents([weighed, { unit_price_cents: 300, quantity: 2 }])).toBe(1851)
  })

  it('a correction of a weighing is negative', () => {
    expect(lineGrossCents({ ...weighed, quantity: -1 })).toBe(-1251)
  })
})
