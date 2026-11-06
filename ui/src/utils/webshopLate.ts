import type { WebshopOrderSummary } from '../api/ipc'
import { parseTimestamp } from './datetime'

// Online orders nobody answered (UX audit O3): platforms cancel an order left unanswered, so
// from 5 minutes of waiting the whole till says so, on every staff screen, and the chime
// comes back every 2 minutes while any order is late.

/** An unanswered order is late from this many minutes of waiting. */
export const LATE_AFTER_MINUTES = 5

/** While any order is late, the chime repeats this often (milliseconds). */
export const LATE_CHIME_EVERY_MS = 2 * 60_000

/** The fields of an order the rule reads. */
export type LateCandidate = Pick<WebshopOrderSummary, 'created_at' | 'hubrise_status' | 'webshop'>

/** Whole minutes an order has been waiting at `nowMs`, or null when its date is unreadable. */
export function waitingMinutes(order: Pick<WebshopOrderSummary, 'created_at'>, nowMs: number) {
  const created = parseTimestamp(order.created_at)
  if (!created) return null
  return Math.max(0, Math.floor((nowMs - created.getTime()) / 60_000))
}

/** An order nobody answered yet (not accepted, not refused) that has waited 5 minutes or more. */
export function isLateOrder(order: LateCandidate, nowMs: number): boolean {
  if ((order.webshop?.fiscal_state ?? 'pending') !== 'pending') return false
  const status = order.hubrise_status ?? 'new'
  if (status !== 'new' && status !== 'received') return false
  return (waitingMinutes(order, nowMs) ?? 0) >= LATE_AFTER_MINUTES
}

/** How many of `orders` are late at `nowMs`. */
export function lateCount(orders: readonly LateCandidate[], nowMs: number): number {
  return orders.filter((o) => isLateOrder(o, nowMs)).length
}

/**
 * Whether the late chime plays now: at once when an order turns late (`lastChimeAt` null), then
 * again every 2 minutes while some order is still late.
 */
export function lateChimeDue(count: number, lastChimeAt: number | null, nowMs: number): boolean {
  if (count <= 0) return false
  return lastChimeAt === null || nowMs - lastChimeAt >= LATE_CHIME_EVERY_MS
}
