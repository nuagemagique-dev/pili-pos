import { describe, expect, it } from 'vite-plus/test'
import { hintAfter, minutesKept, triesWithoutPin } from './pinGrant'

describe('the manager PIN hint', () => {
  it('is none when the engine kept no grant', () => {
    expect(hintAfter(7, null, 1000)).toBeNull()
    expect(hintAfter(7, undefined, 1000)).toBeNull()
    expect(hintAfter(7, 0, 1000)).toBeNull()
  })

  it('runs from the answer for the time the engine said', () => {
    expect(hintAfter(7, 120_000, 1000)).toEqual({ orderId: 7, expiresAt: 121_000 })
  })

  it('tries without the PIN only for the same order before the end', () => {
    const hint = hintAfter(7, 120_000, 0)
    expect(triesWithoutPin(hint, 7, 0)).toBe(true)
    expect(triesWithoutPin(hint, 7, 119_999)).toBe(true)
    expect(triesWithoutPin(hint, 7, 120_000)).toBe(false)
    expect(triesWithoutPin(hint, 8, 1000)).toBe(false)
    expect(triesWithoutPin(hint, undefined, 1000)).toBe(false)
    expect(triesWithoutPin(null, 7, 1000)).toBe(false)
  })

  it('says whole minutes, at least one', () => {
    expect(minutesKept(120_000)).toBe(2)
    expect(minutesKept(119_000)).toBe(2)
    expect(minutesKept(60_000)).toBe(1)
    expect(minutesKept(500)).toBe(1)
  })
})
