import type { CheckoutResult } from '../types'

/** How long the done screen stays before it returns to the tables by itself. */
export const DONE_AUTO_RETURN_MS = 2000

/**
 * Whether the done screen has something the operator must decide or
 * read, so it must not leave by itself: the paper ticket was not
 * printed, the digital ticket waits in the outbox, the FDM was not
 * connected (unsigned ticket), or a gift voucher slip has a note.
 * Anything else (OK, the change) is read in two seconds and the screen
 * returns to the tables.
 */
export function doneNeedsAttention(result: CheckoutResult | null): boolean {
  if (!result) return false
  if (!result.fdm_connected) return true
  if (result.delivery?.needsPaper === true) return true
  if (result.delivery?.email === 'queued') return true
  return (result.gift_voucher_prints ?? []).length > 0
}
