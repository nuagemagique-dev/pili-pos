package dev.nuagemagique.pilipos.ui.format

/** Cents of 5 EUR: the step of the first quick amount. */
private const val STEP_FIVE_EUROS = 500L

/** Cents of 10 EUR: the step of the second quick amount. */
private const val STEP_TEN_EUROS = 1000L

private const val NOTE_20_EUROS = 2000L
private const val NOTE_50_EUROS = 5000L
private const val NOTE_100_EUROS = 10000L
private const val NOTE_200_EUROS = 20000L
private const val NOTE_500_EUROS = 50000L

/** The bank notes offered after the two steps, in cents (Vue: 20, 50, 100, 200, 500 EUR). */
private val BANK_NOTES_CENTS = listOf(NOTE_20_EUROS, NOTE_50_EUROS, NOTE_100_EUROS, NOTE_200_EUROS, NOTE_500_EUROS)

/** How many quick amounts the till shows at most. */
private const val QUICK_AMOUNTS_MAX = 3

/**
 * The four display amounts of the checkout and the refund for which the engine offers no helper (UI-PD306).
 *
 * Ported from `CheckoutView.vue` (`changeCents`, `equalShares`, `quickAmounts`) and `ReportsView.vue`
 * (`refundTotalCents`, its sum). Integers only: no floating point, no rounding. Every line that does arithmetic
 * on a cents value ends with the display-only marker that `p5_gate.sh` (check `ui-money`) exempts in this file
 * and nowhere else.
 *
 * The 5-cent cash rounding does not live here: it stays in the engine, reached through
 * `EngineHelpers.cashDueCents` and `settlePreview`. These functions only place cents the engine has already
 * settled (the rounded cash due, the total to split); the engine still refuses any inconsistent set of parts.
 */
object CashDisplay {
    /** The change to give back: what was handed over minus the (rounded) cash due, never below 0. */
    fun changeCents(
        tenderedCents: Long,
        dueCents: Long,
    ): Long = maxOf(0L, tenderedCents - dueCents) // money-display-only

    /**
     * [totalCents] in [parts] shares, the remainder cents on the first shares: 6030 in 4 gives 1508, 1508, 1507,
     * 1507. The shares always add up to [totalCents].
     */
    fun equalShares(
        totalCents: Long,
        parts: Int,
    ): List<Long> {
        require(parts >= 1) { "parts must be at least 1" }
        require(totalCents >= 0L) { "totalCents must not be negative" }
        val base = totalCents / parts // money-display-only
        val extra = totalCents - base * parts // money-display-only
        return List(parts) { index -> if (index < extra) base + 1L else base } // money-display-only
    }

    /**
     * The likely amounts handed over for a [roundedDueCents] (Vue `quickAmounts`, in cents): the next 5 and 10 EUR
     * multiples, then the bank notes of 20, 50, 100, 200 and 500 EUR; only values strictly above the due (so an
     * exact multiple is not offered itself), distinct, ascending, at most three.
     */
    fun quickAmountsCents(roundedDueCents: Long): List<Long> {
        require(roundedDueCents >= 0L) { "roundedDueCents must not be negative" }
        val nextFive = (roundedDueCents + STEP_FIVE_EUROS - 1L) / STEP_FIVE_EUROS * STEP_FIVE_EUROS // money-display-only
        val nextTen = (roundedDueCents + STEP_TEN_EUROS - 1L) / STEP_TEN_EUROS * STEP_TEN_EUROS // money-display-only
        return (listOf(nextFive, nextTen) + BANK_NOTES_CENTS)
            .filter { it > roundedDueCents }
            .distinct()
            .sorted()
            .take(QUICK_AMOUNTS_MAX)
    }

    /** The sum of [values] (the refund preview: the line totals of the selection). Fails on overflow. */
    fun sumCents(values: List<Long>): Long = values.fold(0L) { acc, value -> Math.addExact(acc, value) } // money-display-only
}
