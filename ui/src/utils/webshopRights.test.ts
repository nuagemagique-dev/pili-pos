import { describe, expect, it } from 'vite-plus/test'
import {
  REJECT_REASONS,
  cashierNoteNeeded,
  rejectReasonKey,
  type CardRightsInput,
} from './webshopRights'

const none: CardRightsInput = { reject: false, cashierPrimary: false, collectCard: false }

describe('cashierNoteNeeded', () => {
  it('shows no note to a role that takes payment', () => {
    expect(cashierNoteNeeded({ reject: true, cashierPrimary: true, collectCard: true }, true)).toBe(
      false,
    )
  })

  it('shows the note to an Apprenti when a card button needs the cashier rights', () => {
    expect(cashierNoteNeeded({ ...none, reject: true }, false)).toBe(true)
    expect(cashierNoteNeeded({ ...none, cashierPrimary: true }, false)).toBe(true)
    expect(cashierNoteNeeded({ ...none, collectCard: true }, false)).toBe(true)
  })

  it('shows no note when the card has only buttons an Apprenti may use', () => {
    expect(cashierNoteNeeded(none, false)).toBe(false)
  })
})

describe('reject reasons', () => {
  it('lists the four reasons in order, each with its own string key', () => {
    expect([...REJECT_REASONS]).toEqual(['out_of_stock', 'too_busy', 'closed', 'other'])
    expect(REJECT_REASONS.map(rejectReasonKey)).toEqual([
      'webshop_reject_reason_out_of_stock',
      'webshop_reject_reason_too_busy',
      'webshop_reject_reason_closed',
      'webshop_reject_reason_other',
    ])
  })
})
