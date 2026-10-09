package dev.nuagemagique.pilipos.ui.format

/** Narrow no-break space (U+202F): the digit grouping separator that `toLocaleString('fr-BE')` prints. */
private const val GROUP_SEPARATOR = "\u202F"

/** Decimal separator of fr-BE. */
private const val DECIMAL_SEPARATOR = ","

/** Group size of the integer part. */
private const val GROUP_SIZE = 3

/** Cents in a euro, as an unsigned value (the magnitude is unsigned so that `Long.MIN_VALUE` has a magnitude). */
private const val CENTS_PER_EURO = 100uL

/** Number of digits of the cents part. */
private const val FRACTION_DIGITS = 2

/**
 * Port of `formatPrice` of `ui/src/utils/format.ts`: [cents] as euros with two decimals in the fr-BE style,
 * `1250` gives `12,50` and `123456789` gives `1 234 567,89` (the group separator is U+202F, the minus sign is
 * the ASCII hyphen, as Node prints them).
 *
 * Display only: this is the one place of the UI that divides a cents value, by 100, to place the decimal
 * separator. It works on the absolute value as an unsigned integer (no Double, no rounding), so every Long
 * including `Long.MIN_VALUE` gives its exact text. Money is never computed in Kotlin (PD208).
 */
fun formatCents(cents: Long): String {
    val negative = cents < 0L
    // Two's complement negation on the unsigned reading: exact for every Long including Long.MIN_VALUE.
    val magnitude: ULong = if (negative) 0uL - cents.toULong() else cents.toULong() // money-format-only
    val euros: ULong = magnitude / CENTS_PER_EURO // money-format-only
    val fraction: ULong = magnitude % CENTS_PER_EURO // money-format-only
    val grouped =
        euros
            .toString()
            .reversed()
            .chunked(GROUP_SIZE)
            .joinToString(GROUP_SEPARATOR)
            .reversed()
    val sign = if (negative) "-" else ""
    return sign + grouped + DECIMAL_SEPARATOR + fraction.toString().padStart(FRACTION_DIGITS, '0')
}

/** [formatCents] followed by a space and the euro sign, as the Vue templates write a price (`{{ formatPrice(c) }} €`). */
fun formatEuro(cents: Long): String = formatCents(cents) + " €"
