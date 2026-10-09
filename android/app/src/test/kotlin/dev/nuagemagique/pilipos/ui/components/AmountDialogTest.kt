package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.click
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import dev.nuagemagique.pilipos.i18n.AppLanguage
import dev.nuagemagique.pilipos.i18n.I18nTestSupport
import dev.nuagemagique.pilipos.ui.text.ProvideUiText
import dev.nuagemagique.pilipos.ui.theme.PiliTheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w800dp-h640dp-mdpi")
class AmountDialogTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val confirmed = mutableListOf<Long>()
    private var cancelled = 0

    private fun show(
        allowZero: Boolean = false,
        itemName: String? = "Sushi libre",
        language: AppLanguage = AppLanguage.FR,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(language, catalog) {
                    AmountDialog(
                        title = "Prix libre",
                        itemName = itemName,
                        texts = defaultAmountDialogTexts(),
                        onConfirm = { confirmed += it },
                        onCancel = { cancelled++ },
                        allowZero = allowZero,
                    )
                }
            }
        }
    }

    private fun press(vararg digits: Char) {
        digits.forEach { compose.onNodeWithTag("numpad_key_$it").performClick() }
    }

    @Test
    fun startsAtZeroWithConfirmDisabled() {
        show()
        compose.onNodeWithTag("amount_dialog").assertIsDisplayed()
        compose.onNodeWithTag("amount_value").assertTextEquals("0,00 €")
        compose.onNodeWithTag("amount_confirm").assertIsNotEnabled()
    }

    @Test
    fun theTypedDigitsAreShownAsEuros() {
        show()
        press('2', '0', '0', '0')
        compose.onNodeWithTag("amount_value").assertTextEquals("20,00 €")
        press('5')
        compose.onNodeWithTag("amount_value").assertTextEquals("200,05 €")
    }

    @Test
    fun confirmGivesTheCents() {
        show()
        press('1', '2', '5', '0')
        compose.onNodeWithTag("amount_confirm").assertIsEnabled().performClick()
        assertEquals(listOf(1250L), confirmed)
    }

    @Test
    fun zeroIsRefusedUnlessAllowed() {
        show(allowZero = false)
        press('0')
        compose.onNodeWithTag("amount_value").assertTextEquals("0,00 €")
        compose.onNodeWithTag("amount_confirm").assertIsNotEnabled()
    }

    @Test
    fun anEmptyDrawerIsConfirmedAsZeroWhenAllowed() {
        show(allowZero = true)
        compose.onNodeWithTag("amount_confirm").assertIsEnabled().performClick()
        assertEquals(listOf(0L), confirmed)
    }

    @Test
    fun theSeventhDigitIsIgnored() {
        show()
        press('1', '2', '3', '4', '5', '6', '7')
        compose.onNodeWithTag("amount_value").assertTextEquals("1\u202F234,56 €")
    }

    @Test
    fun backspaceAndClearEditTheAmount() {
        show()
        press('1', '2', '3')
        compose.onNodeWithTag("numpad_back").performClick()
        compose.onNodeWithTag("amount_value").assertTextEquals("0,12 €")
        compose.onNodeWithTag("numpad_clear").performClick()
        compose.onNodeWithTag("amount_value").assertTextEquals("0,00 €")
        compose.onNodeWithTag("amount_confirm").assertIsNotEnabled()
    }

    @Test
    fun cancelAndTheScrimCallOnCancel() {
        show()
        compose.onNodeWithTag("amount_cancel").performClick()
        assertEquals(1, cancelled)
        compose.onNodeWithTag("pili_dialog_scrim").performTouchInput { click(Offset(4f, 4f)) }
        assertEquals(2, cancelled)
        assertTrue(confirmed.isEmpty())
    }

    @Test
    fun itemNameIsOptional() {
        show(itemName = null)
        compose.onNodeWithTag("amount_dialog").assertIsDisplayed()
    }

    @Test
    fun scrimStopsTapsFromReachingTheScreenBehind() {
        var behind = 0
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box(Modifier.fillMaxSize()) {
                        Box(Modifier.fillMaxSize().clickable { behind++ })
                        AmountDialog("T", null, defaultAmountDialogTexts(), onConfirm = {}, onCancel = {})
                    }
                }
            }
        }
        compose.onNodeWithTag("pili_dialog_scrim").performTouchInput { click(Offset(4f, 4f)) }
        assertEquals(0, behind)
    }
}
