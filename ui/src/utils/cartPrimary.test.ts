import { describe, expect, it } from 'vite-plus/test'
import { kitchenIsPrimary, sendableLineCount, type SendableLine } from './cartPrimary'

function line(change: Partial<SendableLine> = {}): SendableLine {
  return {
    unsent_quantity: 0,
    kitchen_state: 'printed',
    bowl_line_id: null,
    ...change,
  }
}

describe('sendableLineCount', () => {
  it('counts the lines with something unsent', () => {
    const lines = [
      line({ unsent_quantity: 2, kitchen_state: 'unsent' }),
      line({ unsent_quantity: 1, kitchen_state: 'unsent' }),
      line(),
    ]
    expect(sendableLineCount(lines, true)).toBe(2)
  })

  it('leaves out the lines a course holds back', () => {
    const lines = [
      line({ unsent_quantity: 1, kitchen_state: 'unsent' }),
      line({ unsent_quantity: 3, kitchen_state: 'held' }),
    ]
    expect(sendableLineCount(lines, true)).toBe(1)
  })

  it('does not count a bowl option as a line of its own', () => {
    const lines = [
      line({ unsent_quantity: 1, kitchen_state: 'unsent' }),
      line({ unsent_quantity: 1, kitchen_state: 'unsent', bowl_line_id: 1 }),
    ]
    expect(sendableLineCount(lines, true)).toBe(1)
  })

  it('is zero for a takeaway, whose lines go out with the sale', () => {
    const lines = [line({ unsent_quantity: 2, kitchen_state: 'unsent' })]
    expect(sendableLineCount(lines, false)).toBe(0)
  })
})

describe('kitchenIsPrimary', () => {
  it('is the primary while lines wait to be sent', () => {
    expect(kitchenIsPrimary({ dineIn: true, canTakePayment: true, sendableLines: 2 })).toBe(true)
  })

  it('gives the primary back to PAYER once everything is sent', () => {
    expect(kitchenIsPrimary({ dineIn: true, canTakePayment: true, sendableLines: 0 })).toBe(false)
  })

  it('is always the primary for a role that cannot pay', () => {
    expect(kitchenIsPrimary({ dineIn: true, canTakePayment: false, sendableLines: 0 })).toBe(true)
  })

  it('never applies to a takeaway', () => {
    expect(kitchenIsPrimary({ dineIn: false, canTakePayment: false, sendableLines: 0 })).toBe(false)
  })
})
