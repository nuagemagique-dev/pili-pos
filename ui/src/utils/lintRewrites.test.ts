// The helpers whose conditions or regular expressions were rewritten for the Vite+ lint rules
// (explicit null checks, named groups, the `u` flag): same answers as before.
import { describe, expect, it } from 'vite-plus/test'
import { badgeChar } from './badgeWedge'
import { parseTimestamp } from './datetime'
import {
  ERROR_MESSAGES,
  POS_CHECK_MESSAGES,
  fdmCode,
  pendingFdmCode,
  translateFdmError,
} from './fdmMessages'
import { presetRange } from './salesRange'
import { availabilityOf } from './soldOut'

describe('badgeChar', () => {
  it('reads digits from the row and the pad, and the hex letters A to F', () => {
    expect(badgeChar('Digit3')).toBe('3')
    expect(badgeChar('Numpad7')).toBe('7')
    expect(badgeChar('KeyA')).toBe('A')
    expect(badgeChar('KeyF')).toBe('F')
  })

  it('is null for any other key', () => {
    for (const code of ['KeyG', 'Digit', 'Digit33', 'NumpadEnter', 'Enter', 'ShiftLeft', '']) {
      expect(badgeChar(code)).toBeNull()
    }
  })
})

describe('pendingFdmCode', () => {
  it('finds the code the FDM answered with, after any other field', () => {
    expect(pendingFdmCode('FDM_ANSWER_PENDING|label=N|ticket=3|fdm=BUFFER_FULL|cause=x')).toBe(
      'BUFFER_FULL',
    )
    expect(pendingFdmCode('FDM_ANSWER_PENDING|fdm=FDM_LOCKED')).toBe('FDM_LOCKED')
  })

  it('is null without an answer or without a text', () => {
    expect(pendingFdmCode('FDM_ANSWER_PENDING|label=N|ticket=3')).toBeNull()
    expect(pendingFdmCode('')).toBeNull()
    expect(pendingFdmCode(null)).toBeNull()
  })
})

describe('translateFdmError', () => {
  it('says what the FDM answered, then that the same request goes again', () => {
    expect(translateFdmError('FDM_ANSWER_PENDING|fdm=BUFFER_FULL')).toBe(
      `${ERROR_MESSAGES.BUFFER_FULL} ${POS_CHECK_MESSAGES.FDM_RETRY_SAME_REQUEST}`,
    )
  })

  it('is null for nothing', () => {
    expect(translateFdmError(null)).toBeNull()
    expect(translateFdmError('')).toBeNull()
    expect(fdmCode('')).toBeNull()
  })
})

describe('availabilityOf', () => {
  it('tells available, sold out for the day and until re-enabled apart', () => {
    expect(availabilityOf({})).toBe('available')
    expect(availabilityOf({ sold_out: false, sold_out_until: '2026-10-25T03:00:00Z' })).toBe(
      'available',
    )
    expect(availabilityOf({ sold_out: true, sold_out_until: '2026-10-25T03:00:00Z' })).toBe('day')
    expect(availabilityOf({ sold_out: true, sold_out_until: null })).toBe('until')
    expect(availabilityOf({ sold_out: true, sold_out_until: '' })).toBe('until')
  })
})

describe('parseTimestamp', () => {
  it('takes a zoneless SQLite time as UTC and keeps an explicit zone', () => {
    expect(parseTimestamp('2026-10-24 12:00:00')?.toISOString()).toBe('2026-10-24T12:00:00.000Z')
    expect(parseTimestamp('2026-10-24T12:00:00+02:00')?.toISOString()).toBe(
      '2026-10-24T10:00:00.000Z',
    )
    expect(parseTimestamp('2026-10-24T12:00:00Z')?.toISOString()).toBe('2026-10-24T12:00:00.000Z')
    expect(parseTimestamp('not a date')).toBeNull()
  })
})

describe('presetRange', () => {
  it('covers each preset', () => {
    expect(presetRange('today', '2026-10-24')).toEqual(['2026-10-24', '2026-10-24'])
    expect(presetRange('yesterday', '2026-10-24')).toEqual(['2026-10-23', '2026-10-23'])
    expect(presetRange('week', '2026-10-24')).toEqual(['2026-10-19', '2026-10-24'])
    expect(presetRange('month', '2026-10-24')).toEqual(['2026-10-01', '2026-10-24'])
  })
})
