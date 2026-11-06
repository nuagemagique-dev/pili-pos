package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

/** The rules of the whole number dialog (`NumberEntryDialog.vue`). */
class NumberKeypadRulesTest {
    private val spec = NumberKeypadSpec("n", "Nombre", maxDigits = 3)

    @Test
    fun aDigitIsAppendedUpToTheLimit() {
        assertEquals("12", NumberKeypadRules.press("1", NumPadKey.Digit('2'), 3))
        assertEquals("123", NumberKeypadRules.press("123", NumPadKey.Digit('4'), 3))
    }

    @Test
    fun backspaceAndClearErase() {
        assertEquals("1", NumberKeypadRules.press("12", NumPadKey.Backspace, 3))
        assertEquals("", NumberKeypadRules.press("12", NumPadKey.Clear, 3))
    }

    @Test
    fun nothingTypedIsNotValid() {
        assertFalse(NumberKeypadRules.valid("", spec))
    }

    @Test
    fun aValueIsCheckedAgainstTheBounds() {
        val bounded = spec.copy(min = 12, max = 36)
        assertFalse(NumberKeypadRules.valid("11", bounded))
        assertTrue(NumberKeypadRules.valid("12", bounded))
        assertTrue(NumberKeypadRules.valid("36", bounded))
        assertFalse(NumberKeypadRules.valid("37", bounded))
    }

    @Test
    fun anExactLengthKeepsTheLeadingZeros() {
        val exact = spec.copy(maxDigits = 11, exactDigits = 11)
        assertFalse(NumberKeypadRules.valid("0104", exact))
        assertTrue(NumberKeypadRules.valid("01040412345", exact))
        assertEquals("01040412345", NumberKeypadRules.result("01040412345", exact))
    }

    @Test
    fun aPlainNumberDropsItsLeadingZeros() {
        assertEquals("7", NumberKeypadRules.result("007", spec))
    }
}

/** The whole number dialog of a settings field. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class NumberKeypadTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val confirmed = mutableListOf<String>()
    private var cancelled = 0

    private fun show(spec: NumberKeypadSpec) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box { NumberKeypad(spec, onConfirm = { confirmed += it }, onCancel = { cancelled++ }) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun itOpensOnTheWholePartOfTheCurrentValueAndOkNeedsADigit() {
        show(NumberKeypadSpec("n", "Validité", initial = "12.5", maxDigits = 3))

        compose.onNodeWithTag("n_dialog").assertIsDisplayed()
        compose.onNodeWithTag("n_value").assertTextEquals("12")
        compose.onNodeWithTag("numpad_clear").performClick()
        compose.onNodeWithTag("n_value").assertTextEquals("0")
        compose.onNodeWithTag("n_ok").assertIsNotEnabled()
    }

    @Test
    fun theDigitsAreTypedOnThePadAndConfirmed() {
        show(NumberKeypadSpec("n", "Validité", maxDigits = 2, min = 12, max = 36))

        compose.typeOnNumPad("24")
        compose.onNodeWithTag("n_value").assertTextEquals("24")
        compose.onNodeWithTag("n_ok").assertIsEnabled().performClick()

        assertEquals(listOf("24"), confirmed)
    }

    @Test
    fun aValueOutsideTheBoundsCannotBeConfirmed() {
        show(NumberKeypadSpec("n", "Validité", maxDigits = 2, min = 12, max = 36))

        compose.typeOnNumPad("40")

        compose.onNodeWithTag("n_ok").assertIsNotEnabled()
    }

    @Test
    fun backCancelsWithoutAValue() {
        show(NumberKeypadSpec("n", "Validité"))

        compose.typeOnNumPad("5")
        compose.onNodeWithTag("n_cancel").performClick()

        assertEquals(1, cancelled)
        assertEquals(emptyList<String>(), confirmed)
    }
}
