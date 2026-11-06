// Which button of the cart footer is the Volt primary (UX audit C4).
// While a dine-in order has lines the kitchen has not received, "Envoyer
// en cuisine" is the primary and PAYER is a secondary button; once
// everything is sent PAYER is the primary again. Display only: nothing
// here reaches the engine.
import type { CartLine } from '../types'

/** The fields of a cart line this rule reads. */
export type SendableLine = Pick<CartLine, 'unsent_quantity' | 'kitchen_state' | 'bowl_line_id'>

/** The cart lines the kitchen still has to receive: unsent quantity and
 * not held back by a course. A bowl's paid option follows its bowl and is
 * not a line of its own. */
export function sendableLineCount(lines: readonly SendableLine[], dineIn: boolean): number {
  if (!dineIn) return 0
  return lines.filter(
    (line) =>
      (line.unsent_quantity ?? 0) > 0 &&
      line.kitchen_state !== 'held' &&
      (line.bowl_line_id ?? null) === null,
  ).length
}

/** "Envoyer en cuisine" is the primary: a dine-in order with lines to
 * send, or a role that cannot take payment (Apprenti). */
export function kitchenIsPrimary(args: {
  dineIn: boolean
  canTakePayment: boolean
  sendableLines: number
}): boolean {
  return args.dineIn && (!args.canTakePayment || args.sendableLines > 0)
}
