import { describe, expect, it } from 'vite-plus/test'
import type { CheckoutResult, DeliveryOutcome } from '../types'
import { DONE_AUTO_RETURN_MS, doneNeedsAttention } from './checkoutDone'

const DELIVERY: DeliveryOutcome = {
  fiscalEventId: 1,
  deliveryId: null,
  medium: 'PAPER',
  email: 'not_requested',
  recipientHint: null,
  paperPrinted: true,
  needsPaper: false,
}

function sale(change: Partial<CheckoutResult> = {}): CheckoutResult {
  return {
    success: true,
    ticket: null,
    error: null,
    fdm_connected: true,
    fiscal_event_id: 1,
    delivery: DELIVERY,
    kitchen_slip: 'printed',
    gift_voucher_prints: [],
    ...change,
  }
}

describe('doneNeedsAttention', () => {
  it('lets a plain printed sale return by itself', () => {
    expect(doneNeedsAttention(sale())).toBe(false)
  })

  it('lets a sale without a delivery record return by itself', () => {
    expect(doneNeedsAttention(sale({ delivery: null }))).toBe(false)
  })

  it('lets a digital ticket that was sent return by itself', () => {
    const delivery = { ...DELIVERY, medium: 'DIGITAL' as const, email: 'sent' as const }
    expect(doneNeedsAttention(sale({ delivery }))).toBe(false)
  })

  it('stays when the paper ticket could not be printed', () => {
    const delivery = { ...DELIVERY, paperPrinted: false, needsPaper: true }
    expect(doneNeedsAttention(sale({ delivery }))).toBe(true)
  })

  it('stays when the digital ticket waits in the outbox', () => {
    const delivery = { ...DELIVERY, email: 'queued' as const }
    expect(doneNeedsAttention(sale({ delivery }))).toBe(true)
  })

  it('stays when the FDM was not connected', () => {
    expect(doneNeedsAttention(sale({ fdm_connected: false }))).toBe(true)
  })

  it('stays when a gift voucher slip has a note', () => {
    const prints = [{ voucher_id: 3, kind: 'voucher' as const, outcome: 'failed' as const }]
    expect(doneNeedsAttention(sale({ gift_voucher_prints: prints }))).toBe(true)
  })

  it('has nothing to decide before a sale exists', () => {
    expect(doneNeedsAttention(null)).toBe(false)
  })

  it('returns after two seconds', () => {
    expect(DONE_AUTO_RETURN_MS).toBe(2000)
  })
})
