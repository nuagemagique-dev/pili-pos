import { describe, expect, it } from 'vite-plus/test'
import {
  phoneDisplay,
  phoneKey,
  phoneValue,
  TAKEAWAY_PHONE_MAX_DIGITS,
  takeawayTitle,
} from './takeaway'

const tag = (customerName: string | null) => ({ label: 'E-61', customerName })

describe('takeawayTitle', () => {
  it('is the badge alone without a tag', () => {
    expect(takeawayTitle('À emporter', null)).toBe('À emporter')
  })
  it('adds the number of the day', () => {
    expect(takeawayTitle('À emporter', tag(null))).toBe('À emporter E-61')
    expect(takeawayTitle('À emporter', tag('   '))).toBe('À emporter E-61')
  })
  it('adds the customer name after a dot', () => {
    expect(takeawayTitle('À emporter', tag('Dupont'))).toBe('À emporter E-61 · Dupont')
    expect(takeawayTitle('Takeaway', tag(' Martin '))).toBe('Takeaway E-61 · Martin')
  })
})

describe('the phone on the numeric pad', () => {
  it('appends digits, backspaces and clears', () => {
    let d = ''
    for (const k of '0470') d = phoneKey(d, k)
    expect(d).toBe('0470')
    expect(phoneKey(d, 'BACKSPACE')).toBe('047')
    expect(phoneKey(d, 'CLEAR')).toBe('')
    expect(phoneKey('', 'BACKSPACE')).toBe('')
  })
  it('ignores a key that is not a digit', () => {
    expect(phoneKey('04', '+')).toBe('04')
    expect(phoneKey('04', 'AB')).toBe('04')
  })
  it('stops at the most digits', () => {
    let d = ''
    for (let i = 0; i < 30; i++) d = phoneKey(d, '7')
    expect(d).toHaveLength(TAKEAWAY_PHONE_MAX_DIGITS)
  })
  it('groups the digits for the screen and the engine', () => {
    expect(phoneDisplay('0470')).toBe('0470')
    expect(phoneDisplay('0470123456')).toBe('0470 12 34 56')
    expect(phoneDisplay('04701234567')).toBe('0470 12 34 56 7')
    expect(phoneValue('')).toBeNull()
    expect(phoneValue('0470123456')).toBe('0470 12 34 56')
  })
  it('never makes a phone the engine refuses', () => {
    // 24 characters at most: 15 digits become 4 + 5 groups of 2 + 1, with spaces.
    const full = '1'.repeat(TAKEAWAY_PHONE_MAX_DIGITS)
    expect((phoneValue(full) ?? '').length).toBeLessThanOrEqual(24)
  })
})
