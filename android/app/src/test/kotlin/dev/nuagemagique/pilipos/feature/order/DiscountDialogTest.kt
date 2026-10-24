package dev.nuagemagique.pilipos.feature.order

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.components.NumPadKey
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import uniffi.pos_ffi.DiscountInput
import uniffi.pos_ffi.DiscountKind
import uniffi.pos_ffi.DiscountReason
import uniffi.pos_ffi.DiscountTarget

/** What the discount dialog holds, as plain rules. */
class DiscountEntryTest {
    private fun typed(
        kind: DiscountKind,
        digits: String,
    ) = DiscountEntry(kind, digits, reason = DiscountReason.OTHER)

    @Test
    fun aPercentTakesTwoDigitsAndNoLeadingZero() {
        var e = DiscountEntry(DiscountKind.PERCENT)
        e = e.press(NumPadKey.Digit('0'))
        assertEquals("", e.digits)
        e = e.press(NumPadKey.Digit('7')).press(NumPadKey.Digit('5')).press(NumPadKey.Digit('1'))
        assertEquals("75", e.digits)
        assertEquals("7", e.press(NumPadKey.Backspace).digits)
        assertEquals("", e.press(NumPadKey.Clear).digits)
    }

    @Test
    fun anAmountTakesSevenDigits() {
        var e = DiscountEntry(DiscountKind.AMOUNT)
        repeat(9) { e = e.press(NumPadKey.Digit('1')) }
        assertEquals("1111111", e.digits)
    }

    @Test
    fun aKindChangeClearsWhatWasTyped() {
        val e = DiscountEntry(DiscountKind.PERCENT, "20").withKind(DiscountKind.AMOUNT)
        assertEquals(DiscountKind.AMOUNT, e.kind)
        assertEquals("", e.digits)
        assertEquals("15", DiscountEntry(DiscountKind.PERCENT).withQuickPercent(15L).digits)
    }

    @Test
    fun aPercentIsValidFromOneToNinetyNine() {
        assertFalse(typed(DiscountKind.PERCENT, "").valueOk(2400L))
        assertTrue(typed(DiscountKind.PERCENT, "1").valueOk(2400L))
        assertTrue(typed(DiscountKind.PERCENT, "99").valueOk(2400L))
    }

    @Test
    fun anAmountIsValidUpToTheBase() {
        assertTrue(typed(DiscountKind.AMOUNT, "2400").valueOk(2400L))
        assertFalse(typed(DiscountKind.AMOUNT, "2401").valueOk(2400L))
        assertFalse(typed(DiscountKind.AMOUNT, "").valueOk(2400L))
    }

    @Test
    fun offeredNeedsNoValueButAReasonAndSomethingToTakeOff() {
        assertTrue(DiscountEntry(DiscountKind.FREE, reason = DiscountReason.STAFF_MEAL).canConfirm(2400L))
        assertFalse(DiscountEntry(DiscountKind.FREE).canConfirm(2400L))
        assertFalse(DiscountEntry(DiscountKind.FREE, reason = DiscountReason.STAFF_MEAL).canConfirm(0L))
    }

    @Test
    fun thePreviewFollowsTheValue() {
        assertEquals(1200L, typed(DiscountKind.PERCENT, "50").previewCents(2400L))
        assertEquals(2400L, typed(DiscountKind.PERCENT, "").previewCents(2400L))
        assertEquals(0L, DiscountEntry(DiscountKind.FREE).previewCents(2400L))
    }

    @Test
    fun theInputCarriesTheTargetTheValueAndATrimmedText() {
        val order = DiscountEntry(DiscountKind.PERCENT, "20", DiscountReason.SERVICE_ISSUE, "  plat froid  ").toInput(null, 2400L)
        assertEquals(DiscountInput(DiscountTarget.Order, DiscountKind.PERCENT, 20L, DiscountReason.SERVICE_ISSUE, "plat froid"), order)
        val line = DiscountEntry(DiscountKind.FREE, "", DiscountReason.STAFF_MEAL, "   ").toInput(5L, 650L)
        assertEquals(DiscountInput(DiscountTarget.Line(5L), DiscountKind.FREE, null, DiscountReason.STAFF_MEAL, null), line)
    }

    @Test
    fun noInputWithoutAReasonOrWithAnInvalidValue() {
        assertNull(DiscountEntry(DiscountKind.PERCENT, "20").toInput(null, 2400L))
        assertNull(DiscountEntry(DiscountKind.PERCENT, "100", DiscountReason.OTHER).toInput(null, 2400L))
    }
}

