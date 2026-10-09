package dev.nuagemagique.pilipos.nightly

import java.time.DateTimeException
import java.time.OffsetDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale

/** Times of the screen: always the Brussels wall clock, never the zone of the device. */
internal object NightlyTime {
    /** The zone of the restaurant. */
    val ZONE: ZoneId = ZoneId.of("Europe/Brussels")

    private val DAY_TIME: DateTimeFormatter = DateTimeFormatter.ofPattern("dd/MM HH:mm", Locale.ROOT).withZone(ZONE)

    /** A backend timestamp (RFC 3339) as `25/09 14:05` in Brussels; the raw text when it does not parse. */
    fun format(iso: String): String =
        try {
            DAY_TIME.format(OffsetDateTime.parse(iso))
        } catch (_: DateTimeException) {
            iso
        }

    /** [format] of [iso], or [never] when there is no timestamp. */
    fun formatOrNever(
        iso: String?,
        never: String,
    ): String = if (iso == null) never else format(iso)
}
