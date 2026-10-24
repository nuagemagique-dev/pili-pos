import { describe, expect, it } from 'vite-plus/test'
import { errorCode, errorText, parseErrorCode } from './errorMessages'

describe('errorText', () => {
  it('keeps the string Tauri rejects with', () => {
    expect(errorText('pin_invalid|left=2')).toBe('pin_invalid|left=2')
  })

  it('reads the message of an Error a store throws', () => {
    expect(errorText(new Error('discount_refused'))).toBe('discount_refused')
  })

  it('is empty for nothing', () => {
    expect(errorText(null)).toBe('')
    expect(errorText(undefined)).toBe('')
  })

  it('writes numbers, booleans and objects out', () => {
    expect(errorText(42)).toBe('42')
    expect(errorText(false)).toBe('false')
    expect(errorText({ code: 'x' })).toBe('{"code":"x"}')
  })
})

describe('errorCode', () => {
  it('finds the code in a rejected string and in a thrown Error alike', () => {
    expect(errorCode('table_busy|table=4')).toBe('table_busy')
    expect(errorCode(new Error('table_busy|table=4'))).toBe('table_busy')
  })

  it('is null for a text that is not a code', () => {
    expect(errorCode(new Error('No active order'))).toBeNull()
    expect(errorCode('FDM_LOCKED')).toBeNull()
    expect(errorCode(null)).toBeNull()
  })
})

describe('parseErrorCode', () => {
  it('splits the values back', () => {
    expect(parseErrorCode('pin_invalid|left=2|until=12:00')).toEqual({
      code: 'pin_invalid',
      vars: { left: '2', until: '12:00' },
    })
  })
})