/** The discount dialog (`DiscountDialog.vue`). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class DiscountDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val seen = mutableListOf<DiscountInput>()
    private var cancelled = 0

    private fun show(
        base: Long = 2400L,
        lineId: Long? = null,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box { DiscountDialog(base, lineId, onConfirm = { seen += it }, onCancel = { cancelled++ }) }
                }
            }
        }
        compose.waitForIdle()
    }

    private fun tap(tag: String) {
        compose.onNodeWithTag(tag).performClick()
        compose.waitForIdle()
    }

    @Test
    fun confirmStaysDisabledUntilAValueAndAReasonAreChosen() {
        show()

        compose.onNodeWithTag("discount_confirm").assertIsNotEnabled()
        tap("discount_quick_50")
        compose.onNodeWithTag("discount_confirm").assertIsNotEnabled()
        tap("discount_reason_staff_meal")
        compose.onNodeWithTag("discount_confirm").assertIsEnabled()
    }

    @Test
    fun aQuickPercentIsPreviewedAndConfirmedWithTheReason() {
        show()

        tap("discount_quick_50")
        compose.onNodeWithTag("discount_preview").assertTextEquals("Total 24,00 € → 12,00 €")
        compose.onNodeWithTag("discount_typed").assertTextEquals("50 %")
        tap("discount_reason_commercial_gesture")
        tap("discount_confirm")

        assertEquals(
            listOf(DiscountInput(DiscountTarget.Order, DiscountKind.PERCENT, 50L, DiscountReason.COMMERCIAL_GESTURE, null)),
            seen,
        )
    }

    @Test
    fun aCustomPercentIsTypedOnThePad() {
        show()

        tap("numpad_key_7")
        tap("numpad_key_5")
        compose.onNodeWithTag("discount_typed").assertTextEquals("75 %")
        compose.onNodeWithTag("discount_preview").assertTextEquals("Total 24,00 € → 6,00 €")
    }

    @Test
    fun anAmountIsTypedInCentsAndCappedAtTheTotal() {
        show()

        tap("discount_kind_amount")
        for (d in "500") tap("numpad_key_$d")
        compose.onNodeWithTag("discount_typed").assertTextEquals("5,00 €")
        compose.onNodeWithTag("discount_preview").assertTextEquals("Total 24,00 € → 19,00 €")

        for (d in "00") tap("numpad_key_$d")
        tap("discount_reason_other")
        // 500,00 EUR is more than the 24,00 EUR total.
        compose.onNodeWithTag("discount_confirm").assertIsNotEnabled()
    }

    @Test
    fun offeredNeedsNoValue() {
        show()

        tap("discount_kind_free")
        compose.onNodeWithTag("discount_preview").assertTextEquals("Total 24,00 € → 0,00 €")
        tap("discount_reason_staff_meal")
        tap("discount_confirm")

        assertEquals(DiscountKind.FREE, seen.single().kind)
        assertNull(seen.single().value)
    }

    @Test
    fun theReasonTextIsTypedOnTheKeyboardAndSent() {
        show()

        tap("discount_quick_10")
        tap("discount_reason_service_issue")
        compose.onNodeWithTag("discount_reason_text").performTextInput("plat froid")
        tap("discount_confirm")

        assertEquals("plat froid", seen.single().reasonText)
    }

    @Test
    fun aLineOnlyOffersTheReasonsAndPreviewsTheLineGoingToZero() {
        show(base = 650L, lineId = 4L)

        compose.onNodeWithTag("discount_kind_percent").assertDoesNotExist()
        compose.onNodeWithTag("numpad_key_1").assertDoesNotExist()
        compose.onNodeWithTag("discount_preview").assertTextEquals("Total 6,50 € → 0,00 €")
        compose.onNodeWithTag("discount_confirm").assertIsNotEnabled()
        tap("discount_reason_commercial_gesture")
        tap("discount_confirm")

        assertEquals(DiscountTarget.Line(4L), seen.single().target)
        assertEquals(DiscountKind.FREE, seen.single().kind)
    }

    @Test
    fun backCancelsWithoutAnInput() {
        show()

        compose.onNodeWithTag("discount_dialog").assertIsDisplayed()
        tap("discount_cancel")

        assertEquals(1, cancelled)
        assertTrue(seen.isEmpty())
    }
}
