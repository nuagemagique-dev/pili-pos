package dev.nuagemagique.pilipos.ui.format

import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test
import java.time.Instant
import java.time.ZoneId
import java.util.Locale
import java.util.TimeZone

/**
 * The expected texts are what Node prints for `toLocaleString('fr-BE', { timeZone: 'Europe/Brussels', ... })`.
 * The JVM default zone and locale are set to Tokyo and en-US: the Brussels text must not change.
 */
class PosDateTimeTest {
    private lateinit var savedZone: TimeZone
    private lateinit var savedLocale: Locale

    @Before
    fun foreignDefaults() {
        savedZone = TimeZone.getDefault()
        savedLocale = Locale.getDefault()
        TimeZone.setDefault(TimeZone.getTimeZone("Asia/Tokyo"))
        Locale.setDefault(Locale.US)
    }

    @After
    fun restoreDefaults() {
        TimeZone.setDefault(savedZone)
        Locale.setDefault(savedLocale)
    }

    private fun at(iso: String): Instant = Instant.parse(iso)

    private fun texts(iso: String): List<String> =
        at(iso).let { listOf(PosDateTime.formatTime(it), PosDateTime.formatDate(it), PosDateTime.formatDayTime(it)) }

    @Test
    fun theZoneIsBrussels() {
        assertEquals(ZoneId.of("Europe/Brussels"), PosDateTime.ZONE)
    }

    @Test
    fun summerTimeIsTwoHoursAheadOfUtc() {
        assertEquals(listOf("14:05", "15/07/2026", "15/07 14:05"), texts("2026-07-15T12:05:00Z"))
    }

    @Test
    fun winterTimeIsOneHourAheadOfUtc() {
        assertEquals(listOf("14:05", "15/01/2026", "15/01 14:05"), texts("2026-01-15T13:05:00Z"))
    }

    @Test
    fun theSpringForwardJumpsFromOneFiftyNineToThree() {
        assertEquals("01:59", PosDateTime.formatTime(at("2026-03-29T00:59:59Z")))
        assertEquals("03:00", PosDateTime.formatTime(at("2026-03-29T01:00:00Z")))
    }

    @Test
    fun theDayChangesInBrusselsNotInUtc() {
        assertEquals(listOf("00:30", "01/01/2027", "01/01 00:30"), texts("2026-12-31T23:30:00Z"))
    }

    @Test
    fun aTimestampWithoutAZoneIsUtc() {
        assertEquals(at("2026-10-09T10:00:00Z"), PosDateTime.parseTimestamp("2026-10-09 10:00:00"))
        assertEquals(at("2026-10-09T10:00:00Z"), PosDateTime.parseTimestamp("2026-10-09T10:00:00"))
        assertEquals(at("2026-10-09T10:00:00Z"), PosDateTime.parseTimestamp("2026-10-09 10:00"))
        assertEquals(at("2026-10-09T10:00:00.123Z"), PosDateTime.parseTimestamp("2026-10-09T10:00:00.123"))
        assertEquals(at("2026-10-09T00:00:00Z"), PosDateTime.parseTimestamp("2026-10-09"))
    }

    @Test
    fun aTimestampWithAZoneIsTakenAsItIs() {
        assertEquals(at("2026-10-09T10:00:00Z"), PosDateTime.parseTimestamp("2026-10-09T10:00:00Z"))
        assertEquals(at("2026-10-09T08:00:00Z"), PosDateTime.parseTimestamp("2026-10-09T10:00:00+02:00"))
        assertEquals(at("2026-10-09T08:00:00Z"), PosDateTime.parseTimestamp("2026-10-09T10:00:00+0200"))
        assertEquals(at("2026-10-09T10:30:00Z"), PosDateTime.parseTimestamp("2026-10-09 10:00:00-00:30"))
    }

    @Test
    fun anUnparsableTimestampIsNull() {
        assertNull(PosDateTime.parseTimestamp("garbage"))
        assertNull(PosDateTime.parseTimestamp(""))
        assertNull(PosDateTime.parseTimestamp("2026-13-45 10:00:00"))
    }

    @Test
    fun formatTimestampFormatsOrKeepsTheRawText() {
        assertEquals("12:00", PosDateTime.formatTimestamp("2026-10-09 10:00:00", PosDateTime::formatTime))
        assertEquals("garbage", PosDateTime.formatTimestamp("garbage", PosDateTime::formatTime))
        assertNotNull(PosDateTime.parseTimestamp("2026-10-09 10:00:00"))
    }

    @Test
    fun minutesSinceCountsWholeMinutesAndNeverGoesNegative() {
        val now = at("2026-10-09T10:30:59Z")
        assertEquals(30L, PosDateTime.minutesSince("2026-10-09 10:00:00", now))
        assertEquals(0L, PosDateTime.minutesSince("2026-10-09 10:30:00", now))
        assertEquals(0L, PosDateTime.minutesSince("2026-10-09 11:00:00", now))
        assertEquals(0L, PosDateTime.minutesSince("garbage", now))
        assertEquals(90L, PosDateTime.minutesSince("2026-10-09T10:00:00+02:00", at("2026-10-09T09:30:00Z")))
    }
}
