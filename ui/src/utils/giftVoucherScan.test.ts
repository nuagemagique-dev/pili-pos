import { describe, expect, it } from 'vite-plus/test'
import { maskVoucherCode, voucherCodeFromScan } from './giftVoucherScan'
import { claimScans, scansClaimed } from './scanClaim'

describe('voucherCodeFromScan', () => {
  it('takes exactly 12 digits', () => {
    expect(voucherCodeFromScan('123456789095')).toBe('123456789095')
    expect(voucherCodeFromScan('000000000000')).toBe('000000000000')
  })

  it('refuses another length, a letter, a space or nothing', () => {
    for (const token of ['', '12345678909', '1234567890951', '12345678909A', '1234 5678 9095'])
      expect(voucherCodeFromScan(token)).toBeNull()
  })
})

describe('maskVoucherCode', () => {
  it('shows the last 4 digits only, like the engine masks a voucher', () => {
    expect(maskVoucherCode('123456789095')).toBe('**** **** 9095')
    expect(maskVoucherCode('123456789095')).not.toContain('1234')
  })
})

describe('claimScans', () => {
  it('is claimed until every claim is given back, each one once', () => {
    expect(scansClaimed()).toBe(false)
    const a = claimScans()
    const b = claimScans()
    expect(scansClaimed()).toBe(true)
    a()
    a()
    expect(scansClaimed()).toBe(true)
    b()
    expect(scansClaimed()).toBe(false)
  })
})
