package dev.nuagemagique.pilipos.ui.components

import androidx.compose.foundation.layout.Box
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
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

/** The AZERTY keyboard dialog (`TextKeyboard.vue`): capitals by default, accents, the case key, the limit. */
@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w1920dp-h1080dp-mdpi")
class TextKeyboardTest {
    @get:Rule
    val compose = createComposeRule()

    private val catalog = I18nTestSupport.catalog()
    private val confirmed = mutableListOf<String>()
    private var cancelled = 0

    private fun show(
        initial: String = "",
        maxLength: Int = 12,
        caseToggle: Boolean = false,
    ) {
        compose.setContent {
            PiliTheme {
                ProvideUiText(AppLanguage.FR, catalog) {
                    Box {
                        TextKeyboardDialog(
                            TextKeyboardSpec("k", "Nom", "Saisir", initial, maxLength, caseToggle),
                            onConfirm = { confirmed += it },
                            onCancel = { cancelled++ },
                        )
                    }
                }
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun itOpensEmptyWithThePlaceholder() {
        show()

        compose.onNodeWithTag("k_dialog").assertIsDisplayed()
        compose.onNodeWithTag("k_placeholder").assertTextEquals("Saisir")
    }

    @Test
    fun theLettersAreCapitalsByDefault() {
        show()

        compose.typeOnKeyboard("rue 9", "k")

        compose.onNodeWithTag("k_text").assertTextEquals("RUE 9")
    }

    @Test
    fun theAccentsAndThePunctuationAreOnTheKeyboard() {
        show()

        listOf("É", "Ç", "-", "'", "&").forEach { compose.onNodeWithTag("k_key_$it").performClick() }

        compose.onNodeWithTag("k_text").assertTextEquals("ÉÇ-'&")
    }

    @Test
    fun theCaseKeyGivesLowerCaseWhenAsked() {
        show(caseToggle = true)

        compose.typeOnKeyboard("Thé", "k")

        compose.onNodeWithTag("k_text").assertTextEquals("Thé")
    }

    @Test
    fun noCaseKeyUnlessAsked() {
        show()
        compose.onNodeWithTag("k_case").assertDoesNotExist()
    }

    @Test
    fun theTextStopsAtTheLimit() {
        show(maxLength = 4)

        compose.typeOnKeyboard("ABCDEF", "k")

        compose.onNodeWithTag("k_text").assertTextEquals("ABCD")
    }

    @Test
    fun backspaceAndClearErase() {
        show(initial = "ABC")

        compose.onNodeWithTag("k_backspace").performClick()
        compose.onNodeWithTag("k_text").assertTextEquals("AB")
        compose.onNodeWithTag("k_clear").performClick()
        compose.onNodeWithTag("k_placeholder").assertIsDisplayed()
    }

    @Test
    fun validateGivesTheTextAndCancelLeaves() {
        show()

        compose.typeOnKeyboard("ACME", "k")
        compose.onNodeWithTag("k_cancel").performClick()
        compose.onNodeWithTag("k_confirm").performClick()

        assertEquals(1, cancelled)
        assertEquals(listOf("ACME"), confirmed)
    }

    @Test
    fun theKeysAreTouchTargetsOfAtLeast48Dp() {
        show(caseToggle = true)

        listOf("k_key_A", "k_key_1", "k_space", "k_case", "k_clear", "k_backspace", "k_confirm").forEach {
            compose.onNodeWithTag(it).assertHeightIsAtLeast(48.dp)
        }
    }
}
