import { describe, expect, it } from 'vite-plus/test'
import {
  COPY_FIRST_PAGE,
  OLDER_PAGE,
  REFUND_FIRST_PAGE,
  hasOlderTickets,
  matchKindKey,
  ticketLimits,
  ticketNumberOf,
} from './ticketLookup'

describe('ticketLimits', () => {
  it('starts with the latest sales and grows by 20 a page', () => {
    expect(ticketLimits(0)).toEqual({ copy: COPY_FIRST_PAGE, refund: REFUND_FIRST_PAGE })
    expect(ticketLimits(1)).toEqual({
      copy: COPY_FIRST_PAGE + OLDER_PAGE,
      refund: REFUND_FIRST_PAGE + OLDER_PAGE,
    })
    expect(ticketLimits(3).refund).toBe(80)
  })
  it('never goes below the first page', () => {
    expect(ticketLimits(-2)).toEqual(ticketLimits(0))
  })
})

describe('hasOlderTickets', () => {
  const limits = ticketLimits(0)
  it('offers older tickets while a list came back full', () => {
    expect(hasOlderTickets(limits, 30, 3)).toBe(true)
    expect(hasOlderTickets(limits, 4, 20)).toBe(true)
  })
  it('stops when both lists ran out', () => {
    expect(hasOlderTickets(limits, 29, 19)).toBe(false)
    expect(hasOlderTickets(limits, 0, 0)).toBe(false)
  })
})

describe('matchKindKey', () => {
  it('names a training ticket, a pre-bill, a refund and a sale', () => {
    expect(matchKindKey({ eventLabel: 'T', isRefund: false })).toBe('copy_kind_training')
    expect(matchKindKey({ eventLabel: 'P', isRefund: false })).toBe('copy_kind_pre_bill')
    expect(matchKindKey({ eventLabel: 'N', isRefund: true })).toBe('copy_kind_refund')
    expect(matchKindKey({ eventLabel: 'N', isRefund: false })).toBe('copy_kind_sale')
  })
})

describe('ticketNumberOf', () => {
  it('reads a whole number from 1 up', () => {
    expect(ticketNumberOf('1')).toBe(1)
    expect(ticketNumberOf('4821')).toBe(4821)
    expect(ticketNumberOf('000012')).toBe(12)
  })
  it('refuses anything else', () => {
    for (const typed of ['', '0', '00', '-3', '1.5', '12a', ' 7', '1234567890']) {
      expect(ticketNumberOf(typed)).toBeNull()
    }
  })
})
