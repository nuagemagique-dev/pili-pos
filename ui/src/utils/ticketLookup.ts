// The tickets tab: how many tickets it loads, and the kind of a ticket found by its number.

import type { TicketMatch } from '../api/ipc'

/** The first page of the copy list, and of the refund list (the latest sales). */
export const COPY_FIRST_PAGE = 30
export const REFUND_FIRST_PAGE = 20
/** "Plus ancien" loads this many more tickets. */
export const OLDER_PAGE = 20

/** The limits of the two lists once `older` pages were asked for. */
export function ticketLimits(older: number): { copy: number; refund: number } {
  const extra = Math.max(0, older) * OLDER_PAGE
  return { copy: COPY_FIRST_PAGE + extra, refund: REFUND_FIRST_PAGE + extra }
}

/** True when a list came back full: the archive may hold older tickets. */
export function hasOlderTickets(
  limits: { copy: number; refund: number },
  copyCount: number,
  refundCount: number,
): boolean {
  return copyCount >= limits.copy || refundCount >= limits.refund
}

/** The i18n key of the kind of a found ticket (the same words as the copy picker). */
export function matchKindKey(
  match: Pick<TicketMatch, 'eventLabel' | 'isRefund'>,
): 'copy_kind_training' | 'copy_kind_pre_bill' | 'copy_kind_refund' | 'copy_kind_sale' {
  if (match.eventLabel === 'T') return 'copy_kind_training'
  if (match.eventLabel === 'P') return 'copy_kind_pre_bill'
  return match.isRefund ? 'copy_kind_refund' : 'copy_kind_sale'
}

/** The number typed on the pad, or null when it is not a ticket number (1 or more, whole). */
export function ticketNumberOf(typed: string): number | null {
  if (!/^\d{1,9}$/u.test(typed)) return null
  const n = Number(typed)
  return n >= 1 ? n : null
}
