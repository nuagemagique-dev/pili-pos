package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.v2.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
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

/** The pure rules of the text keypad (`press` of `TextKeypad.vue`). */
class TextKeypadRulesTest {
    @Test
    fun aKeyAppendsWhileTheLimitAllows() {
        assertEquals("ab", keypadAppend("a", "b", 5))
        assertEquals("abcde", keypadAppend("abcd", "e", 5))
    }

    @Test
    fun aKeyThatWouldPassTheLimitChangesNothing() {
        assertEquals("abcde", keypadAppend("abcde", "f", 5))
        assertEquals("abc", keypadAppend("abc", "def", 5))
    }
}

/** The on-screen text keypad (`TextKeypad.vue`): the keys, the limit, the chips, the shift key and the actions. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TextKeypadTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val confirmed = mutableListOf<String>()
    private var cancelled = 0

    private fun show(spec: TextKeypadSpec) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box { TextKeypad(spec, onConfirm = { confirmed += it }, onCancel = { cancelled++ }) }
                }
            }
        }
        compose.waitForIdle()
    }

    @Suppress("LongParameterList")
    private fun spec(
        initial: String = "",
        maxLen: Int = 10,
        symbols: List<String> = emptyList(),
        space: Boolean = false,
        caseToggle: Boolean = false,
        valid: (String) -> Boolean = { true },
    ) = TextKeypadSpec(
        tag = "t",
        title = "Titre",
        placeholder = "Saisir",
        initial = initial,
        maxLen = maxLen,
        symbols = symbols,
        space = space,
        caseToggle = caseToggle,
        valid = valid,
        invalidText = "Refusé",
    )

    @Test
    fun itOpensEmptyWithThePlaceholderAndCanBeValidatedEmpty() {
        show(spec())

        compose.onNodeWithTag("t_dialog").assertIsDisplayed()
        compose.onNodeWithTag("t_placeholder").assertTextEquals("Saisir")
        compose.onNodeWithTag("t_confirm").assertIsEnabled().performClick()
        assertEquals(listOf(""), confirmed)
    }

    @Test
    fun theKeysTypeLettersDigitsAndSymbols() {
        show(spec(symbols = listOf("-", "é"), maxLen = 20))

        compose.typeOnKeypad("ab-é9", "t")

        compose.onNodeWithTag("t_text").assertTextEquals("ab-é9")
    }

    @Test
    fun theSpaceBarOnlyExistsWhenAsked() {
        show(spec())
        compose.onNodeWithTag("t_space").assertDoesNotExist()
    }

    @Test
    fun theSpaceBarTypesASpace() {
        show(spec(space = true))

        compose.typeOnKeypad("a b", "t")

        compose.onNodeWithTag("t_text").assertTextEquals("a b")
    }

    @Test
    fun theTextStopsAtTheLimit() {
        show(spec(maxLen = 3))

        compose.typeOnKeypad("abcde", "t")

        compose.onNodeWithTag("t_text").assertTextEquals("abc")
    }

    @Test
    fun theInitialTextIsKeptAndCutToTheLimit() {
        show(spec(initial = "abcdefghijklmno", maxLen = 10))

        compose.onNodeWithTag("t_text").assertTextEquals("abcdefghij")
    }

    @Test
    fun backspaceErasesTheLastCharacterAndClearErasesAll() {
        show(spec())

        compose.typeOnKeypad("abc", "t")
        compose.onNodeWithTag("t_backspace").performClick()
        compose.onNodeWithTag("t_text").assertTextEquals("ab")
        compose.onNodeWithTag("t_clear").performClick()
        compose.onNodeWithTag("t_placeholder").assertIsDisplayed()
    }

    @Test
    fun theShiftKeyGivesCapitalsAndComesBack() {
        show(spec(caseToggle = true))

        compose.typeOnKeypad("aBc", "t")

        compose.onNodeWithTag("t_text").assertTextEquals("aBc")
    }

    @Test
    fun theShiftKeyAlsoCapitalisesTheAccents() {
        show(spec(symbols = listOf("é", "-"), caseToggle = true))

        compose.typeOnKeypad("É-", "t")

        compose.onNodeWithTag("t_text").assertTextEquals("É-")
    }

    @Test
    fun noShiftKeyUnlessAsked() {
        show(spec())
        compose.onNodeWithTag("t_case").assertDoesNotExist()
    }

    @Test
    fun anInvalidTextShowsTheErrorAndCannotBeConfirmed() {
        show(spec(valid = { it != "no" }))

        compose.typeOnKeypad("no", "t")

        compose.onNodeWithTag("t_error").assertTextEquals("Refusé")
        compose.onNodeWithTag("t_confirm").assertIsNotEnabled()
        compose.onNodeWithTag("t_backspace").performClick()
        compose.onNodeWithTag("t_error").assertDoesNotExist()
        compose.onNodeWithTag("t_confirm").assertIsEnabled()
    }

    @Test
    fun validateGivesTheTextAndCancelLeavesWithoutIt() {
        show(spec())

        compose.typeOnKeypad("ok", "t")
        compose.onNodeWithTag("t_cancel").performClick()
        assertEquals(1, cancelled)
        assertEquals(emptyList<String>(), confirmed)

        compose.onNodeWithTag("t_confirm").performClick()
        assertEquals(listOf("ok"), confirmed)
    }

    @Test
    fun theKeysAreTouchTargetsOfAtLeast48Dp() {
        show(spec(space = true, caseToggle = true))

        listOf("t_key_a", "t_key_1", "t_space", "t_case", "t_backspace", "t_confirm", "t_cancel").forEach {
            compose.onNodeWithTag(it).assertHeightIsAtLeast(48.dp)
        }
    }

    @Test
    fun theCounterAndTheHintShowWhenAsked() {
        show(spec().copy(hint = "Une aide", showCounter = true))

        compose.onNodeWithTag("t_hint").assertTextEquals("Une aide")
        compose.onNodeWithTag("t_counter").assertTextEquals("0/10")
        compose.typeOnKeypad("ab", "t")
        compose.onNodeWithTag("t_counter").assertTextEquals("2/10")
    }
}
