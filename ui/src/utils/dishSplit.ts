// Split by dishes (UX audit P3): the cart lines are shared between the friends unit by unit. A
// part is the sum of the units checked for it. This file only places cents the engine already
// settled (the line nets already carry every discount, offered item and offered line); the sale
// itself stays ONE sale: the units never leave the screen, the payment list sent to the engine is
// the same card / meal voucher / cash parts as for equal parts.
//
// Cents rule (same as the equal parts): the net amount N of a line of q units is shared in q unit
// shares, the remainder cents going to the first units: 1000 over 3 units is 334, 333, 333. The
// shares of a line always add up to N exactly, and so the shares of the lines add up to the
// order total: no cent is created or lost whatever the units checked. Units of a line are taken
// in order (the first unit paid first), so the same line gives the same shares again after a
// part was taken. The discounts and offered lines are already inside the line nets (the engine
// spreads an order discount over its lines), so they are shared pro rata by construction: a
// free line has a net of 0, its units cost 0, and a 10 % order discount lowers every unit.
import type { CartLine } from '../types'
import { lineNetCents } from './cartAmounts'
import { round5 } from './format'

/** One cart line as the split sees it: its id, its units and its net amount in cents. */
export interface DishLine {
  id: number
  quantity: number
  netCents: number
}

/** Units of one line given to a part. */
export interface DishUnits {
  lineId: number
  count: number
}

/** The lines of the cart as the split sees them. */
export function dishLines(
  lines: readonly Pick<
    CartLine,
    'id' | 'quantity' | 'unit_price_cents' | 'weighed_amount_cents' | 'discount_cents'
  >[],
): DishLine[] {
  return lines.map((l) => ({ id: l.id, quantity: l.quantity, netCents: lineNetCents(l) }))
}

/** The net of a line shared in its units, the remainder cents on the first units. */
export function unitShares(netCents: number, quantity: number): number[] {
  if (quantity <= 0) return []
  const base = Math.floor(netCents / quantity)
  const extra = netCents - base * quantity
  return Array.from({ length: quantity }, (_, i) => base + (i < extra ? 1 : 0))
}

/** Units already given to the parts taken, per line. */
export function paidUnits(parts: readonly { units?: readonly DishUnits[] }[]): Map<number, number> {
  const paid = new Map<number, number>()
  for (const part of parts) {
    for (const u of part.units ?? []) paid.set(u.lineId, (paid.get(u.lineId) ?? 0) + u.count)
  }
  return paid
}

/** Units of a line not given to a part yet. */
export function unitsLeft(line: DishLine, paid: ReadonlyMap<number, number>): number {
  return Math.max(0, line.quantity - (paid.get(line.id) ?? 0))
}

/** What the `count` next units of a line cost, after the units already `paid`. */
export function unitsCents(line: DishLine, paidCount: number, count: number): number {
  return unitShares(line.netCents, line.quantity)
    .slice(paidCount, paidCount + count)
    .reduce((sum, c) => sum + c, 0)
}

/** The units checked for the current part, per line id, kept to what each line has left. */
export type DishPick = Readonly<Record<number, number>>

/** The units checked, as a list (lines in cart order, nothing for a line with no unit checked). */
export function pickedUnits(lines: readonly DishLine[], pick: DishPick): DishUnits[] {
  return lines
    .filter((l) => (pick[l.id] ?? 0) > 0)
    .map((l) => ({ lineId: l.id, count: pick[l.id] }))
}

/** How many units are checked in all. */
export function pickedCount(pick: DishPick): number {
  return Object.values(pick).reduce((sum, n) => sum + n, 0)
}

/** The amount of the units checked, after the units already given to the parts taken. */
export function pickedCents(
  lines: readonly DishLine[],
  paid: ReadonlyMap<number, number>,
  pick: DishPick,
): number {
  return lines.reduce(
    (sum, l) =>
      sum + unitsCents(l, paid.get(l.id) ?? 0, Math.min(pick[l.id] ?? 0, unitsLeft(l, paid))),
    0,
  )
}

/** Every unit not given yet is checked: the part pays the whole rest. */
export function allLeftPicked(
  lines: readonly DishLine[],
  paid: ReadonlyMap<number, number>,
  pick: DishPick,
): boolean {
  return lines.every((l) => (pick[l.id] ?? 0) >= unitsLeft(l, paid))
}

/** A line with the units it has left to give, as the checks need it. */
export interface DishTarget {
  id: number
  left: number
}

/** The checks with `count` units on a line, clamped to the units it has left (0 removes the entry). */
export function withUnits(pick: DishPick, target: DishTarget, count: number): DishPick {
  const next = { ...pick }
  const clamped = Math.max(0, Math.min(count, target.left))
  if (clamped === 0) delete next[target.id]
  else next[target.id] = clamped
  return next
}

/** A tap on the line: every unit left is checked, or none when all of them already were. */
export function toggleLine(pick: DishPick, target: DishTarget): DishPick {
  return withUnits(pick, target, (pick[target.id] ?? 0) >= target.left ? 0 : target.left)
}

/** The parts kept still fit the cart: every line they name exists and has the units they took. */
export function unitsFit(
  lines: readonly DishLine[],
  parts: readonly { units?: readonly DishUnits[] }[],
): boolean {
  const paid = paidUnits(parts)
  for (const [lineId, count] of paid) {
    const line = lines.find((l) => l.id === lineId)
    if (!line || count > line.quantity) return false
  }
  return true
}

/**
 * The cash to hand over for a cash part of `partCents`, when `takenCents` of cash parts were taken
 * before. The sale is signed with ONE cash line, rounded to 5 cents once (POS description v2.2
 * section 7.1: "une seule ligne de paiement sur le ticket, ARRONDIE"; the rounding of a ticket is
 * 2 cents at most), so the rounding of the whole cash is shared between the cash parts: each part
 * pays the difference of the rounded running cash total. The cash dues of all the parts add up
 * to the rounded cash line exactly. With no cash part before, this is the part rounded to 5 cents.
 */
export function cashDueCents(takenCents: number, partCents: number): number {
  return round5(takenCents + partCents) - round5(takenCents)
}
