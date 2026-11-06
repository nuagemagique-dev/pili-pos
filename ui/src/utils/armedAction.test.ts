import { describe, expect, it } from 'vite-plus/test'
import { ARM_MS, isArmed, tapArmed } from './armedAction'

describe('tapArmed', () => {
  it('arms on the first tap without firing', () => {
    expect(tapArmed(null, 1000)).toEqual({ fire: false, armedUntil: 1000 + ARM_MS })
  })

  it('fires on a second tap within 5 seconds and disarms', () => {
    const first = tapArmed(null, 1000)
    expect(tapArmed(first.armedUntil, 1000 + ARM_MS - 1)).toEqual({ fire: true, armedUntil: null })
  })

  it('arms again, without firing, when the second tap comes too late', () => {
    const first = tapArmed(null, 1000)
    const late = 1000 + ARM_MS
    expect(tapArmed(first.armedUntil, late)).toEqual({ fire: false, armedUntil: late + ARM_MS })
  })
})

describe('isArmed', () => {
  it('is armed until the deadline only', () => {
    expect(isArmed(null, 0)).toBe(false)
    expect(isArmed(5000, 4999)).toBe(true)
    expect(isArmed(5000, 5000)).toBe(false)
  })
})
