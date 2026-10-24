// Amounts of the cart with the till discounts taken off. The engine
// owns every rule (the spread, the VAT); the screen only adds up what
// the cart lines report.
import type { CartLine } from '../types'

/** Quantity times unit price, before any discount. */
export function lineGrossCents(line: Pick<CartLine, 'unit_price_cents' | 'quantity'>): number {
  return line.unit_price_cents * line.quantity
}

/** A line's amount after its price changes (`discount_cents` is negative). */
export function lineNetCents(
  line: Pick<CartLine, 'unit_price_cents' | 'quantity'> & { discount_cents?: number },
): number {
  return lineGrossCents(line) + (line.discount_cents ?? 0)
}

/** Σ of the lines' net amounts: the amount due. */
export function netTotalCents(
  lines: readonly (Pick<CartLine, 'unit_price_cents' | 'quantity'> & { discount_cents?: number })[],
): number {
  return lines.reduce((sum, line) => sum + lineNetCents(line), 0)
}

/** Σ of the discounts of the lines (zero or negative). */
export function discountTotalCents(lines: readonly { discount_cents?: number }[]): number {
  return lines.reduce((sum, line) => sum + (line.discount_cents ?? 0), 0)
}

/** Whether an order discount (scope EVENT) applies to some line. */
export function hasOrderDiscount(lines: readonly Pick<CartLine, 'price_changes'>[]): boolean {
  return lines.some((l) => l.price_changes?.some((c) => c.scope === 'EVENT'))
}

/** What a discount takes off `netCents`, as the engine computes it:
 * a percent rounded half up, an amount, or everything. */
export function discountPreviewCents(
  kind: 'percent' | 'amount' | 'free',
  value: number,
  netCents: number,
): number {
  if (kind === 'free') return netCents
  if (kind === 'percent') return Math.floor((netCents * value + 50) / 100)
  return value
}
