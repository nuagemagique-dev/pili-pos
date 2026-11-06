// Payment parts taken by hand and not signed yet (UX audit P4). They stay attached to the order
// when the operator goes back to it from the checkout, and come back on return. A part the
// terminal approved is the engine's (it is read again from there), so it is never kept here:
// only the cart units it paid in a split by dishes are (`eftUnits`), so its dishes stay greyed.

import type { DishUnits } from './dishSplit'

/** The methods a part can be taken with. */
export type HeldMethod = 'cash' | 'card' | 'meal_voucher' | 'gift_voucher'

/** One part taken by hand: what it pays and, for cash, the change handed back. */
export interface HeldPart {
  method: HeldMethod
  cents: number
  /** Change handed back for a cash part, in cents. */
  changeCents: number
  /** Cash handed over for a cash part, in cents (shown in the recap with the change). */
  givenCents?: number
  /** A gift voucher part: the voucher it redeems, and its masked code. */
  giftVoucherId?: number
  codeMasked?: string
  /** The cart units this part pays when the sale is split by dishes (UX audit P3). */
  units?: DishUnits[]
}

/** The parts kept for one order. */
export interface HeldParts {
  orderId: number
  parts: HeldPart[]
  /** The cart units paid by each approved terminal payment, by `eftPaymentId` (split by dishes). */
  eftUnits: Record<number, DishUnits[]>
}

/**
 * The parts to show again once the checkout reopens. `restCents` is what the order total leaves
 * after the card parts the terminal approved. Each manual part must fit in what is left; as soon
 * as one does not (the order changed meanwhile), every manual part is dropped and `dropped` says
 * so, for the toast.
 */
export function reconcileHeld(
  parts: readonly HeldPart[],
  restCents: number,
): { parts: HeldPart[]; dropped: boolean } {
  let left = restCents
  for (const part of parts) {
    if (part.cents > left) return { parts: [], dropped: parts.length > 0 }
    left -= part.cents
  }
  return { parts: [...parts], dropped: false }
}
