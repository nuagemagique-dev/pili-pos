// The manager PIN kept for the discounts of one order (UX audit P8).
//
// The ENGINE owns the grant (`commands/pin_grant.rs`): after a discount
// confirmed by the PIN it answers `pinGrantMs`, the time the same manager
// may still discount the same order with no PIN. The screen keeps only a
// hint of that answer to decide whether to ask for the PIN first or to try
// without it. The hint can be stale (a lock, an order switch or the sale
// dropped the grant): the engine then answers `pin_required` and the screen
// falls back to the PIN pad, so a wrong hint costs one extra step, never a
// discount without a PIN.

export interface PinGrantHint {
  orderId: number
  /** `Date.now()` value after which the grant is over. */
  expiresAt: number
}

/** The hint after a discount command: none when the engine kept no grant. */
export function hintAfter(
  orderId: number,
  pinGrantMs: number | null | undefined,
  now: number,
): PinGrantHint | null {
  if (pinGrantMs === null || pinGrantMs === undefined || pinGrantMs <= 0) return null
  return { orderId, expiresAt: now + pinGrantMs }
}

/** Whether to try the next discount of `orderId` without the PIN. */
export function triesWithoutPin(
  hint: PinGrantHint | null,
  orderId: number | undefined,
  now: number,
): boolean {
  return hint !== null && orderId !== undefined && hint.orderId === orderId && now < hint.expiresAt
}

/** Whole minutes left, rounded up, for the "code gardé" toast (at least 1). */
export function minutesKept(pinGrantMs: number): number {
  return Math.max(1, Math.ceil(pinGrantMs / 60_000))
}
