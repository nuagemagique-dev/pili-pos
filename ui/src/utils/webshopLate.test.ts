import { describe, expect, it } from 'vite-plus/test'
import type { WebshopState } from '../api/ipc'
import {
  LATE_AFTER_MINUTES,
  LATE_CHIME_EVERY_MS,
  isLateOrder,
  lateChimeDue,
  lateCount,
  waitingMinutes,
} from './webshopLate'

// A fake clock: "now" is a number the tests move by hand.
const NOW = Date.parse('2026-11-06T12:00:00Z')
const ago = (minutes: number) => new Date(NOW - minutes * 60_000).toISOString()

function state(fiscal: string): WebshopState {
  return {
    order_id: 1,
    hubrise_order_id: 'abc',
    channel: 'deliveroo',
    channel_kind: 'platform',
    service_type: 'delivery',
    payment_mode: 'unpaid',
    payment_type: null,
    fiscal_state: fiscal,
    resolution: 'ok',
    attention: [],
    warnings: [],
  }
}

function order(
  over: { created_at?: string; hubrise_status?: string | null; fiscal?: string } = {},
) {
  return {
    created_at: over.created_at ?? ago(0),
    hubrise_status: over.hubrise_status === undefined ? 'new' : over.hubrise_status,
    webshop: over.fiscal === undefined ? null : state(over.fiscal),
  }
}

describe('waitingMinutes', () => {
  it('counts whole minutes since the order arrived', () => {
    expect(waitingMinutes({ created_at: ago(7) }, NOW)).toBe(7)
    expect(waitingMinutes({ created_at: ago(0) }, NOW)).toBe(0)
  })

  it('reads the zoneless UTC stamp of the database', () => {
    expect(waitingMinutes({ created_at: '2026-11-06 11:55:00' }, NOW)).toBe(5)
  })

  it('never goes negative and gives up on an unreadable date', () => {
    expect(waitingMinutes({ created_at: ago(-3) }, NOW)).toBe(0)
    expect(waitingMinutes({ created_at: 'not a date' }, NOW)).toBeNull()
  })
})

describe('isLateOrder (O3)', () => {
  it('is late from 5 minutes of waiting, not before', () => {
    expect(isLateOrder(order({ created_at: ago(LATE_AFTER_MINUTES - 1) }), NOW)).toBe(false)
    expect(isLateOrder(order({ created_at: ago(LATE_AFTER_MINUTES) }), NOW)).toBe(true)
  })

  it('counts a received order but not one that was answered', () => {
    const old = ago(30)
    expect(isLateOrder(order({ created_at: old, hubrise_status: 'received' }), NOW)).toBe(true)
    expect(isLateOrder(order({ created_at: old, hubrise_status: 'accepted' }), NOW)).toBe(false)
    expect(isLateOrder(order({ created_at: old, hubrise_status: 'rejected' }), NOW)).toBe(false)
  })

  it('does not count an order already signed or refused', () => {
    const old = ago(30)
    expect(isLateOrder(order({ created_at: old, fiscal: 'signed_p' }), NOW)).toBe(false)
    expect(isLateOrder(order({ created_at: old, fiscal: 'pending' }), NOW)).toBe(true)
  })

  it('reads a missing status as new', () => {
    expect(isLateOrder(order({ created_at: ago(9), hubrise_status: null }), NOW)).toBe(true)
  })
})

describe('lateCount', () => {
  it('counts the late orders only', () => {
    const orders = [
      order({ created_at: ago(9) }),
      order({ created_at: ago(1) }),
      order({ created_at: ago(6) }),
    ]
    expect(lateCount(orders, NOW)).toBe(2)
    expect(lateCount([], NOW)).toBe(0)
  })
})

describe('lateChimeDue: at once, then every 2 minutes while any order is late', () => {
  it('is silent while no order is late', () => {
    expect(lateChimeDue(0, null, NOW)).toBe(false)
    expect(lateChimeDue(0, NOW - 10 * LATE_CHIME_EVERY_MS, NOW)).toBe(false)
  })

  it('plays at once when an order turns late', () => {
    expect(lateChimeDue(1, null, NOW)).toBe(true)
  })

  it('waits 2 minutes before the next one', () => {
    expect(lateChimeDue(2, NOW - LATE_CHIME_EVERY_MS + 1, NOW)).toBe(false)
    expect(lateChimeDue(2, NOW - LATE_CHIME_EVERY_MS, NOW)).toBe(true)
  })

  it('repeats on a moving clock: 0, 2, 4 minutes', () => {
    let last: number | null = null
    const played: number[] = []
    for (let minute = 0; minute <= 5; minute++) {
      const now = NOW + minute * 60_000
      if (lateChimeDue(1, last, now)) {
        last = now
        played.push(minute)
      }
    }
    expect(played).toEqual([0, 2, 4])
  })
})
