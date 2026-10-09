package dev.nuagemagique.pilipos.ui.components

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
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class EmailKeypadTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val confirmed = mutableListOf<String>()
    private var cancelled = 0

    private fun show(language: AppLanguage = AppLanguage.FR) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(language, catalog) {
                    EmailKeypad(
                        texts = defaultEmailKeypadTexts(),
                        onConfirm = { confirmed += it },
                        onCancel = { cancelled++ },
                    )
                }
            }
        }
    }

    private fun type(text: String) {
        text.forEach { compose.onNodeWithTag("email_key_$it").performClick() }
    }

    @Test
    fun startsEmptyWithThePlaceholderAndValidateDisabled() {
        show()
        compose.onNodeWithTag("email_keypad").assertIsDisplayed()
        compose.onNodeWithTag("email_placeholder").assertTextEquals("Saisir l'adresse…")
        compose.onNodeWithTag("email_validate").assertIsNotEnabled()
        compose.onNodeWithTag("email_error").assertDoesNotExist()
    }

    @Test
    fun theTextsFollowTheLanguage() {
        show(AppLanguage.EN)
        compose.onNodeWithTag("email_placeholder").assertIsDisplayed()
        compose.onNodeWithTag("email_validate").assertTextEquals("Confirm")
    }

    @Test
    fun theTypedKeysBuildTheAddress() {
        show()
        type("jean")
        compose.onNodeWithTag("email_key_@").performClick()
        type("mail")
        compose.onNodeWithTag("email_key_.").performClick()
        compose.onNodeWithTag("email_quick_.be").performClick()
        compose.onNodeWithTag("email_buffer").assertTextEquals("jean@mail..be")
    }

    @Test
    fun digitsAndSymbolsAreTyped() {
        show()
        type("a1_-+9")
        compose.onNodeWithTag("email_buffer").assertTextEquals("a1_-+9")
    }

    @Test
    fun aWrongAddressShowsTheErrorAndKeepsValidateDisabled() {
        show()
        type("jean")
        compose.onNodeWithTag("email_error").assertTextEquals("Adresse invalide")
        compose.onNodeWithTag("email_validate").assertIsNotEnabled()
    }

    @Test
    fun aValidAddressEnablesValidateAndHidesTheError() {
        show()
        type("a@b")
        compose.onNodeWithTag("email_quick_.com").performClick()
        compose.onNodeWithTag("email_buffer").assertTextEquals("a@b.com")
        compose.onNodeWithTag("email_error").assertDoesNotExist()
        compose.onNodeWithTag("email_validate").assertIsEnabled()
    }

    @Test
    fun backspaceAndClearEditTheAddress() {
        show()
        type("abc")
        compose.onNodeWithTag("email_backspace").performClick()
        compose.onNodeWithTag("email_buffer").assertTextEquals("ab")
        compose.onNodeWithTag("email_clear").performClick()
        compose.onNodeWithTag("email_placeholder").assertIsDisplayed()
    }

    @Test
    fun validateThenConfirmGivesTheAddress() {
        show()
        type("a@b.be")
        compose.onNodeWithTag("email_validate").performClick()
        compose.onNodeWithTag("email_confirm_display").assertTextEquals("a@b.be")
        assertEquals(emptyList<String>(), confirmed)
        compose.onNodeWithTag("email_confirm").performClick()
        assertEquals(listOf("a@b.be"), confirmed)
        assertEquals(0, cancelled)
    }

    @Test
    fun editGoesBackToTypingWithTheAddressKept() {
        show()
        type("a@b.be")
        compose.onNodeWithTag("email_validate").performClick()
        compose.onNodeWithTag("email_edit").performClick()
        compose.onNodeWithTag("email_buffer").assertTextEquals("a@b.be")
        compose.onNodeWithTag("email_confirm").assertDoesNotExist()
        assertEquals(emptyList<String>(), confirmed)
    }

    @Test
    fun cancelCallsBack() {
        show()
        type("abc")
        compose.onNodeWithTag("email_cancel").performClick()
        assertEquals(1, cancelled)
        assertEquals(emptyList<String>(), confirmed)
    }

    @Test
    fun aTapOutsideTheCardCancels() {
        show()
        compose.onNodeWithTag("pili_dialog_scrim").performTouchInput { click(Offset(4f, 4f)) }
        assertEquals(1, cancelled)
    }

    @Test
    fun theCardIsNotDismissedByATapInside() {
        show()
        compose.onNodeWithTag("pili_dialog_card").performTouchInput { click(Offset(4f, 4f)) }
        assertEquals(0, cancelled)
    }
}
