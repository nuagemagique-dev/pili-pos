package dev.nuagemagique.pilipos.feature.order

// ── Digit after an add = quantity (`stepDigitWindow` of `ui/src/utils/codeEntry.ts`) ──

/** How long a digit typed on the pad still sets the quantity of the product just added, in milliseconds. */
internal const val DIGIT_WINDOW_MS: Long = 3_000L

/** Digits a quantity takes in the window (1 to 99). */
private const val DIGIT_WINDOW_DIGITS = 2

private val DigitKey = Regex("^[0-9]$")

/**
 * The window that follows an add of a plain product: for [DIGIT_WINDOW_MS], a digit typed while the code field is
 * empty sets how many of that product this add gave ("3" makes the add count 3, "1" then "2" make it 12). The deadline
 * moves with each digit. Pure: the caller passes the clock.
 */
internal data class DigitWindow(
    /** The cart line the add went to (a merged line keeps its earlier units). */
    val lineId: Long,
    val itemId: Long,
    /** How many units this add gave so far. */
    val quantity: Int,
    /** The digits typed in the window. */
    val digits: String,
    /** The clock value (ms) after which the window is closed. */
    val expiresAt: Long,
)

/** The window opened by an add of [quantity] units to [lineId] at [now]. */
internal fun openDigitWindow(
    lineId: Long,
    itemId: Long,
    quantity: Int,
    now: Long,
): DigitWindow = DigitWindow(lineId, itemId, quantity, "", now + DIGIT_WINDOW_MS)

/** What a key does to the window: it sets the add's quantity, or it is not for the window. */
internal sealed interface DigitStep {
    data class Set(
        val window: DigitWindow,
    ) : DigitStep

    data object Pass : DigitStep
}

/**
 * A key typed on the empty code field. A digit inside the open window sets (or extends) the add's quantity, never
 * below the units the add gave; anything
 * else closes the window and starts a code as usual: a letter, the multiply sign, a first "0", a third digit, a digit
 * after the deadline.
 */
internal fun stepDigitWindow(
    window: DigitWindow?,
    key: String,
    now: Long,
): DigitStep {
    val takes =
        window != null && now <= window.expiresAt && DigitKey.matches(key) &&
            window.digits.length < DIGIT_WINDOW_DIGITS && !(window.digits.isEmpty() && key == "0")
    if (!takes || window == null) return DigitStep.Pass
    val digits = window.digits + key
    // Never under the units the add already gave: a smaller digit leaves the line as it is.
    return DigitStep.Set(
        window.copy(digits = digits, quantity = maxOf(digits.toInt(), window.quantity), expiresAt = now + DIGIT_WINDOW_MS),
    )
}
