package dev.nuagemagique.pilipos.ui.format

import java.time.Instant
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.OffsetDateTime
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.time.format.DateTimeFormatterBuilder
import java.time.temporal.ChronoUnit
import java.util.Locale

/**
 * Port of `ui/src/utils/datetime.ts`: every date the till shows is Belgian time, whatever zone the device
 * runs in (tickets and reports are fiscally in Europe/Brussels). The zone and the locale are fixed here; the
 * system ones are never read.
 */
object PosDateTime {
    /** The zone of every displayed date and time. */
    val ZONE: ZoneId = ZoneId.of("Europe/Brussels")

    private val LOCALE: Locale = Locale.forLanguageTag("fr-BE")

    private val TIME: DateTimeFormatter = formatter("HH:mm")
    private val DATE: DateTimeFormatter = formatter("dd/MM/yyyy")
    private val DAY_TIME: DateTimeFormatter = formatter("dd/MM HH:mm")

    /** The zone suffix the Vue code recognises: `Z`, `+02:00`, `+0200`. */
    private val ZONE_SUFFIX = Regex("(Z|[+-]\\d{2}:?\\d{2})$")

    /** An ISO local date time followed by an offset written without the colon (`+0200`). */
    private val COMPACT_OFFSET: DateTimeFormatter =
        DateTimeFormatterBuilder()
            .append(DateTimeFormatter.ISO_LOCAL_DATE_TIME)
            .appendOffset("+HHmm", "Z")
            .toFormatter(Locale.ROOT)

    private fun formatter(pattern: String): DateTimeFormatter = DateTimeFormatter.ofPattern(pattern, LOCALE).withZone(ZONE)

    /**
     * Parses a backend timestamp. SQLite's `datetime('now')` gives `YYYY-MM-DD HH:MM:SS` in UTC without a zone;
     * a timestamp with a zone is taken as it is. Null when [iso] does not parse (never a made-up date).
     */
    fun parseTimestamp(iso: String): Instant? {
        val text = iso.trim().replaceFirst(' ', 'T')
        return runCatching { if (ZONE_SUFFIX.containsMatchIn(text)) parseWithZone(text) else parseUtc(text) }.getOrNull()
    }

    private fun parseWithZone(text: String): Instant =
        runCatching { OffsetDateTime.parse(text, DateTimeFormatter.ISO_OFFSET_DATE_TIME).toInstant() }
            .getOrElse { OffsetDateTime.parse(text, COMPACT_OFFSET).toInstant() }

    private fun parseUtc(text: String): Instant =
        if ('T' in text) {
            LocalDateTime.parse(text).toInstant(ZoneOffset.UTC)
        } else {
            LocalDate.parse(text).atStartOfDay().toInstant(ZoneOffset.UTC)
        }

    /** `14:05` */
    fun formatTime(instant: Instant): String = TIME.format(instant)

    /** `25/09/2026` */
    fun formatDate(instant: Instant): String = DATE.format(instant)

    /** `25/09 14:05` */
    fun formatDayTime(instant: Instant): String = DAY_TIME.format(instant)

    /** A backend timestamp through [formatter], or the raw text when it does not parse. */
    fun formatTimestamp(
        iso: String,
        formatter: (Instant) -> String,
    ): String = parseTimestamp(iso)?.let(formatter) ?: iso

    /** Whole minutes from a backend timestamp to [now] (0 when unparsable or in the future). */
    fun minutesSince(
        iso: String,
        now: Instant,
    ): Long = parseTimestamp(iso)?.let { ChronoUnit.MINUTES.between(it, now).coerceAtLeast(0L) } ?: 0L
}
