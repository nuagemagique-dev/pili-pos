package dev.nuagemagique.pilipos.feature.reports

import java.time.DateTimeException
import java.time.DayOfWeek
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.temporal.TemporalAdjusters

/** The range choices of the sales tab (`Preset` of `ui/src/utils/salesRange.ts`). */
internal enum class SalesPreset {
    TODAY,
    YESTERDAY,
    WEEK,
    MONTH,
    CUSTOM,
}

/**
 * Date arithmetic of the sales report presets, a port of `ui/src/utils/salesRange.ts`. Dates are booking dates,
 * `YYYY-MM-DD` strings in Belgian time. Nothing here depends on the device time zone: the calendar date of "now"
 * is read in Europe/Brussels, and the rest is calendar arithmetic.
 */
internal object SalesRange {
    /** The time zone of every date the till shows. */
    val BRUSSELS: ZoneId = ZoneId.of("Europe/Brussels")

    private const val TYPED_LENGTH = 8
    private const val DATE_PARTS = 3
    private const val DAY_DIGITS = 2
    private const val MONTH_END = 4

    /** The calendar date in Belgium at [now]: the first guess of "today" before the engine says the business date. */
    fun calendarToday(now: Instant = Instant.now()): String = now.atZone(BRUSSELS).toLocalDate().toString()

    /** The range of [preset] around [today] (the business date): the week runs from Monday, the month from the 1st. */
    fun presetRange(
        preset: SalesPreset,
        today: String,
    ): Pair<String, String> {
        val day = LocalDate.parse(today)
        return when (preset) {
            SalesPreset.TODAY, SalesPreset.CUSTOM -> today to today
            SalesPreset.YESTERDAY -> day.minusDays(1).toString().let { it to it }
            SalesPreset.WEEK -> day.with(TemporalAdjusters.previousOrSame(DayOfWeek.MONDAY)).toString() to today
            SalesPreset.MONTH -> day.withDayOfMonth(1).toString() to today
        }
    }

    /** `ddmmyyyy` typed on the pad as `YYYY-MM-DD`, or null when it is not a real date. */
    fun parseTyped(digits: String): String? {
        if (digits.length != TYPED_LENGTH || !digits.all { it in '0'..'9' }) return null
        val day = digits.substring(0, DAY_DIGITS).toInt()
        val month = digits.substring(DAY_DIGITS, MONTH_END).toInt()
        val year = digits.substring(MONTH_END).toInt()
        return try {
            LocalDate.of(year, month, day).toString()
        } catch (_: DateTimeException) {
            null
        }
    }

    /** `YYYY-MM-DD` as `dd/mm/yyyy`. */
    fun showDate(date: String): String {
        val parts = date.split("-")
        return if (parts.size == DATE_PARTS) "${parts[2]}/${parts[1]}/${parts[0]}" else date
    }

    /** The digits typed so far as `dd/mm/yyyy`, the missing ones as underscores. */
    fun showTyped(digits: String): String {
        val padded = digits.padEnd(TYPED_LENGTH, '_')
        return "${padded.substring(0, DAY_DIGITS)}/${padded.substring(DAY_DIGITS, MONTH_END)}/${padded.substring(MONTH_END)}"
    }

    /** `0` to `23` as `HHh`. */
    fun showHour(hour: Int): String = hour.toString().padStart(DAY_DIGITS, '0') + "h"

    /** A quantity in thousandths as `2` or `0,350` (a decimal comma, three decimals when not whole). */
    fun showQuantity(milli: Long): String {
        val sign = if (milli < 0L) "-" else ""
        val magnitude = if (milli < 0L) -milli else milli
        val whole = magnitude / MILLI
        val fraction = magnitude % MILLI
        return if (fraction == 0L) "$sign$whole" else "$sign$whole,${fraction.toString().padStart(MILLI_DIGITS, '0')}"
    }

    private const val MILLI = 1000L
    private const val MILLI_DIGITS = 3
}
