package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
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

/** A field that opens a keypad of the host instead of the system keyboard (C13 / R6). */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class KeypadFieldTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val done = mutableListOf<String>()

    private fun show(
        start: String = "",
        kind: KeypadKind = KeypadKind.Text(),
        enabled: Boolean = true,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    var value by remember { mutableStateOf(start) }
                    Column {
                        KeypadField(
                            value = value,
                            onValueChange = { value = it },
                            title = "Mon champ",
                            placeholder = "Rien",
                            modifier = Modifier.testTag("f"),
                            kind = kind,
                            enabled = enabled,
                            onDone = { done += value },
                        )
                    }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun anEmptyFieldShowsItsPlaceholderAndAFilledOneItsValue() {
        show()
        compose.onNodeWithTag("f").assertTextEquals("Rien")
    }

    @Test
    fun theValueIsShown() {
        show(start = "Salle 1")
        compose.onNodeWithTag("f").assertTextEquals("Salle 1")
    }

    @Test
    fun aTapOpensTheKeypadHeadedByTheTitleAndNoSystemKeyboardIsNeeded() {
        show()

        compose.onNodeWithTag("f").performClick()

        compose.onNodeWithTag("keypad_dialog").assertIsDisplayed()
    }

    @Test
    fun theValidatedTextReplacesTheValue() {
        show(start = "ab")

        compose.fillWithKeypad("f", "cd")

        compose.onNodeWithTag("f").assertTextEquals("cd")
        compose.onNodeWithTag("keypad_dialog").assertDoesNotExist()
        assertEquals(listOf("cd"), done)
    }

    @Test
    fun cancelKeepsTheValue() {
        show(start = "ab")

        compose.onNodeWithTag("f").performClick()
        compose.typeOnKeypad("zz")
        compose.onNodeWithTag("keypad_cancel").performClick()

        compose.onNodeWithTag("f").assertTextEquals("ab")
        assertEquals(emptyList<String>(), done)
    }

    @Test
    fun aNumberFieldOpensTheNumberPad() {
        show(kind = KeypadKind.Number(maxDigits = 2))

        compose.fillWithNumPad("f", "18")

        compose.onNodeWithTag("f").assertTextEquals("18")
    }

    @Test
    fun aKeyboardFieldOpensTheAzertyKeyboard() {
        show(kind = KeypadKind.Keyboard(maxLength = 10))

        compose.fillWithKeyboard("f", "RUE 1")

        compose.onNodeWithTag("f").assertTextEquals("RUE 1")
    }

    @Test
    fun aDisabledFieldDoesNotOpen() {
        show(enabled = false)

        compose.onNodeWithTag("f").performClick()

        compose.onNodeWithTag("keypad_dialog").assertDoesNotExist()
    }
}
