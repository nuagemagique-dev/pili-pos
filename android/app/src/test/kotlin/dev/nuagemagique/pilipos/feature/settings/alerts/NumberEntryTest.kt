package dev.nuagemagique.pilipos.feature.settings.alerts

import dev.nuagemagique.pilipos.ui.components.NumPadKey
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

private fun NumberEntry.type(digits: String): NumberEntry = digits.fold(this) { e, d -> e.press(NumPadKey.Digit(d)) }

class NumberEntryTest {
    @Test
    fun aChatIdKeepsEveryDigitAndItsMinusSign() {
        val entry = NumberEntry(AlertNumber.ChatId).type("1001234567890").toggleMinus()

        assertEquals("-1001234567890", entry.text)
        assertEquals("-1001234567890", entry.result)
        assertTrue(entry.valid)
    }

    @Test
    fun aChatIdStopsAtTwentyDigits() {
        val entry = NumberEntry(AlertNumber.ChatId).type("123456789012345678901234")

        assertEquals("12345678901234567890", entry.digits)
    }

    @Test
    fun nothingTypedIsNotValidAndShowsNothing() {
        val entry = NumberEntry(AlertNumber.ChatId)

        assertEquals("", entry.text)
        assertFalse(entry.valid)
        assertFalse(NumberEntry(AlertNumber.ChatId, minus = true).valid)
    }

    @Test
    fun theKeysEditTheDigits() {
        val typed = NumberEntry(AlertNumber.Reminder).type("120")

        assertEquals("12", typed.press(NumPadKey.Backspace).digits)
        assertEquals("", typed.press(NumPadKey.Clear).digits)
        assertEquals("", NumberEntry(AlertNumber.Reminder).press(NumPadKey.Backspace).digits)
    }

    @Test
    fun aSmallSettingIsKeptInItsRange() {
        assertTrue(NumberEntry(AlertNumber.ZDeadlineHour).type("12").valid)
        assertFalse(NumberEntry(AlertNumber.ZDeadlineHour).type("13").valid)
        assertTrue(NumberEntry(AlertNumber.ZDeadlineHour).type("0").valid)
        assertFalse(NumberEntry(AlertNumber.OnlineWait).type("0").valid)
        assertTrue(NumberEntry(AlertNumber.OnlineWait).type("120").valid)
        assertFalse(NumberEntry(AlertNumber.OnlineWait).type("121").valid)
        assertTrue(NumberEntry(AlertNumber.Reminder).type("0").valid)
        assertTrue(NumberEntry(AlertNumber.Reminder).type("1440").valid)
        assertFalse(NumberEntry(AlertNumber.Reminder).type("1441").valid)
    }

    @Test
    fun aSmallSettingHasNoMinusAndNoLeadingZero() {
        val entry = NumberEntry(AlertNumber.OnlineWait).type("007").toggleMinus()

        assertFalse(entry.minus)
        assertEquals("007", entry.text)
        assertEquals("7", entry.result)
    }

    @Test
    fun theDialogStartsFromTheCurrentValue() {
        val group = numberEntryFor(AlertNumber.ChatId, "-4242")
        assertEquals("4242", group.digits)
        assertTrue(group.minus)
        assertEquals("-4242", group.result)

        val hour = numberEntryFor(AlertNumber.ZDeadlineHour, "5")
        assertEquals("5", hour.digits)
        assertFalse(hour.minus)

        val none = numberEntryFor(AlertNumber.ChatId, "")
        assertEquals("", none.digits)
    }

    @Test
    fun theDraftTakesTheTypedValues() {
        val draft = ownerAlerts(chatId = "1").toDraft()

        assertEquals("-9", draft.with(AlertNumber.ChatId, "-9").chatId)
        assertEquals(7L, draft.with(AlertNumber.ZDeadlineHour, "7").zDeadlineHour)
        assertEquals(9L, draft.with(AlertNumber.OnlineWait, "9").onlineWaitMinutes)
        assertEquals(0L, draft.with(AlertNumber.Reminder, "0").reminderMinutes)
        assertEquals("1", draft.valueOf(AlertNumber.ChatId))
        assertEquals("5", draft.valueOf(AlertNumber.ZDeadlineHour))
        assertEquals("5", draft.valueOf(AlertNumber.OnlineWait))
        assertEquals("60", draft.valueOf(AlertNumber.Reminder))
    }

    @Test
    fun everyKindFlipsAloneAndAnUnknownKindIsIgnored() {
        val kinds = allKinds()

        ALERT_KINDS.forEach { kind ->
            val off = kinds.toggled(kind)
            assertFalse(kind, off.isOn(kind))
            assertEquals(kind, ALERT_KINDS.filter { off.isOn(it) }.size, ALERT_KINDS.size - 1)
            assertEquals(kinds, off.toggled(kind))
        }
        assertEquals(kinds, kinds.toggled("nope"))
        assertFalse(kinds.isOn("nope"))
    }

    @Test
    fun anUnknownLanguageIsReadAsFrench() {
        assertEquals("fr", ownerAlerts(language = "nl").toDraft().language)
        assertEquals("zh", ownerAlerts(language = "zh").toDraft().language)
    }
}
