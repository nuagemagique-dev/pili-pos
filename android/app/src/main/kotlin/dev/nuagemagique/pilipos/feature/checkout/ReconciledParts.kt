package dev.nuagemagique.pilipos.feature.checkout

import dev.nuagemagique.pilipos.ui.format.CashDisplay
import dev.nuagemagique.pilipos.ui.state.HeldPart

/** The parts to show again at the checkout, and whether some had to be dropped. */
internal data class ReconciledParts(
    val parts: List<HeldPart>,
    val dropped: Boolean,
)

/**
 * `reconcileHeld` of `utils/heldParts.ts` (UX audit P4). [lockedCents] is what the card parts the terminal approved
 * already pay of the order's [totalCents]. Each part taken by hand must fit in what is left; as soon as one does not
 * (the order changed meanwhile) every part taken by hand is dropped and [ReconciledParts.dropped] says so, for the
 * warning.
 */
internal fun reconcileHeld(
    parts: List<HeldPart>,
    totalCents: Long,
    lockedCents: Long,
): ReconciledParts {
    // The parts taken so far, with the locked card parts, must never pass the total: compared on the running sum.
    val fits =
        parts.indices.all {
            CashDisplay.sumCents(listOf(lockedCents) + parts.take(it + 1).map { part -> part.cents }) <= totalCents
        }
    return if (fits) ReconciledParts(parts, dropped = false) else ReconciledParts(emptyList(), dropped = true)
}
