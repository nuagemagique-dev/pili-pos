package dev.nuagemagique.pilipos.feature.reports

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.time.Instant

class SalesRangeTest {
    @Test
    fun todayIsTheCalendarDateInBrusselsWhateverTheDeviceZone() {
        // 22:30 UTC in October is 00:30 the next day in Brussels (+02:00).
        assertEquals("2026-10-18", SalesRange.calendarToday(Instant.parse("2026-10-17T22:30:00Z")))
        assertEquals("2026-10-17", SalesRange.calendarToday(Instant.parse("2026-10-17T21:30:00Z")))
        // Winter time: +01:00.
        assertEquals("2026-01-11", SalesRange.calendarToday(Instant.parse("2026-01-10T23:30:00Z")))
    }

    @Test
    fun theWeekRunsFromMondayEvenWhenTodayIsMonday() {
        assertEquals("2026-10-12" to "2026-10-17", SalesRange.presetRange(SalesPreset.WEEK, "2026-10-17"))
        assertEquals("2026-10-12" to "2026-10-12", SalesRange.presetRange(SalesPreset.WEEK, "2026-10-12"))
        assertEquals("2026-10-12" to "2026-10-18", SalesRange.presetRange(SalesPreset.WEEK, "2026-10-18"))
    }

    @Test
    fun theMonthRunsFromTheFirstAndYesterdayCrossesMonths() {
        assertEquals("2026-10-01" to "2026-10-17", SalesRange.presetRange(SalesPreset.MONTH, "2026-10-17"))
        assertEquals("2026-09-30" to "2026-09-30", SalesRange.presetRange(SalesPreset.YESTERDAY, "2026-10-01"))
        assertEquals("2025-12-31" to "2025-12-31", SalesRange.presetRange(SalesPreset.YESTERDAY, "2026-01-01"))
        assertEquals("2026-10-17" to "2026-10-17", SalesRange.presetRange(SalesPreset.TODAY, "2026-10-17"))
    }

    @Test
    fun typedDatesAreRealDatesOrNothing() {
        assertEquals("2026-10-15", SalesRange.parseTyped("15102026"))
        assertEquals("2028-02-29", SalesRange.parseTyped("29022028"))
        assertNull(SalesRange.parseTyped("29022026"))
        assertNull(SalesRange.parseTyped("31042026"))
        assertNull(SalesRange.parseTyped("0110202"))
        assertNull(SalesRange.parseTyped("01102026x"))
        assertNull(SalesRange.parseTyped(""))
    }

    @Test
    fun datesAndQuantitiesAreShownTheBelgianWay() {
        assertEquals("17/10/2026", SalesRange.showDate("2026-10-17"))
        assertEquals("15/10/____", SalesRange.showTyped("1510"))
        assertEquals("__/__/____", SalesRange.showTyped(""))
        assertEquals("05h", SalesRange.showHour(5))
        assertEquals("2", SalesRange.showQuantity(2_000L))
        assertEquals("0,350", SalesRange.showQuantity(350L))
        assertEquals("-1", SalesRange.showQuantity(-1_000L))
        assertEquals("-1,500", SalesRange.showQuantity(-1_500L))
    }
}
