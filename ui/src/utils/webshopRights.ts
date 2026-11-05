// Online orders: who may act, and why an order is refused (UX audit L6, O1).
// On screen "Serveur" is the internal role `cashier` and "Apprenti" is the
// role `waiter`; the internal names do not change. Display only: the
// engine checks the rights again (`Access::Cashier`) and refuses a waiter.

/** The reasons a staff member picks when refusing an online order. The
 * HubRise API has no reject reason field, so the reason stays on the till
 * (it is the confirmation's second step) and never reaches HubRise. */
export const REJECT_REASONS = ['out_of_stock', 'too_busy', 'closed', 'other'] as const

export type RejectReason = (typeof REJECT_REASONS)[number]

/** The i18n key of a reason's chip. */
export function rejectReasonKey(reason: RejectReason): string {
  return `webshop_reject_reason_${reason}`
}

/** The buttons of an order card that need the cashier rights: accept,
 * reject, handover and collect (and closing a cancelled order). */
export interface CardRightsInput {
  /** "Refuser" is offered. */
  reject: boolean
  /** The primary button exists and signs or hands over (accept, collect, complete, close). */
  cashierPrimary: boolean
  /** "Encaisser carte" is offered. */
  collectCard: boolean
}

/** True when the card shows a button the operator may not use: the buttons
 * are then shown disabled with the line "Réservé au serveur ou au gérant". */
export function cashierNoteNeeded(card: CardRightsInput, canTakePayment: boolean): boolean {
  if (canTakePayment) return false
  return card.reject || card.cashierPrimary || card.collectCard
}
