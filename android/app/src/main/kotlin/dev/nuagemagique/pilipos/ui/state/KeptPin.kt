package dev.nuagemagique.pilipos.ui.state

/**
 * The manager PIN kept for the discounts of one order (UX audit P8), `utils/pinGrant.ts` of the Vue UI.
 *
 * The ENGINE owns the grant (`commands/pin_grant.rs`): after a discount confirmed by the PIN it answers
 * `pinGrantMs`, the time the same manager may still discount the same order with no PIN. The screen keeps only a
 * hint of that answer to decide whether to ask for the PIN first or to try without it. The hint can be stale (a
 * lock, an order switch or the sale dropped the grant): the engine then answers `pin_required` and the screen falls
 * back to the PIN pad, so a wrong hint costs one extra step, never a discount without a PIN.
 */
internal data class PinGrantHint(
    val orderId: Long,
    /** The wall clock value (milliseconds) after which the grant is over. */
    val expiresAtMs: Long,
)

internal object KeptPin {
    /** The code the engine answers when it keeps no PIN for the order. */
    const val PIN_REQUIRED = "pin_required"

    private const val MINUTE_MS = 60_000L

    /** The hint after a discount command: none when the engine kept no grant. */
    fun hintAfter(
        orderId: Long,
        pinGrantMs: Long?,
        nowMs: Long,
    ): PinGrantHint? = if (pinGrantMs == null || pinGrantMs <= 0L) null else PinGrantHint(orderId, nowMs + pinGrantMs)

    /** Whether to try the next discount of [orderId] without the PIN. */
    fun triesWithoutPin(
        hint: PinGrantHint?,
        orderId: Long?,
        nowMs: Long,
    ): Boolean = hint != null && orderId != null && hint.orderId == orderId && nowMs < hint.expiresAtMs

    /** Whole minutes left, rounded up, for the "code gardé" toast (at least 1). */
    fun minutesKept(pinGrantMs: Long): Int = maxOf(1L, (pinGrantMs + MINUTE_MS - 1) / MINUTE_MS).toInt()
}
