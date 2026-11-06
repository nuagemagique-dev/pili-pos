import { describe, expect, it } from 'vite-plus/test'
import { reconcileHeld, type HeldPart } from './heldParts'

const card = (cents: number): HeldPart => ({ method: 'card', cents, changeCents: 0 })
const cash = (cents: number, changeCents = 0): HeldPart => ({ method: 'cash', cents, changeCents })

describe('reconcileHeld (P4)', () => {
  it('keeps parts that fit in what is left, in order', () => {
    const kept = reconcileHeld([card(500), cash(700, 300)], 1500)
    expect(kept).toEqual({ parts: [card(500), cash(700, 300)], dropped: false })
  })

  it('accepts parts that pay the whole rest exactly', () => {
    expect(reconcileHeld([card(500), card(1000)], 1500).dropped).toBe(false)
  })

  it('drops every manual part when one exceeds the rest after the order shrank', () => {
    expect(reconcileHeld([card(500), card(1000)], 1200)).toEqual({ parts: [], dropped: true })
  })

  it('drops them when the rest fell to nothing', () => {
    expect(reconcileHeld([card(100)], 0)).toEqual({ parts: [], dropped: true })
  })

  it('says nothing when there was nothing kept', () => {
    expect(reconcileHeld([], 0)).toEqual({ parts: [], dropped: false })
  })

  it('answers a copy, not the kept list itself', () => {
    const kept: HeldPart[] = [card(500)]
    expect(reconcileHeld(kept, 900).parts).not.toBe(kept)
  })
})
